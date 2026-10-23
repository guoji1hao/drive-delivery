async (page) => {
  await page.addInitScript(() => {
    const state = {};
    const storage = { get: async keys => Object.fromEntries((Array.isArray(keys) ? keys : [keys]).map(k => [k, state[k]])), set: async data => Object.assign(state, data), setAccessLevel: async () => {} };
    window.chrome = { storage: { local: storage, session: storage }, runtime: { id: 'fixture', getManifest: () => ({ oauth2: { client_id: 'fixture.apps.googleusercontent.com' } }) }, identity: { getRedirectURL: () => 'https://fixture.chromiumapp.org/oauth2' } };
  });
  await page.setViewportSize({ width: 1180, height: 900 });
  await page.goto('http://127.0.0.1:1421');
  await page.locator('.drive').first().waitFor();
  if (await page.locator('.drive').count() !== 3) throw Error('Drive cards missing');
  await page.locator('#pause').click();
  await page.screenshot({ path: 'output/playwright/browser-skin.png', fullPage: true });
  await page.locator('#language').selectOption('en');
  await page.setViewportSize({ width: 390, height: 844 });
  if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)) throw Error('Mobile overflow');
  await page.keyboard.press('Tab');
  await page.locator('summary').first().focus();
  if (await page.locator('summary').first().evaluate(el => getComputedStyle(el).outlineStyle) === 'none') throw Error('Missing focus');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  if (await page.locator('#pause').evaluate(el => getComputedStyle(el).transitionDuration) !== '0s') throw Error('Reduced motion ignored');
  await page.screenshot({ path: 'output/playwright/browser-skin-mobile.png', fullPage: true });
  return 'PASS: browser cards, pause control, language, 390px layout, keyboard focus and reduced motion; mock Chrome storage only';
}
