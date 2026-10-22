import { json, request, sleep } from './providers/common.js';
const MS = 'https://login.microsoftonline.com/common/oauth2/v2.0';
const BAIDU = 'https://openapi.baidu.com/oauth/2.0';
const scopes = 'https://graph.microsoft.com/Files.ReadWrite offline_access';
const refreshing = new Map();
const generations = new Map();
const key = provider => `auth:${provider}`;
const read = async provider => (await chrome.storage.session.get(key(provider)))[key(provider)];
const store = async (provider, value) => chrome.storage.session.set({ [key(provider)]: value });
export const redirectURI = () => chrome.identity.getRedirectURL('oauth2');
export async function status(provider) { const value = await read(provider); return value?.generation; }

export function validateCallback(value, expected, state) {
  const url = new URL(value), target = new URL(expected);
  if (url.origin !== target.origin || url.pathname !== target.pathname || url.searchParams.get('state') !== state)
    throw Error('Authorization callback verification failed');
  if (url.searchParams.has('error')) throw Error('Authorization was denied or cancelled');
  const code = url.searchParams.get('code');
  if (!code) throw Error('Authorization code is missing');
  return code;
}
export async function pkce() {
  const encode = bytes => btoa(String.fromCharCode(...bytes)).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
  const verifier = encode(crypto.getRandomValues(new Uint8Array(32)));
  return { verifier, challenge: encode(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)))) };
}
function tokenData(data, previous) {
  if (!data.access_token || !Number.isFinite(Number(data.expires_in)) || Number(data.expires_in) <= 0)
    throw Error('Authorization failed. Check the app configuration and sign in again.');
  return { ...previous, access: data.access_token, refresh: data.refresh_token || previous.refresh,
    expires: Date.now() + Number(data.expires_in) * 1000 };
}
async function msToken(values) {
  return json(await request(`${MS}/token`, { method: 'POST', body: new URLSearchParams(values) }));
}
async function bdToken(values) {
  // Device flow is documented for personal installed clients. Never ship a shared secret.
  const response = await request(`${BAIDU}/token?${new URLSearchParams(values)}`);
  if (response.status === 400) { try { return await response.json(); } catch { throw Error('Invalid authorization response'); } }
  return json(response);
}
export async function connect(provider, config, showCode, signal) {
  const generation = crypto.randomUUID(); generations.set(provider, generation);
  let record = { generation, clientId: config.clientId };
  if (provider === 'google') {
    if (chrome.runtime.getManifest().oauth2.client_id.includes('configure')) throw Error('Google app configuration is missing. Open the first-time setup guide.');
    const result = await chrome.identity.getAuthToken({ interactive: true });
    if (!result.token) throw Error('Google authorization failed');
    record.access = result.token;
  } else if (provider === 'onedrive') {
    if (!/^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(config.clientId || '')) throw Error('Enter a valid Microsoft Application (client) ID');
    const { verifier, challenge } = await pkce(); const state = crypto.randomUUID();
    const redirect = redirectURI();
    const params = new URLSearchParams({ client_id: config.clientId, response_type: 'code', redirect_uri: redirect,
      scope: scopes, state, code_challenge: challenge, code_challenge_method: 'S256', prompt: 'select_account' });
    const response = await chrome.identity.launchWebAuthFlow({ url: `${MS}/authorize?${params}`, interactive: true });
    const code = validateCallback(response, redirect, state);
    record = tokenData(await msToken({ client_id: config.clientId, grant_type: 'authorization_code',
      code, redirect_uri: redirect, code_verifier: verifier, scope: scopes }), record);
  } else if (provider === 'baidu') {
    if (!config.clientId || !config.secret) throw Error('Enter your own Baidu App Key and Secret Key');
    const device = await json(await request(`${BAIDU}/device/code?${new URLSearchParams({ response_type: 'device_code', client_id: config.clientId, scope: 'basic,netdisk' })}`));
    if (!device.device_code || !device.user_code || !device.expires_in) throw Error('Baidu device authorization failed');
    const verify = new URL(device.verification_url || 'https://openapi.baidu.com/device');
    if (verify.protocol !== 'https:' || !['openapi.baidu.com', 'passport.baidu.com'].includes(verify.hostname)) throw Error('Untrusted authorization page');
    showCode({ code: device.user_code, url: verify.href });
    await chrome.tabs.create({ url: verify.href });
    const end = Date.now() + Math.min(Number(device.expires_in), 1800) * 1000;
    let interval = Math.max(Number(device.interval) || 5, 5);
    let token;
    while (Date.now() < end) {
      await sleep(interval * 1000);
      if (signal.aborted || generations.get(provider) !== generation) throw Error('Authorization cancelled');
      const data = await bdToken({ grant_type: 'device_token', code: device.device_code, client_id: config.clientId, client_secret: config.secret });
      if (data.access_token) { token = data; break; }
      if (data.error === 'slow_down') interval += 5;
      else if (data.error !== 'authorization_pending') throw Error('Baidu authorization expired or was denied');
    }
    if (!token) throw Error('Authorization timed out. Please try again.');
    record = tokenData(token, { ...record, secret: config.secret });
  } else throw Error('Unsupported drive');
  if (signal.aborted || generations.get(provider) !== generation) throw Error('Authorization cancelled');
  await store(provider, record);
  return generation;
}
export async function disconnect(provider) {
  generations.set(provider, crypto.randomUUID());
  const old = await read(provider);
  await chrome.storage.session.remove(key(provider));
  if (provider === 'google' && old?.access) await chrome.identity.removeCachedAuthToken({ token: old.access });
}
export async function access(provider, binding) {
  const old = await read(provider);
  if (!old || old.generation !== binding) throw Error('Drive binding changed. Select the files again to use the new account.');
  if (provider === 'google') {
    const result = await chrome.identity.getAuthToken({ interactive: false });
    if (!result.token) throw Error('Sign in to Google again');
    return result.token;
  }
  if (old.expires > Date.now() + 120000) return old.access;
  if (!refreshing.has(provider)) {
    const promise = (async () => {
      const data = provider === 'onedrive' ? await msToken({ client_id: old.clientId, grant_type: 'refresh_token', refresh_token: old.refresh, scope: scopes }) :
        await bdToken({ client_id: old.clientId, client_secret: old.secret, grant_type: 'refresh_token', refresh_token: old.refresh });
      const value = tokenData(data, old);
      if ((await read(provider))?.generation !== binding) throw Error('Drive binding changed');
      await store(provider, value); return value.access;
    })().finally(() => refreshing.delete(provider));
    refreshing.set(provider, promise);
  }
  return refreshing.get(provider);
}
