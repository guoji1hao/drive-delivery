async (page) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    window.bindingCalls = [];
    window.bindingPending = false;
    window.bindingFail = false;
    const settings = { baiduDefaultDir: '/apps/demo/', googleDefaultDir: '/自动发货/', onedriveDefaultDir: '/自动发货/', datedSubdirectory: false, maxConcurrentFiles: 2, retryLimit: 3, sharePeriodDays: 0, copyTemplate: '{filename}', autoCopy: false };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => 1, unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        window.bindingCalls.push({ cmd, args });
        if (cmd === 'initialize_app') return { tasks: [], settings, auth: ['baidu', 'google', 'onedrive'].map(provider => ({ provider, authenticated: false, available: false, message: 'fixture' })), capabilities: [], dataDir: 'C:/fixture', logDir: 'C:/fixture/logs' };
        if (cmd.startsWith('save_') && cmd.endsWith('_credentials')) return { provider: cmd.split('_')[1], authenticated: false, available: true, message: 'fixture saved' };
        if (cmd.startsWith('clear_')) return { provider: cmd.split('_')[1], authenticated: false, available: true, message: 'fixture cleared' };
        if (cmd.startsWith('start_')) {
          if (window.bindingFail) throw new Error('network timeout');
          return { provider: cmd.split('_')[1], verificationUrl: 'https://example.test/login', deviceCode: 'fixture-handle', userCode: cmd.includes('google') ? undefined : 'ABCD-1234', expiresIn: 30, interval: 1 };
        }
        if (cmd.startsWith('poll_')) return { pending: window.bindingPending, authenticated: !window.bindingPending, message: 'fixture bound' };
        return 1;
      }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  });
  await page.setViewportSize({ width: 1180, height: 900 });
  await page.goto('http://127.0.0.1:1420');
  await page.getByRole('button', { name: '绑定网盘', exact: true }).click();
  const baidu = page.getByRole('region', { name: '百度网盘绑定', exact: true });
  const google = page.getByRole('region', { name: '谷歌网盘绑定', exact: true });
  const ms = page.getByRole('region', { name: '微软网盘绑定', exact: true });
  if (await google.getByLabel('应用编号（Client ID）').isVisible()) throw new Error('Developer inputs should start collapsed');
  await page.screenshot({ path: 'output/playwright/binding-simple.png', fullPage: true });
  await baidu.getByRole('button', { name: '绑定百度网盘', exact: true }).click();
  await baidu.getByText('保存并去登录', { exact: true }).waitFor();
  if (await page.evaluate(() => window.bindingCalls.some(c => c.cmd === 'start_baidu_auth'))) throw new Error('Missing setup must not call OAuth');
  await baidu.getByText('首次设置 · 不知道怎么填就看这里', { exact: true }).click();
  await google.getByRole('button', { name: '绑定谷歌网盘', exact: true }).click();
  await google.locator('input[type=file]').setInputFiles({ name: 'wrong.json', mimeType: 'application/json', buffer: Buffer.from('{"web":{}}') });
  await google.getByRole('alert').filter({ hasText: '桌面应用' }).waitFor();
  await google.locator('input[type=file]').setInputFiles({ name: 'desktop.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify({ installed: { client_id: '123-demo.apps.googleusercontent.com', client_secret: 'fixture' } })) });
  await google.getByText('配置已读好，直接点“保存并去登录”。', { exact: true }).waitFor();
  await page.screenshot({ path: 'output/playwright/binding-google-guide.png', fullPage: true });
  await google.getByRole('button', { name: '保存并去登录', exact: true }).click();
  await google.getByText('已绑定', { exact: true }).waitFor();
  await google.getByRole('button', { name: '换一个账号', exact: true }).click();
  await google.getByRole('button', { name: '解除绑定', exact: true }).click();
  await page.evaluate(() => { window.bindingFail = true; });
  await google.getByRole('button', { name: '绑定谷歌网盘', exact: true }).click();
  await google.getByRole('alert').filter({ hasText: '暂时连不上 Google' }).waitFor();
  await page.evaluate(() => { window.bindingFail = false; });
  await google.getByRole('button', { name: '绑定谷歌网盘', exact: true }).click();
  await google.getByText('已绑定', { exact: true }).waitFor();
  if (await page.evaluate(() => window.bindingCalls.filter(c => c.cmd === 'save_google_credentials').length) !== 1) throw new Error('Rebinding must reuse credentials');
  await ms.getByRole('button', { name: '绑定微软网盘', exact: true }).click();
  await ms.getByLabel('应用编号（应用程序 / 客户端 ID）').fill('11111111-1111-4111-8111-111111111111');
  await page.evaluate(() => { window.bindingPending = true; });
  await ms.getByRole('button', { name: '保存并去登录', exact: true }).click();
  await ms.getByText('ABCD-1234', { exact: true }).waitFor();
  await page.screenshot({ path: 'output/playwright/binding-device-code.png', fullPage: true });
  await ms.getByRole('button', { name: '暂时不绑', exact: true }).click();
  await ms.getByRole('button', { name: '绑定微软网盘', exact: true }).click();
  await ms.getByText('ABCD-1234', { exact: true }).waitFor();
  await page.getByRole('button', { name: '发货任务', exact: true }).click();
  await page.evaluate(() => { window.bindingPending = false; });
  await page.getByRole('button', { name: '绑定网盘', exact: true }).click();
  await ms.getByText('已绑定', { exact: true }).waitFor();
  if (await page.evaluate(() => window.bindingCalls.filter(c => c.cmd === 'save_onedrive_credentials').length) !== 1) throw new Error('OneDrive retry must reuse config');
  await page.screenshot({ path: 'output/playwright/binding-success.png', fullPage: true });
  if (errors.length) throw new Error(errors.join('; '));
  return 'PASS: first setup, Google import validation, saved-config rebind, network retry, device code, stop/restart and navigation during binding; mock IPC only';
}
