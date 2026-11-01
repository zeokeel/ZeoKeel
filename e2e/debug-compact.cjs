const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto('http://127.0.0.1:8788/', { waitUntil: 'networkidle' });
  await page.getByRole('complementary', { name: '工作区与会话' }).waitFor();
  await page.getByRole('button', { name: '新建会话', exact: true }).last().click();
  const composer = page.getByRole('textbox', { name: '消息输入框' });
  for (let i = 0; i < 7; i += 1) {
    await composer.fill(`compact-history-${i}`);
    await page.keyboard.press('Enter');
    await page.getByTestId('user-text').last().waitFor();
    await page.waitForFunction(() => !document.querySelector('[aria-label="停止生成"]'));
  }
  await composer.fill('/compact');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(3000);
  console.log('TOASTS', await page.locator('body').innerText().then(t => t.includes('较早消息已压缩')));
  console.log('DEBUG', await page.evaluate(() => Array.from(document.querySelectorAll('[data-testid="compaction-debug"]')).map(e => ({id: e.getAttribute('data-message-id'), through: e.getAttribute('data-through')}))));
  console.log('SEPARATOR', await page.locator('[role="separator"]').count());
  await browser.close();
})();
