import { access } from '../auth.js';
import { parts } from '../files.js';
import { request, json, serial, sessionURL } from './common.js';
const API = 'https://www.googleapis.com/drive/v3';
const folders = serial();
const escape = value => value.replaceAll('\\', '\\\\').replaceAll("'", "\\'");
async function api(job, path, options = {}) {
  return request(`${API}${path}`, { ...options, headers: { ...options.headers, Authorization: `Bearer ${await access('google', job.binding)}` } }, !options.method);
}
async function folder(job, names) {
  return folders(async () => {
    let parent = 'root';
    for (const name of names) {
      const q = `name='${escape(name)}' and mimeType='application/vnd.google-apps.folder' and '${parent}' in parents and trashed=false`;
      const found = await json(await api(job, `/files?${new URLSearchParams({ q, fields: 'files(id)', pageSize: '1' })}`));
      parent = found.files?.[0]?.id || (await json(await api(job, '/files?fields=id', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, mimeType: 'application/vnd.google-apps.folder', parents: [parent] })
      }))).id;
      if (!parent) throw Error('Google folder ID missing');
    }
    return parent;
  });
}
async function existing(job) {
  const response = await api(job, `/files/${encodeURIComponent(job.state.id)}?fields=id,size,webViewLink`);
  if (response.status === 404) return null;
  const value = await json(response);
  if (Number(value.size) !== job.file.size) throw Error('Existing Google file size does not match');
  return value;
}
export async function upload(job, progress) {
  const path = [...parts(job.destination), ...parts(job.path)];
  const name = job.directory ? null : path.pop();
  if (job.directory) { await folder(job, path); return { path: path.join('/'), notice: 'Folder created' }; }
  let result = job.state.result;
  if (!result) {
    if (job.state.id) result = await existing(job);
    if (!result) {
      const parent = await folder(job, path);
      if (!job.state.id) job.state.id = (await json(await api(job, '/files/generateIds?count=1&space=drive'))).ids?.[0];
      if (!job.state.id) throw Error('Google could not reserve a file ID');
      const metadata = { id: job.state.id, name, parents: [parent] };
      if (!job.file.size) {
        result = await json(await api(job, '/files?fields=id,size,webViewLink', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ...metadata, mimeType: 'application/octet-stream' }) }));
      } else {
        let offset = 0;
        if (job.state.session) {
          const response = await request(sessionURL(job.state.session, 'google'), { method: 'PUT', headers: { Authorization: `Bearer ${await access('google', job.binding)}`, 'Content-Range': `bytes */${job.file.size}` }, body: new Blob([]) });
          if (response.status === 200 || response.status === 201) result = await json(response);
          else if (response.status === 404 || response.status === 410) { result = await existing(job); job.state.session = null; }
          else if (response.status === 308) offset = googleOffset(response, job.file.size);
          else throw Error(`Upload status check failed (HTTP ${response.status})`);
        }
        if (!result && !job.state.session) {
          const response = await request('https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable&fields=id,size,webViewLink', {
            method: 'POST', headers: { Authorization: `Bearer ${await access('google', job.binding)}`, 'Content-Type': 'application/json', 'X-Upload-Content-Length': String(job.file.size), 'X-Upload-Content-Type': job.file.type || 'application/octet-stream' }, body: JSON.stringify(metadata)
          });
          if (!response.ok) throw Error(`Google upload initialization failed (HTTP ${response.status})`);
          job.state.session = sessionURL(response.headers.get('Location'), 'google');
        }
        while (!result && offset < job.file.size) {
          const end = Math.min(offset + 8 * 1024 * 1024, job.file.size);
          const response = await request(sessionURL(job.state.session, 'google'), { method: 'PUT', headers: { Authorization: `Bearer ${await access('google', job.binding)}`, 'Content-Range': `bytes ${offset}-${end - 1}/${job.file.size}` }, body: job.file.slice(offset, end) });
          if (response.status === 200 || response.status === 201) result = await json(response);
          else if (response.status === 308) { const next = googleOffset(response, job.file.size); if (next <= offset) throw Error('Google upload made no progress'); offset = next; progress(offset / job.file.size * 100); }
          else throw Error(`Google upload failed (HTTP ${response.status})`);
        }
      }
      if (!result?.id) throw Error('Google upload completion is unknown; retry to check');
    }
    job.state.result = result;
  }
  if (job.publicShare) {
    const permissions = await json(await api(job, `/files/${encodeURIComponent(result.id)}/permissions?fields=permissions(id,type,role)`));
    if (!permissions.permissions?.some(p => p.type === 'anyone' && p.role === 'reader'))
      await json(await api(job, `/files/${encodeURIComponent(result.id)}/permissions`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ type: 'anyone', role: 'reader' }) }));
  }
  const info = await json(await api(job, `/files/${encodeURIComponent(result.id)}?fields=webViewLink`));
  return { path: [...path, name].join('/'), url: info.webViewLink, notice: job.publicShare ? 'Public link' : 'Private file link' };
}
export function googleOffset(response, size) {
  const range = response.headers.get('Range');
  if (!range) return 0;
  const match = /^bytes=0-(\d+)$/.exec(range);
  const offset = match ? Number(match[1]) + 1 : NaN;
  if (!Number.isSafeInteger(offset) || offset < 0 || offset >= size) throw Error('Invalid Google upload offset');
  return offset;
}
