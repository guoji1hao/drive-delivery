import { access } from '../auth.js';
import { parts } from '../files.js';
import { json, request, sessionURL, safeName, nextOffset } from './common.js';
const API = 'https://graph.microsoft.com/v1.0/me/drive';
const enc = encodeURIComponent;
async function api(job, path, options = {}) {
  return request(`${API}${path}`, { ...options, headers: { ...options.headers, Authorization: `Bearer ${await access('onedrive', job.binding)}` } }, !options.method);
}
async function folder(job, names) {
  let parent = (await json(await api(job, '/root'))).id;
  for (const name of names) {
    safeName(name);
    const path = `/items/${enc(parent)}:/${enc(name)}`;
    let response = await api(job, path);
    if (response.status === 404) {
      response = await api(job, `/items/${enc(parent)}/children`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name, folder: {}, '@microsoft.graph.conflictBehavior': 'fail' }) });
      if (response.status === 409) response = await api(job, path);
    }
    const value = await json(response);
    if (!value.folder || !value.id) throw Error('OneDrive folder is occupied by a file');
    parent = value.id;
  }
  return parent;
}
async function existing(job) {
  const response = await api(job, job.state.itemPath);
  if (response.status === 404) return null;
  const value = await json(response);
  if (!value.file || value.size !== job.file.size) throw Error('Existing OneDrive file size does not match');
  return value;
}
export async function upload(job, progress) {
  const names = [...parts(job.destination), ...parts(job.path)];
  if (job.directory) { await folder(job, names); return { path: names.join('/'), notice: 'Folder created' }; }
  const original = safeName(names.pop());
  const dot = original.lastIndexOf('.');
  const name = dot > 0 ? `${original.slice(0, dot)}-${job.id}${original.slice(dot)}` : `${original}-${job.id}`;
  const drive = (await json(await api(job, ''))).id;
  if (!drive || job.state.drive && job.state.drive !== drive) throw Error('OneDrive account changed');
  job.state.drive = drive;
  if (!job.state.itemPath) job.state.itemPath = `/items/${enc(await folder(job, names))}:/${enc(name)}`;
  let result = job.state.result || await existing(job);
  if (!result && !job.file.size) {
    result = await json(await api(job, `${job.state.itemPath}:/content?%40microsoft.graph.conflictBehavior=fail`, { method: 'PUT', body: job.file }));
  }
  let offset = 0;
  if (!result && job.state.session) {
    const response = await request(sessionURL(job.state.session, 'onedrive'));
    if ([404, 410].includes(response.status)) { result = await existing(job); job.state.session = null; }
    else offset = nextOffset(await json(response), job.file.size);
  }
  if (!result && !job.state.session) {
    const session = await json(await api(job, `${job.state.itemPath}:/createUploadSession`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ item: { name, '@microsoft.graph.conflictBehavior': 'fail' } }) }));
    job.state.session = sessionURL(session.uploadUrl, 'onedrive');
  }
  while (!result && offset < job.file.size) {
    const end = Math.min(offset + 5 * 1024 * 1024, job.file.size);
    // Preauthenticated upload URLs never receive Graph Authorization headers.
    const response = await request(sessionURL(job.state.session, 'onedrive'), { method: 'PUT', headers: { 'Content-Range': `bytes ${offset}-${end - 1}/${job.file.size}` }, body: job.file.slice(offset, end) });
    const value = await json(response);
    if ([200, 201].includes(response.status)) result = value;
    else { const next = nextOffset(value, job.file.size); if (next <= offset) throw Error('OneDrive upload made no progress'); offset = next; progress(offset / job.file.size * 100); }
  }
  if (!result?.id || result.size !== job.file.size) throw Error('OneDrive upload completion is unknown; retry to check');
  job.state.result = result;
  if (job.publicShare) {
    const share = await json(await api(job, `/items/${enc(result.id)}/createLink`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ type: 'view', scope: 'anonymous' }) }));
    return { url: share.link?.webUrl, path: [...names, name].join('/'), notice: 'Public link' };
  }
  return { url: result.webUrl, path: [...names, name].join('/'), notice: 'Private file link' };
}
