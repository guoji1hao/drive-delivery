import SparkMD5 from 'spark-md5';
import { access } from '../auth.js';
import { parts } from '../files.js';
import { json, request, serial } from './common.js';
const API = 'https://pan.baidu.com/rest/2.0/xpan/file';
const CHUNK = 4 * 1024 * 1024;
const folders = serial();
function checked(value) {
  if ((value.errno ?? value.error_code ?? 0) !== 0) throw Error(`Baidu request failed (code ${Number(value.errno ?? value.error_code)})`);
  return value;
}
async function api(job, method, values, write = false) {
  const query = new URLSearchParams({ method, access_token: await access('baidu', job.binding) });
  const options = write ? { method: 'POST', body: new URLSearchParams(values) } : {};
  if (!write) for (const [key, value] of Object.entries(values)) query.set(key, value);
  return checked(await json(await request(`${API}?${query}`, options, !write)));
}
async function folder(job, names) {
  return folders(async () => {
    let parent = '';
    for (const name of names) {
      const path = `${parent}/${name}`; let found = false;
      for (let start = 0; ; start += 1000) {
        const value = await api(job, 'list', { dir: parent || '/', start: String(start), limit: '1000' });
        if (!Array.isArray(value.list)) throw Error('Baidu folder list missing');
        const item = value.list.find(item => item.path === path);
        if (item) { if (item.isdir !== 1) throw Error('Baidu folder is occupied by a file'); found = true; break; }
        if (value.list.length < 1000) break;
      }
      if (!found) await api(job, 'create', { path, isdir: '1', rtype: '0' }, true);
      parent = path;
    }
  });
}
export async function upload(job, progress) {
  const names = [...parts(job.destination), ...parts(job.path)];
  if (names[0] !== 'apps' || names.length < 3) throw Error('Baidu destination must be inside /apps/your-app/');
  if (job.directory) { await folder(job, names); return { path: `/${names.join('/')}`, notice: 'Folder created' }; }
  if (!job.file.size) throw Error('Baidu API does not support empty files');
  const original = names.pop();
  const dot = original.lastIndexOf('.');
  const name = dot > 0 ? `${original.slice(0, dot)}-${job.id}${original.slice(dot)}` : `${original}-${job.id}`;
  const path = `/${[...names, name].join('/')}`;
  await folder(job, names);
  if (job.state.result) return job.state.result;
  // A stable task suffix makes repeated commit requests target only this job's file.
  let existing;
  for (let start = 0; ; start += 1000) {
    const current = await api(job, 'list', { dir: `/${names.join('/')}`, start: String(start), limit: '1000' });
    if (!Array.isArray(current.list)) throw Error('Baidu folder list missing');
    existing = current.list.find(item => item.path === path);
    if (existing || current.list.length < 1000) break;
  }
  if (existing) {
    if (existing.size !== job.file.size || existing.isdir) throw Error('Existing Baidu file size does not match');
    return { path, notice: 'Uploaded. Baidu public sharing is not supported by this official API.' };
  }
  if (!job.state.hashes) {
    const hashes = [];
    for (let start = 0; start < job.file.size; start += CHUNK)
      hashes.push(SparkMD5.ArrayBuffer.hash(await job.file.slice(start, start + CHUNK).arrayBuffer()));
    job.state.hashes = hashes;
  }
  if (!job.state.uploadId) {
    const value = await api(job, 'precreate', { path, size: String(job.file.size), isdir: '0', autoinit: '1', rtype: '0', block_list: JSON.stringify(job.state.hashes) }, true);
    if (value.return_type === 2) return { path, notice: 'Uploaded. Baidu public sharing is not supported by this official API.' };
    if (!value.uploadid || !Array.isArray(value.block_list) || value.block_list.some(i => !Number.isInteger(i) || i < 0 || i >= job.state.hashes.length)) throw Error('Invalid Baidu upload session');
    job.state.uploadId = value.uploadid; job.state.needed = value.block_list; job.state.done = {};
  }
  for (const index of job.state.needed) {
    if (job.state.done[index]) continue;
    const query = new URLSearchParams({ method: 'upload', type: 'tmpfile', path, uploadid: job.state.uploadId, partseq: String(index), access_token: await access('baidu', job.binding) });
    const form = new FormData(); form.append('file', job.file.slice(index * CHUNK, (index + 1) * CHUNK), 'file');
    const value = checked(await json(await request(`https://d.pcs.baidu.com/rest/2.0/pcs/superfile2?${query}`, { method: 'POST', body: form })));
    if (value.md5 !== job.state.hashes[index]) throw Error('Baidu chunk checksum mismatch');
    job.state.done[index] = value.md5;
    progress(Object.keys(job.state.done).reduce((total, i) => total + Math.min(CHUNK, job.file.size - Number(i) * CHUNK), 0) / job.file.size * 100);
  }
  await api(job, 'create', { path, size: String(job.file.size), isdir: '0', rtype: '0', uploadid: job.state.uploadId,
    block_list: JSON.stringify(job.state.hashes) }, true);
  job.state.result = { path, notice: 'Uploaded. Baidu public sharing is not supported by this official API.' };
  return job.state.result;
}
