async (page) => {
  await page.addInitScript(() => {
    const platform = (url) => ({ state: 'completed', progress: 100, uploadedBytes: 42, totalBytes: 42, retryCount: 0, share: { url } });
    const task = { id: 'fixture', batchId: 'fixture', fileName: '中文发货.zip', fileSize: 42, localPath: 'C:/fixture/中文发货.zip', fileHash: 'fixture', createdAt: 1, updatedAt: 1, state: 'completed', baidu: platform('https://example.test/baidu'), google: platform('https://example.test/google'), onedrive: platform('https://example.test/onedrive') };
    const settings = { baiduDefaultDir: '/apps/demo/', googleDefaultDir: '/自动发货/', onedriveDefaultDir: '/自动发货/', datedSubdirectory: false, maxConcurrentFiles: 2, retryLimit: 3, sharePeriodDays: 0, copyTemplate: '{filename}\n{google_url}\n{onedrive_url}', autoCopy: false };
    window.smokeCalls = [];
    localStorage.removeItem("drive-delivery-language");
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        window.smokeCalls.push({ cmd, args });
        if (cmd === 'initialize_app') return { plugins: ["baidu", "google", "onedrive"].map(id => ({ manifest: { id, apiVersion: 1, version: "1.0.0" }, installed: true, enabled: true })), tasks: [task], settings, auth: ['baidu', 'google', 'onedrive'].map(provider => ({ provider, authenticated: false, available: false, message: '测试数据：尚未授权' })), capabilities: [], dataDir: 'C:/fixture', logDir: 'C:/fixture/logs' };
        if (cmd === 'search_history') return [{ ...task, uploadedAt: 1, taskStatus: 'completed', baiduUrl: task.baidu.share.url, googleUrl: task.google.share.url, onedriveUrl: task.onedrive.share.url }];
        if (cmd === 'save_settings') return args.settings;
        if (cmd.startsWith('copy_')) return 'fixture copied';
        return 1;
      }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  });
  await page.setViewportSize({ width: 1180, height: 780 });
  await page.goto('http://127.0.0.1:1420');
  await page.getByRole('heading', { name: '中文发货.zip' }).waitFor();
  await page.getByRole('button', { name: '复制 OneDrive' }).click();
  if (!await page.evaluate(() => window.smokeCalls.some(c => c.cmd === 'copy_task_result' && c.args.provider === 'onedrive'))) throw new Error('OneDrive task copy did not reach API');
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'output/playwright/three-drive-tasks.png', fullPage: true });
  await page.getByRole('button', { name: '绑定网盘', exact: true }).click();
  await page.getByRole('region', { name: '微软网盘绑定', exact: true }).waitFor();
  await page.getByRole('button', { name: '绑定微软网盘', exact: true }).waitFor();
  await page.getByText('上传位置和其他设置（一般不用改）', { exact: true }).click();
  await page.getByLabel('OneDrive 默认目录').fill('/中文目录/');
  await page.getByRole('button', { name: '保存设置', exact: true }).click();
  if (!await page.evaluate(() => window.smokeCalls.some(c => c.cmd === 'save_settings' && c.args.settings.onedriveDefaultDir === '/中文目录/'))) throw new Error('OneDrive directory not saved');
  await page.screenshot({ path: 'output/playwright/three-drive-settings.png', fullPage: true });
  await page.getByRole('button', { name: '历史记录', exact: true }).click();
  await page.getByRole('columnheader', { name: 'OneDrive', exact: true }).waitFor();
  await page.getByRole('button', { name: 'OneDrive', exact: true }).click();
  if (!await page.evaluate(() => window.smokeCalls.some(c => c.cmd === 'copy_history_result' && c.args.provider === 'onedrive'))) throw new Error('OneDrive history copy did not reach API');
  await page.screenshot({ path: 'output/playwright/three-drive-history.png', fullPage: true });
  console.log('PASS: three-provider UI, settings save, task copy and history copy (mock IPC only)');
}
