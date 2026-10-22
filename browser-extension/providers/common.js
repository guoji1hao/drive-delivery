export const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
export async function request(url, options = {}, retry = false) {
  for (let attempt = 0; ; attempt++) {
    let response;
    try { response = await fetch(url, { ...options, redirect: 'error', signal: AbortSignal.timeout(120000) }); }
    catch { throw Error('Network request failed. Retry to check the saved upload session.'); }
    if (retry && attempt < 2 && (response.status === 429 || response.status >= 500)) {
      await sleep(Math.min(30, Math.max(1, Number(response.headers.get('Retry-After')) || 2 ** attempt)) * 1000); continue;
    }
    return response;
  }
}
export async function json(response) {
  if (!response.ok) throw Error(`Cloud request failed (HTTP ${response.status})`);
  try { return await response.json(); } catch { throw Error('Cloud returned an invalid response'); }
}
export function sessionURL(value, provider) {
  let url; try { url = new URL(value); } catch { throw Error('Invalid upload session URL'); }
  const validHost = provider === 'google' ? url.hostname === 'www.googleapis.com' :
    ['.up.1drv.com', '.sharepoint.com', '.onedrive.com'].some(suffix => url.hostname.endsWith(suffix));
  if (url.protocol !== 'https:' || url.port && url.port !== '443' || url.username || url.password || !validHost)
    throw Error('Untrusted upload session URL');
  return url.href;
}
export function serial() {
  let tail = Promise.resolve();
  return async fn => { const next = tail.then(fn); tail = next.catch(() => {}); return next; };
}
export function safeName(value) {
  if (!value || /["*:<>?\/\\|\x00-\x1f]/.test(value) || ['.', '..'].includes(value) || /[. ]$/.test(value))
    throw Error('This filename is not supported by the selected drive');
  return value;
}
export function nextOffset(value, size) {
  const text = value.nextExpectedRanges?.[0];
  if (typeof text !== 'string' || !/^\d+-\d*$/.test(text)) throw Error('Invalid server upload offset');
  const offset = Number(text.split('-')[0]);
  if (!Number.isSafeInteger(offset) || offset < 0 || offset >= size) throw Error('Invalid server upload offset');
  return offset;
}
