async (page) => {
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    localStorage.clear();
    window.uploadCalls = [];
    window.nextSelection = null;
    const callbacks = {};
    const listeners = {};
    let nextId = 1;
    let queue = { paused: false, waiting: 0, running: 0 };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: fn => { const id = nextId++; callbacks[id] = fn; return id; },
      unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        window.uploadCalls.push({cmd, args});
        if (cmd === 'plugin:event|listen') { listeners[args.event] = callbacks[args.handler]; return nextId++; }
        if (cmd === 'initialize_app') return {
          queue, tasks: [], plugins: ['baidu','google','onedrive'].map(id => ({ manifest: { id, apiVersion: 1, version: '1.0.0' }, installed: true, enabled: true })),
          settings: { baiduDefaultDir:'/apps/demo/', googleDefaultDir:'/demo/', onedriveDefaultDir:'/demo/', maxConcurrentFiles:2, retryLimit:3, sharePeriodDays:0, copyTemplate:'', autoCopy:false },
          auth: [], capabilities: [], dataDir:'C:/fixture', logDir:'C:/fixture/logs'
        };
        if (cmd === 'plugin:dialog|open') return window.nextSelection;
        if (cmd === 'set_queue_paused') { queue = { ...queue, paused: args.paused }; return queue; }
        if (cmd === 'enqueue_paths') {
          queue = { ...queue, waiting: queue.waiting + args.paths.length };
          listeners['queue-update']?.({ event: 'queue-update', payload: queue });
          return { accepted: args.paths.length, duplicates: 0, tasks: [] };
        }
        return 1;
      }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  });
  await page.setViewportSize({width: 1180,height: 900});
  await page.goto('http://127.0.0.1:1420');
  await page.getByRole('button', {name:'选择文件上传', exact:true}).click();
  if (await page.evaluate(() => window.uploadCalls.some(c => c.cmd === 'enqueue_paths'))) throw Error('Cancelling picker enqueued a job');
  await page.evaluate(() => { window.nextSelection = ['C:/测试/a.txt','C:/测试/b.txt']; });
  await page.getByRole('button', {name:'选择文件上传', exact:true}).click();
  await page.getByText('0 项处理中 · 2 项排队', {exact:true}).waitFor();
  await page.getByRole('button', {name:'暂停队列',exact:true}).click();
  await page.getByText('队列已暂停', {exact:true}).waitFor();
  await page.evaluate(() => { window.nextSelection = 'C:/客户资料'; });
  await page.getByRole('button', {name:'上传整个文件夹',exact:true}).click();
  await page.getByText('0 项处理中 · 3 项排队', {exact:true}).waitFor();
  const calls = await page.evaluate(() => window.uploadCalls);
  const dialogs = calls.filter(c => c.cmd === 'plugin:dialog|open');
  if (!dialogs.at(-1).args.options.directory || dialogs.at(-1).args.options.multiple) throw Error('Folder picker options invalid');
  if (!dialogs[1].args.options.multiple || dialogs[1].args.options.directory) throw Error('File picker options invalid');
  if (JSON.stringify(calls.filter(c=>c.cmd==='enqueue_paths').map(c=>c.args.paths)) !== JSON.stringify([['C:/测试/a.txt','C:/测试/b.txt'],['C:/客户资料']])) throw Error('Selected paths changed');
  await page.screenshot({path:'output/playwright/upload-queue-paused.png',fullPage:true});
  await page.getByRole('combobox',{name:'Language / 语言'}).selectOption('en');
  await page.getByRole('button',{name:'Resume queue',exact:true}).click();
  await page.getByText('Queue active',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Upload a folder',exact:true}).waitFor();
  await page.setViewportSize({width:960,height:680});
  if (await page.evaluate(()=>document.documentElement.scrollWidth>960)) throw Error('Upload controls overflow');
  await page.screenshot({path:'output/playwright/upload-queue-english.png',fullPage:true});
  if(errors.length) throw Error(errors.join(';'));
  return 'PASS: clickable +, cancel picker, multi-file selection, whole folder selection, queue pause/add/resume, live counts, English and minimum width (mock IPC)';
}
