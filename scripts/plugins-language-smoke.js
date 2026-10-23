async (page) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const defaults = ['baidu', 'google', 'onedrive'].map(id => ({ manifest: { id, apiVersion: 1, version: '1.0.0' }, installed: true, enabled: true }));
    let plugins = JSON.parse(localStorage.getItem('fixture-plugins') || 'null') || defaults;
    window.pluginCalls = [];
    const platform = { state: 'completed', progress: 100, uploadedBytes: 42, totalBytes: 42, retryCount: 0, share: { url: 'https://example.test/share' } };
    const task = { id: 'fixture', batchId: 'fixture', fileName: '客户资料.zip', localPath: 'C:/客户资料.zip', fileSize: 42, createdAt: 1, updatedAt: 1, state: 'completed', baidu: { ...platform, state: 'skipped', progress: 0, uploadedBytes: 0, share: {} }, google: platform, onedrive: { ...platform, state: 'skipped', progress: 0, uploadedBytes: 0, share: {} } };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => 1, unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        window.pluginCalls.push({ cmd, args });
        if (cmd === 'initialize_app') return {
          plugins, tasks: [task],
          settings: { baiduDefaultDir: '/apps/demo/', googleDefaultDir: '/自动发货/', onedriveDefaultDir: '/自动发货/', datedSubdirectory: false, maxConcurrentFiles: 2, retryLimit: 3, sharePeriodDays: 0, copyTemplate: '客户：{filename}', autoCopy: false },
          auth: defaults.map(item => ({ provider: item.manifest.id, authenticated: false, available: false })),
          capabilities: [], dataDir: 'C:/fixture', logDir: 'C:/fixture/logs'
        };
        if (cmd === 'update_drive_plugin') {
          if (args.manifest && JSON.parse(args.manifest).apiVersion !== 1) throw new Error('配置错误：此插件需要其他版本的软件，或尚未支持该网盘');
          plugins = plugins.map(item => item.manifest.id !== args.id ? item : { ...item, installed: args.action === 'remove' ? false : args.action === 'install' ? true : item.installed, enabled: ['install', 'enable'].includes(args.action) });
          localStorage.setItem('fixture-plugins', JSON.stringify(plugins));
          return plugins;
        }
        if (cmd === 'search_history') return [{ ...task, uploadedAt: 1, taskStatus: 'completed', googleUrl: platform.share.url }];
        return 1;
      }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  });
  await page.setViewportSize({ width: 1180, height: 900 });
  await page.goto('http://127.0.0.1:1420');
  await page.evaluate(() => { localStorage.clear(); });
  await page.reload();
  await page.getByRole('combobox', { name: 'Language / 语言' }).selectOption('en');
  await page.getByRole('heading', { name: 'Drop once. Deliver to your enabled drives.' }).waitFor();
  await page.getByRole('heading', { name: '客户资料.zip', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Copy Google', exact: true }).click();
  if (!await page.evaluate(() => window.pluginCalls.some(c => c.cmd === 'copy_task_result' && c.args.language === 'en'))) throw new Error('Copy language missing from IPC');
  await page.setViewportSize({ width: 960, height: 680 });
  const overlap = await page.evaluate(() => { const nav = document.querySelector('nav').getBoundingClientRect(); const tools = document.querySelector('.header-tools').getBoundingClientRect(); return (nav.left < tools.right && nav.right > tools.left && nav.top < tools.bottom && nav.bottom > tools.top) || document.documentElement.scrollWidth > 960; });
  if (overlap) throw new Error('English navigation overflows the minimum window size');
  await page.setViewportSize({ width: 1180, height: 900 });
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'output/playwright/english-tasks.png', fullPage: true });
  await page.getByRole('button', { name: 'Connect drives', exact: true }).click();
  const google = page.locator('[data-plugin="google"]');
  await google.getByRole('button', { name: 'Disable', exact: true }).click();
  if (await page.getByRole('region', { name: 'Connect Google Drive', exact: true }).count()) throw new Error('Disabled drive still has a binding card');
  await google.getByRole('button', { name: 'Remove plugin', exact: true }).click();
  await google.getByText('Not installed', { exact: true }).waitFor();
  await page.reload();
  await page.getByRole('button', { name: 'Connect drives', exact: true }).click();
  await google.getByText('Not installed', { exact: true }).waitFor();
  await page.locator('.plugin-import input').setInputFiles({ name: 'wrong.json', mimeType: 'application/json', buffer: Buffer.from('{"id":"unknown"}') });
  await page.getByText('Import failed. Choose a supported .drive-plugin.json file (up to 4 KB).', { exact: true }).waitFor();
  await page.locator('.plugin-import input').setInputFiles({ name: 'google.drive-plugin.json', mimeType: 'application/json', buffer: Buffer.from('{"id":"google","apiVersion":2,"version":"1.0.0"}') });
  await page.getByText('Configuration: This plugin needs a different app version or an unsupported drive adapter', { exact: true }).waitFor();
  await page.locator('.plugin-import input').setInputFiles({ name: 'google.drive-plugin.json', mimeType: 'application/json', buffer: Buffer.from('{"id":"google","apiVersion":1,"version":"1.0.0"}') });
  await google.getByText('Enabled', { exact: true }).waitFor();
  const binding = page.getByRole('region', { name: 'Connect Google Drive', exact: true });
  await binding.getByRole('button', { name: 'Connect Google Drive', exact: true }).click();
  await binding.getByText('Import Google configuration (recommended)', { exact: true }).waitFor();
  const text = await binding.innerText();
  if (/[\u4e00-\u9fff]/.test(text)) throw new Error('Chinese leaked into English binding guide: ' + text);
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'output/playwright/english-plugins-guide.png', fullPage: true });
  await page.getByText('Upload folders and other settings (usually no changes needed)', { exact: true }).click();
  if (await page.locator('textarea').inputValue() !== '客户：{filename}') throw new Error('Custom template changed by language switch');
  await page.getByRole('button', { name: 'History', exact: true }).click();
  await page.getByRole('columnheader', { name: 'Uploaded', exact: true }).waitFor();
  await page.getByRole('combobox', { name: 'Language / 语言' }).selectOption('zh-CN');
  await page.getByRole('columnheader', { name: '上传时间', exact: true }).waitFor();
  await page.getByRole('button', { name: '绑定网盘', exact: true }).click();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'output/playwright/chinese-plugins.png', fullPage: true });
  if (errors.length) throw new Error(errors.join('; '));
  return 'PASS: language persistence, plugin disable/remove/import/reinstall, unsupported package, English binding guide, task/history labels and custom-content preservation (mock IPC only)';
}
