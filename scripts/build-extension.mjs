import { build } from 'vite';
import { readFile, writeFile, copyFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
const root = resolve('browser-extension');
const outDir = resolve('dist-extension');
await build({ configFile: false, root, base: './', build: { outDir, emptyOutDir: true, target: 'chrome116' } });
const manifest = JSON.parse(await readFile(resolve(outDir, 'manifest.json'), 'utf8'));
if (process.env.EXTENSION_GOOGLE_CLIENT_ID) {
  if (!/^[\w-]+\.apps\.googleusercontent\.com$/.test(process.env.EXTENSION_GOOGLE_CLIENT_ID)) throw Error('Invalid Google client ID');
  manifest.oauth2.client_id = process.env.EXTENSION_GOOGLE_CLIENT_ID;
}
await writeFile(resolve(outDir, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
await copyFile(resolve('src-tauri/icons/128x128.png'), resolve(outDir, 'icon.png'));
await copyFile(resolve('browser-extension/INSTALL.md'), resolve(outDir, 'INSTALL.md'));
await copyFile(resolve('LICENSE'), resolve(outDir, 'LICENSE'));
await mkdir(resolve(outDir, 'licenses'), { recursive: true });
await copyFile(resolve('node_modules/spark-md5/LICENSE'), resolve(outDir, 'licenses/spark-md5.txt'));
console.log('Browser extension built: dist-extension (Chrome / Edge; Google sign-in requires Chrome)');
