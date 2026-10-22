import test, { beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { upload as google } from '../providers/google.js';
import { upload as onedrive } from '../providers/onedrive.js';
import { upload as baidu } from '../providers/baidu.js';
import { access } from '../auth.js';
import SparkMD5 from 'spark-md5';
const response = (body, status = 200, headers = {}) => new Response(JSON.stringify(body), { status, headers });
beforeEach(() => {
  globalThis.chrome = { storage: { session: { get: async key => ({ [key]: { generation: 'bound', access: 'fixture-token', expires: Date.now() + 3600000 } }) } }, identity: { getAuthToken: async () => ({ token: 'fixture-token' }) } };
});
const job = (provider, size = 10) => ({ provider, binding: 'bound', id: 'fixture-id', destination: '/delivery/', path: '客户/a.txt', file: new File([new Uint8Array(size)], 'a.txt'), state: {}, publicShare: false });

test('changing account prevents old queued tasks from accessing a new account', async () => {
  await assert.rejects(access('onedrive', 'other-account'), /binding changed/);
});
test('Google recovers final response loss by ID and never starts a second upload', async () => {
  const task = job('google'); task.state = { id: 'reserved-id', session: 'https://www.googleapis.com/upload/session' };
  const calls = [];
  globalThis.fetch = async (url, options) => { calls.push({url, options}); return response(url.includes('size') ? { id: 'reserved-id', size: '10', webViewLink: 'https://drive.google.com/file/d/reserved-id/view' } : { webViewLink: 'https://drive.google.com/file/d/reserved-id/view' }); };
  const result = await google(task, () => {});
  assert.match(result.url, /reserved-id/); assert.equal(calls.length, 2);
  assert.ok(calls.every(call => !call.options.method));
});
test('Google resumable PUT uses trusted URL and server offset after network failure', async () => {
  const task = job('google', 524288); task.state = { id: 'reserved', session: 'https://www.googleapis.com/upload/session' };
  let put = 0; const progress = [];
  globalThis.fetch = async (url, options) => {
    if (url.includes('/upload/session')) {
      assert.equal(options.headers.Authorization, 'Bearer fixture-token');
      if (++put === 1) return new Response(null, { status: 308, headers: { Range: 'bytes=0-262143' } });
      assert.equal(options.headers['Content-Range'], 'bytes 262144-524287/524288');
      assert.equal(options.body.size, 262144); return response({ id: 'reserved', size: '524288' });
    }
    if (url.includes('files/reserved?fields=id')) return response({}, 404);
    if (url.includes('files?')) return response({ files: [{ id: 'folder' }] });
    return response({ webViewLink: 'https://drive.google.com/file/d/reserved/view' });
  };
  await google(task, p => progress.push(p)); assert.equal(put, 2);
});
test('OneDrive upload sends no bearer to upload URL and share failure retries only sharing', async () => {
  const task = job('onedrive'); task.publicShare = true; task.state = { drive: 'drive', itemPath: '/items/parent:/a-fixture-id.txt', session: 'https://sn.up.1drv.com/session' };
  let uploads = 0, shares = 0;
  globalThis.fetch = async (url, options) => {
    if (url.includes('up.1drv.com')) {
      assert.equal(options.headers?.Authorization, undefined);
      if (options.method === 'PUT') { uploads++; return response({ id: 'item', size: 10, webUrl: 'https://onedrive.live.com/fixture' }, 201); }
      return response({ nextExpectedRanges: ['0-'] });
    }
    if (url.endsWith('/me/drive')) return response({ id: 'drive' });
    if (url.includes('createLink')) return ++shares === 1 ? response({}, 403) : response({ link: { webUrl: 'https://1drv.ms/fixture' } });
    return response({}, 404);
  };
  await assert.rejects(onedrive(task, () => {}), /403/); assert.equal(task.state.result.id, 'item');
  const result = await onedrive(task, () => {}); assert.equal(result.url, 'https://1drv.ms/fixture'); assert.equal(uploads, 1); assert.equal(shares, 2);
});
test('Baidu hashes bounded chunks and commits all hashes when precreate requests only a missing chunk', async () => {
  const task = job('baidu', 4 * 1024 * 1024 + 1); task.destination = '/apps/demo/'; task.path = 'a.txt';
  let committed, uploaded = 0;
  globalThis.fetch = async (url, options) => {
    const query = new URL(url).searchParams;
    if (url.includes('superfile2')) { uploaded++; assert.equal(query.get('partseq'), '1'); return response({ md5: SparkMD5.ArrayBuffer.hash(new Uint8Array(1).buffer) }); }
    if (query.get('method') === 'list') {
      const dir = query.get('dir'); return response({ errno: 0, list: dir === '/' ? [{ path: '/apps', isdir: 1 }] : dir === '/apps' ? [{ path: '/apps/demo', isdir: 1 }] : [] });
    }
    if (query.get('method') === 'precreate') return response({ errno: 0, uploadid: 'fixture', block_list: [1] });
    committed = JSON.parse(options.body.get('block_list')); return response({ errno: 0 });
  };
  await baidu(task, () => {}); assert.equal(uploaded, 1); assert.equal(committed.length, 2);
  assert.equal(committed[1], SparkMD5.ArrayBuffer.hash(new Uint8Array(1).buffer));
});
test('unknown OneDrive session status fails without creating a replacement upload', async () => {
  const task = job('onedrive'); task.state = { drive: 'drive', itemPath: '/items/parent:/a', session: 'https://sn.up.1drv.com/session' };
  globalThis.fetch = async (url, options) => {
    assert.notEqual(options.method, 'POST');
    return url.includes('up.1drv.com') ? response({}, 503) : url.endsWith('/me/drive') ? response({ id: 'drive' }) : response({}, 404);
  };
  await assert.rejects(onedrive(task, () => {}), /503/); assert.ok(task.state.session);
});
