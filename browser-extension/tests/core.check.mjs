import test from 'node:test';
import assert from 'node:assert/strict';
import { UploadQueue } from '../queue.js';
import { folderEntries, fileEntries, parts } from '../files.js';
import { validateCallback, pkce } from '../auth.js';
import { sessionURL, nextOffset } from '../providers/common.js';
import { googleOffset } from '../providers/google.js';
const tick = () => new Promise(resolve => setImmediate(resolve));

test('queue pause covers later batches, retries and one global concurrency limit', async () => {
  const releases = [], starts = [];
  const queue = new UploadQueue(job => { starts.push(job.path); return new Promise((resolve, reject) => releases.push({ resolve, reject })); }, () => {}, 1);
  const settings = { google: { directory: '/delivery/' }, publicShare: false };
  queue.add([{ path: 'a' }, { path: 'b' }], ['google'], settings, { google: 'binding' });
  await tick(); assert.deepEqual(starts, ['a']);
  queue.pause(true); queue.add([{ path: 'c' }], ['google'], settings, {});
  releases[0].reject(Error('offline')); await tick();
  queue.retry(queue.jobs[0]); queue.retry(queue.jobs[0]); await tick();
  assert.deepEqual(starts, ['a']); assert.equal(queue.running, 0);
  queue.pause(false); await tick(); assert.deepEqual(starts, ['a', 'a']);
  releases[1].resolve({}); await tick(); assert.equal(starts.at(-1), 'b');
  releases[2].resolve({}); await tick(); assert.equal(starts.at(-1), 'c');
  releases[3].resolve({}); await tick(); assert.equal(queue.running, 0);
  assert.ok(queue.jobs.every(job => job.status === 'completed'));
});
test('folders preserve unicode roots, same names and empty leaves without reading whole blobs', async () => {
  const file = name => ({ name, kind: 'file', getFile: async () => new File(['same'], name) });
  const dir = (name, children) => ({ name, kind: 'directory', async *values() { yield* children; } });
  const entries = await folderEntries(dir('客户资料', [dir('甲', [file('a.txt')]), dir('乙', [file('a.txt')]), dir('空目录', [])]));
  assert.deepEqual(entries.map(e => e.path), ['客户资料/甲/a.txt', '客户资料/乙/a.txt', '客户资料/空目录']);
  assert.equal(entries[2].directory, true);
  assert.deepEqual(fileEntries([new File(['x'], 'a.txt')]).map(e => e.path), ['a.txt']);
  assert.throws(() => parts('../escape')); assert.throws(() => parts('bad\x00path'));
});
test('OAuth callback rejects wrong origin, path, state and missing code', async () => {
  const expected = 'https://extension.chromiumapp.org/oauth2';
  assert.equal(validateCallback(`${expected}?code=ok&state=nonce`, expected, 'nonce'), 'ok');
  for (const url of ['https://evil.test/oauth2?code=ok&state=nonce', `${expected}/other?code=ok&state=nonce`, `${expected}?code=ok&state=wrong`, `${expected}?state=nonce`])
    assert.throws(() => validateCallback(url, expected, 'nonce'));
  const value = await pkce(); assert.equal(value.verifier.length, 43);
  assert.equal(value.challenge, Buffer.from(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value.verifier))).toString('base64url'));
});
test('session URL allowlists prevent bearer/preauthorized URL redirection', () => {
  assert.equal(sessionURL('https://www.googleapis.com/upload/session', 'google'), 'https://www.googleapis.com/upload/session');
  assert.ok(sessionURL('https://sn123.up.1drv.com/session?private=fixture', 'onedrive'));
  for (const url of ['https://sn.up.1drv.com.evil.test/a', 'http://sn.up.1drv.com/a', 'https://user:pass@sn.up.1drv.com/a', 'https://sn.up.1drv.com:444/a'])
    assert.throws(() => sessionURL(url, 'onedrive'));
  assert.throws(() => sessionURL('https://evil.test/a', 'google'));
});
test('only server-confirmed offsets can resume uploads', () => {
  assert.equal(nextOffset({ nextExpectedRanges: ['327680-'] }, 1000000), 327680);
  assert.throws(() => nextOffset({ nextExpectedRanges: ['-1-'] }, 100));
  assert.throws(() => nextOffset({ nextExpectedRanges: ['100-'] }, 100));
  assert.equal(googleOffset(new Response(null, { headers: { Range: 'bytes=0-262143' } }), 1000000), 262144);
  assert.throws(() => googleOffset(new Response(null, { headers: { Range: 'bytes=9-99' } }), 1000));
});
