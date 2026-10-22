import './style.css';
import { language, setLanguage, t } from './i18n.js';
import { connect, disconnect, status, redirectURI, access } from './auth.js';
import { fileEntries, folderEntries, droppedEntries, parts } from './files.js';
import { UploadQueue } from './queue.js';
import * as google from './providers/google.js';
import * as onedrive from './providers/onedrive.js';
import * as baidu from './providers/baidu.js';

const providers = { google, onedrive, baidu };
const names = { google: 'Google Drive', onedrive: 'OneDrive', baidu: '百度网盘' };
const $ = selector => document.querySelector(selector);
const bindings = {}, authControllers = {}, views = {};
let settings, history = [], saving = Promise.resolve(), toastTimer, queue;
function node(tag, text, className) { const value = document.createElement(tag); if (text) value.textContent = text; if (className) value.className = className; return value; }
function button(label, handler, secondary = false) { const value = node('button', label, secondary ? 'secondary' : ''); value.type = 'button'; value.onclick = () => Promise.resolve().then(handler).catch(notify); return value; }
function notify(error) { const box = $('#toast'); box.textContent = t(error?.message || String(error)); box.hidden = false; clearTimeout(toastTimer); toastTimer = setTimeout(() => box.hidden = true, 6500); }
function pending(provider) { return queue?.jobs.some(job => job.provider === provider && job.status !== 'completed'); }
function canChange(provider) { if (pending(provider)) throw Error(t('keepAccount')); }
function field(parent, label, type = 'text', value = '') { const wrap = node('label', t(label)); const input = node('input'); input.type = type; input.value = value; input.autocomplete = 'off'; wrap.append(input); parent.append(wrap); return input; }
async function copy(value) { await navigator.clipboard.writeText(value); notify(t('copied')); }
function resultText(value) { return [value.path, value.url, t(value.notice)].filter(Boolean).join('\n'); }
function validateResult(result) {
  if (result.url) { const url = new URL(result.url); if (url.protocol !== 'https:') throw Error('Invalid cloud result URL'); }
  return result;
}
async function saveSettings() { await chrome.storage.local.set({ settings }); }
function driveCard(provider) {
  const card = node('article', '', 'drive'); card.dataset.provider = provider;
  card.append(node('h2', names[provider]));
  const state = node('div', t(bindings[provider] ? 'connected' : 'disconnected'), `status ${bindings[provider] ? 'connected' : ''}`); card.append(state);
  const selectedLabel = node('label', t('select')); const selected = node('input'); selected.type = 'checkbox'; selected.checked = settings[provider].enabled;
  selected.onchange = () => { settings[provider].enabled = selected.checked; saveSettings().catch(notify); }; selectedLabel.prepend(selected); card.append(selectedLabel);
  const buttons = node('div', '', 'buttons');
  const start = button(t('connect'), async () => {
    canChange(provider);
    const controller = new AbortController(); authControllers[provider] = controller;
    start.disabled = true; details.open = true; state.textContent = t('binding');
    stop.hidden = false; challenge.replaceChildren();
    try {
      const config = { clientId: client.value.trim(), secret: secret?.value.trim() };
      // Public app IDs can persist; the Baidu secret is never written to local/sync storage.
      settings[provider].clientId = config.clientId; await saveSettings();
      bindings[provider] = await connect(provider, config, data => {
        challenge.append(node('p', t('code')), node('strong', data.code));
        challenge.append(button(t('copy'), () => copy(data.code)));
        const link = node('a', language === 'en' ? 'Open authorization page' : '打开官方授权页'); link.href = data.url; link.target = '_blank'; link.rel = 'noreferrer'; challenge.append(link);
      }, controller.signal);
      challenge.replaceChildren(); if (secret) secret.value = '';
      state.textContent = t('connected'); state.className = 'status connected'; details.open = false;
    } catch (error) { state.textContent = t(bindings[provider] ? 'connected' : 'disconnected'); throw error; }
    finally { delete authControllers[provider]; start.disabled = false; stop.hidden = true; }
  });
  const stop = button(t('stop'), () => { authControllers[provider]?.abort(); }); stop.hidden = true;
  buttons.append(start, button(t('disconnect'), async () => { canChange(provider); authControllers[provider]?.abort(); await disconnect(provider); delete bindings[provider]; state.textContent = t('disconnected'); state.className = 'status'; }, true), stop);
  card.append(buttons);
  const challenge = node('div', '', 'challenge'); card.append(challenge);
  const details = node('details'); details.append(node('summary', t('configure'))); const guide = node('div', '', 'guide'); guide.append(node('p', t(provider === 'google' ? 'googleGuide' : provider === 'onedrive' ? 'msGuide' : 'bdGuide')));
  const link = node('a', language === 'en' ? 'Open official app console ↗' : '打开官方应用后台 ↗');
  link.href = { google: 'https://console.cloud.google.com/apis/credentials', onedrive: 'https://entra.microsoft.com/', baidu: 'https://pan.baidu.com/union/console/applist' }[provider]; link.target = '_blank'; link.rel = 'noreferrer'; guide.append(link);
  if (provider !== 'baidu') guide.append(node('code', provider === 'google' ? chrome.runtime.id : redirectURI()));
  const client = field(guide, 'client', 'text', provider === 'google' ? chrome.runtime.getManifest().oauth2.client_id : settings[provider].clientId || '');
  const secret = provider === 'baidu' ? field(guide, 'secret', 'password') : null;
  const destination = field(guide, 'destination', 'text', settings[provider].directory);
  guide.append(button(t('save'), async () => {
    canChange(provider); const path = parts(destination.value).join('/');
    if (!path || provider === 'baidu' && (!path.startsWith('apps/') || parts(path).length < 2)) throw Error('Enter a valid upload folder');
    settings[provider].directory = `/${path}/`; settings[provider].clientId = client.value.trim();
    await saveSettings(); notify(t('saved'));
  }));
  if (provider === 'google') {
    guide.append(button(t('download'), () => {
      if (!/^[\w-]+\.apps\.googleusercontent\.com$/.test(client.value.trim()) || client.value.includes('configure')) throw Error('Enter the Chrome Extension OAuth client ID from Google Cloud');
      const manifest = structuredClone(chrome.runtime.getManifest());
      // getManifest includes browser-injected metadata which must not be exported.
      delete manifest.update_url;
      manifest.oauth2.client_id = client.value.trim();
      const url = URL.createObjectURL(new Blob([JSON.stringify(manifest, null, 2)], { type: 'application/json' }));
      const a = node('a'); a.href = url; a.download = 'manifest.json'; a.click(); setTimeout(() => URL.revokeObjectURL(url), 10000);
    }, true));
    guide.append(node('p', t('googleNative')));
  }
  details.append(guide); card.append(details); views[provider] = { card, start, selected }; return card;
}
function renderHistory() {
  $('#history').replaceChildren(...history.map(item => { const row = node('div', '', 'history-row'); row.append(node('span', `${names[item.provider]} · ${item.path}`), button(t('copy'), () => copy(resultText(item)), true)); return row; }));
}
function renderQueue() {
  const waiting = queue.jobs.filter(job => job.status === 'waiting').length;
  $('#queue-title').textContent = t(queue.paused ? 'paused' : queue.running || waiting ? 'active' : 'ready');
  $('#counts').textContent = language === 'en' ? `${queue.running} processing · ${waiting} waiting` : `${queue.running} 项处理中 · ${waiting} 项排队`;
  $('#pause').textContent = t(queue.paused ? 'resume' : 'pause');
  $('#jobs').replaceChildren(...queue.jobs.slice().reverse().map(job => {
    const row = node('article', '', 'job'); const info = node('div'); info.append(node('strong', job.path), node('small', names[job.provider]));
    if (job.error) info.append(node('p', job.error));
    if (job.result) info.append(node('p', t(job.result.notice)));
    const status = node('div'); status.append(node('small', t(job.status))); const bar = node('progress'); bar.max = 100; bar.value = job.progress; status.append(bar);
    row.append(info, status);
    if (job.status === 'failed') row.append(button(t('retry'), () => queue.retry(job)));
    if (job.result) row.append(button(t('copy'), () => copy(resultText(job.result)), true));
    return row;
  }));
}
async function add(entries) {
  const selected = Object.keys(providers).filter(provider => settings[provider].enabled);
  if (!selected.length || selected.some(provider => !bindings[provider])) throw Error(t('bindingRequired'));
  if (!entries.length) return;
  // File handles remain in this tab. Do not duplicate entire large files into extension storage.
  queue.add(entries, selected, structuredClone(settings), { ...bindings });
}
function renderLanguage() {
  document.querySelectorAll('[data-i18n]').forEach(item => item.textContent = t(item.dataset.i18n));
  $('#drives').replaceChildren(...Object.keys(providers).map(driveCard));
  renderQueue(); renderHistory();
}
async function initialize() {
  await chrome.storage.session.setAccessLevel({ accessLevel: 'TRUSTED_CONTEXTS' });
  await chrome.storage.local.setAccessLevel({ accessLevel: 'TRUSTED_CONTEXTS' });
  const saved = await chrome.storage.local.get(['settings', 'language', 'history']);
  settings = { google: { enabled: true, directory: '/Drive Delivery/' }, onedrive: { enabled: false, directory: '/Drive Delivery/' }, baidu: { enabled: false, directory: '/apps/双盘发货助手/发货/' }, publicShare: false, ...saved.settings };
  history = Array.isArray(saved.history) ? saved.history.slice(0, 200) : [];
  setLanguage(saved.language); $('#language').value = language;
  for (const provider of Object.keys(providers)) bindings[provider] = await status(provider);
  queue = new UploadQueue(async (job, progress) => {
    await access(job.provider, job.binding);
    const result = validateResult(await providers[job.provider].upload(job, progress));
    history = [{ ...result, provider: job.provider, completedAt: Date.now() }, ...history].slice(0, 200);
    const snapshot = structuredClone(history);
    saving = saving.catch(() => {}).then(() => chrome.storage.local.set({ history: snapshot }));
    // Storage failure must not turn an already committed cloud upload into a failed upload.
    saving.catch(notify); renderHistory(); return result;
  }, () => renderQueue());
  renderLanguage(); $('#public-share').checked = settings.publicShare;
  $('#language').onchange = async event => {
    if (Object.keys(authControllers).length) { event.target.value = language; notify(t('binding')); return; }
    setLanguage(event.target.value); await chrome.storage.local.set({ language }); renderLanguage();
  };
  $('#public-share').onchange = () => { settings.publicShare = $('#public-share').checked; saveSettings().catch(notify); };
  $('#choose').onclick = $('#files-button').onclick = () => $('#files').click();
  $('#files').onchange = async event => { try { await add(fileEntries(event.target.files)); } catch (e) { notify(e); } finally { event.target.value = ''; } };
  $('#folder-button').onclick = async () => {
    try {
      if (window.showDirectoryPicker) await add(await folderEntries(await window.showDirectoryPicker({ mode: 'read' })));
      else { $('#folder').click(); notify(t('folderFallback')); }
    } catch (error) { if (error.name !== 'AbortError') notify(error); }
  };
  $('#folder').onchange = $('#files').onchange;
  const drop = $('#drop-zone'); drop.ondragover = event => { event.preventDefault(); drop.classList.add('over'); };
  drop.ondragleave = () => drop.classList.remove('over');
  drop.ondrop = event => { event.preventDefault(); drop.classList.remove('over'); droppedEntries(event.dataTransfer.items).then(add).catch(notify); };
  $('#pause').onclick = () => queue.pause(!queue.paused);
  $('#clear-history').onclick = async () => { history = []; await saving.catch(() => {}); await chrome.storage.local.set({ history }); renderHistory(); };
  window.addEventListener('beforeunload', event => { if (queue.jobs.some(job => ['waiting', 'running'].includes(job.status))) { event.preventDefault(); event.returnValue = ''; } });
}
function fail(error) { $('#fatal').textContent = error.message || String(error); $('#fatal').hidden = false; document.querySelectorAll('button,input,select').forEach(item => item.disabled = true); }
if (!globalThis.chrome?.storage?.session) fail(Error('请从浏览器扩展管理页加载扩展后打开。 Load this folder as a browser extension first.'));
else navigator.locks.request('drive-delivery-upload-page', { ifAvailable: true }, async lock => {
  if (!lock) return fail(Error(t('otherTab')));
  try { await initialize(); } catch (error) { fail(error); }
  await new Promise(() => {}); // Hold the single queue owner until this tab closes.
}).catch(fail);
