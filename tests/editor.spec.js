const { test, expect } = require('@playwright/test');
test.beforeEach(async ({ page }) => { await page.goto('/'); });
const yaml = page => page.getByRole('textbox', { name: 'config.yaml', exact: true });
test('indent, outdent, selection and Unicode', async ({ page }) => {
  const area = yaml(page);
  await area.fill('😀one\ntwo');
  await area.selectText();
  await area.press('Tab');
  await expect(area).toHaveValue('  😀one\n  two');
  await area.press('Shift+Tab');
  await expect(area).toHaveValue('😀one\ntwo');
  await area.evaluate(el => el.setSelectionRange(0, 0));
  await area.press('Shift+Tab');
  await expect(area).toHaveValue('😀one\ntwo');
});
test('autoindent and native undo redo', async ({ page }) => {
  const area = yaml(page);
  await area.evaluate(el => { el.value = '  value'; el.dispatchEvent(new Event('input', { bubbles: true })); });
  await area.focus();
  await area.press('End');
  await area.press('Enter');
  await expect(area).toHaveValue('  value\n  ');
  await area.press('ControlOrMeta+z');
  await expect(area).toHaveValue('  value');
  await expect(page.locator('.nb-toolbar').first()).toContainText('Unsaved changes');
  await area.press('ControlOrMeta+Shift+z');
  await expect(area).toHaveValue('  value\n  ');
});
test('Tab is undoable and Escape Tab moves focus', async ({ page }) => {
  const area = yaml(page);
  await area.evaluate(el => { el.value = 'value'; el.dispatchEvent(new Event('input', { bubbles: true })); });
  await area.focus();
  await area.press('End');
  await area.press('Tab');
  await expect(area).toHaveValue('value  ');
  await area.press('ControlOrMeta+z');
  await expect(area).toHaveValue('value');
  await area.press('Escape');
  await area.press('Tab');
  await expect(area).not.toBeFocused();
});
test('validation and successful backend save', async ({ page }) => {
  const area = yaml(page);
  const editor = page.locator('.nb-editor').first();
  await area.fill('bad: [');
  await editor.getByRole('button', { name: 'Validate' }).click();
  await expect(editor.locator('.nb-validation').first()).toContainText('Line');
  await area.fill('enabled: false\n');
  await expect(editor).toContainText('Unsaved changes');
  const responsePromise = page.waitForResponse(r => r.url().endsWith('/api/save'));
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  const response = await responsePromise;
  expect(response.status(), await response.text()).toBe(200);
  await expect(editor.getByRole('button', { name: 'Save', exact: true })).toBeDisabled();
  await expect(editor.locator('.nb-toolbar')).toContainText('Saved');
});
test('large input line count and gutter scroll alignment', async ({ page }) => {
  const area = yaml(page);
  await area.fill(Array.from({ length: 2000 }, (_, i) => `key${i}: value`).join('\n'));
  await expect(page.locator('.nb-status').first()).toContainText('2000 lines');
  await area.evaluate(el => { el.scrollTop = 931; el.dispatchEvent(new Event('scroll')); });
  await expect.poll(async () => area.evaluate(el => {
    const gutter = el.parentElement.querySelector('.nb-gutter > div');
    return Math.abs(new DOMMatrix(getComputedStyle(gutter).transform).m42 + el.scrollTop);
  })).toBeLessThan(1);
});
test('read-only mode and dark theme', async ({ page }) => {
  const area = page.getByRole('textbox', { name: 'reference.yaml' });
  await expect(area).toHaveAttribute('readonly', '');
  await area.focus();
  await area.press('a');
  await area.press('Enter');
  await expect(area).toHaveValue('example: read-only\n');
  const editor = page.locator('.nb-editor').filter({ has: area });
  await expect(editor).toHaveAttribute('data-theme', 'dark');
  await expect(editor.getByRole('button', { name: 'Save', exact: true })).toBeDisabled();
});
for (const [name, content] of [['config.yaml (dark)', 'enabled: false\n'], ['query.sql (dark)', 'SELECT 42;\n']]) {
  test(`${name} supports editing, validation, and saving in dark theme`, async ({ page }) => {
    const area = page.getByRole('textbox', { name, exact: true });
    const editor = page.locator('.nb-editor').filter({ has: area });
    await expect(editor).toHaveAttribute('data-theme', 'dark');
    await expect(area).toBeEditable();
    await area.fill(content);
    await editor.getByRole('button', { name: 'Validate', exact: true }).click();
    await expect(editor.locator('.nb-validation').first()).toHaveText('Syntax valid.');
    await editor.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(editor.locator('.nb-toolbar')).toContainText('Saved');
    await expect(area).toHaveValue(content);
  });
}
test('failed save preserves unsaved content', async ({ page }) => {
  await page.route('**/api/save', route => route.fulfill({ status: 500, body: 'Storage unavailable' }));
  await yaml(page).fill('enabled: true');
  const editor = page.locator('.nb-editor').first();
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(editor.getByRole('alert')).toContainText('Storage unavailable');
  await expect(editor.locator('.nb-toolbar')).toContainText('Unsaved changes');
});
test('native clipboard copy, cut, and paste', async ({ page }) => {
  const area = yaml(page);
  await area.fill('clipboard 😀');
  await area.selectText();
  await area.press('ControlOrMeta+c');
  await area.press('ControlOrMeta+x');
  await expect(area).toHaveValue('');
  await area.press('ControlOrMeta+v');
  await expect(area).toHaveValue('clipboard 😀');
});
test('editing during save remains dirty', async ({ page }) => {
  let release;
  const pending = new Promise(resolve => { release = resolve; });
  await page.route('**/api/save', async route => { await pending; await route.fulfill({ status: 200, body: 'Saved' }); });
  const area = yaml(page);
  const editor = page.locator('.nb-editor').first();
  await area.fill('enabled: true');
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(editor.locator('.nb-toolbar')).toContainText('Saving');
  await area.fill('enabled: false');
  release();
  await expect(editor.locator('.nb-toolbar')).toContainText('Unsaved changes');
  await expect(area).toHaveValue('enabled: false');
});
test('SQL syntax validation', async ({ page }) => {
  await page.getByRole('textbox', { name: 'query.sql', exact: true }).fill('SELECT FROM');
  const editor = page.locator('.nb-editor').nth(1);
  await editor.getByRole('button', { name: 'Validate' }).click();
  await expect(editor.locator('.nb-validation').first()).not.toContainText('Syntax valid');
});
