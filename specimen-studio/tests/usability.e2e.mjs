import { chromium, expect, firefox, webkit } from '@playwright/test';
import { mkdir } from 'node:fs/promises';

const base = process.env.STUDIO_URL ?? 'http://localhost:3000';
const engine = process.env.STUDIO_BROWSER ?? 'chrome';
const launchers = { chrome: chromium, firefox, webkit };
if (!(engine in launchers))
  throw Error(`Unsupported STUDIO_BROWSER: ${engine}`);
const browser = await launchers[engine].launch(
  engine === 'chrome'
    ? { channel: 'chrome', headless: true }
    : { headless: true },
);
const failures = [];
const theme = process.env.STUDIO_THEME ?? 'dark';

async function scenario(name, exercise) {
  // Every scenario gets disposable browser storage, never the user's workspace.
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 },
    reducedMotion: 'reduce',
  });
  await context.addInitScript((value) => {
    localStorage.setItem('wdbx-studio-theme', value);
  }, theme);
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  try {
    await page.goto(base);
    await expect(
      page.getByText('Stored on this device', { exact: true }),
    ).toBeVisible({ timeout: 15000 });
    // Ensure post-hydration stability before testing interactive state.
    await page.waitForTimeout(100);
    await exercise(page);
    expect(errors).toEqual([]);
    console.log(`PASS: ${name}`);
  } catch (error) {
    failures.push(`${name}: ${error.message}`);
  } finally {
    await context.close();
  }
}

try {
  await scenario('empty prompts cannot be submitted', async (page) => {
    const run = page.getByRole('button', { name: 'Run cycle', exact: true });
    const prompt = page.getByLabel('Prompt your specimen', { exact: true });
    await expect(run).toBeDisabled({ timeout: 10000 });
    await prompt.fill('   ');
    await expect(run).toBeDisabled({ timeout: 1000 });
    await prompt.fill('hello');
    await expect(prompt).toHaveValue('hello');
    await expect(run).toBeEnabled({ timeout: 10000 });
    await prompt.press('Enter');
    await expect(page.locator('.answer-text')).not.toBeEmpty({
      timeout: 10000,
    });
    await expect(prompt).toHaveValue('');
    await expect(run).toBeDisabled({ timeout: 1000 });
  });

  await scenario(
    'mobile prompts stay reachable after running and reloading',
    async (page) => {
      await page
        .getByRole('button', { name: 'What is 2 + 2?', exact: true })
        .click();
      await expect(page.locator('.answer-text')).toHaveText('4');
      await expect(
        page.getByText('Stored on this device', { exact: true }),
      ).toBeVisible();
      await page.reload();
      await expect(page.locator('.answer-text')).toHaveText('4');
      for (const width of [390, 768]) {
        await page.setViewportSize({ width, height: 844 });
        await page.evaluate(() => window.scrollTo(0, 0));
        const reachable = await page
          .locator('.composer')
          .evaluate((element) => {
            const box = element.getBoundingClientRect();
            return box.top >= 0 && box.bottom <= innerHeight;
          });
        expect(
          reachable,
          `Composer must fit the first screen at ${width}px`,
        ).toBe(true);
      }
      await page.setViewportSize({ width: 390, height: 844 });
      await mkdir(new URL('../work/', import.meta.url), { recursive: true });
      await page.screenshot({
        path: new URL(
          `../work/usability-mobile-${engine}-${theme}.png`,
          import.meta.url,
        ).pathname,
        fullPage: true,
      });
    },
  );

  await scenario(
    'teach an unmatched prompt without replacing a composer draft',
    async (page) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      const prompt = page.getByLabel('Prompt your specimen', { exact: true });
      const input = 'cobalt orchard lantern';
      await prompt.fill(input);
      const runBtn = page.getByRole('button', {
        name: 'Run cycle',
        exact: true,
      });
      await expect(runBtn).toBeEnabled({ timeout: 5000 });
      await runBtn.click();
      const teach = page.getByRole('button', {
        name: 'Teach this prompt',
        exact: true,
      });
      await expect(teach).toBeVisible();
      await prompt.fill('unfinished next question');
      await teach.click();
      const dialog = page.getByRole('dialog');
      await expect(
        dialog.getByLabel('Original pattern', { exact: true }),
      ).toHaveValue(input);
      // Closing without teaching preserves the original draft and return focus.
      await page.keyboard.press('Escape');
      await expect(dialog).toHaveCount(0);
      await expect(prompt).toHaveValue('unfinished next question');
      await expect(teach).toBeFocused();
      // A later ordinary editor must not reuse the unmatched prompt override.
      await page
        .getByRole('navigation', { name: 'Specimen workflow' })
        .getByRole('button')
        .filter({ hasText: 'Teach pattern' })
        .click();
      await expect(
        dialog.getByLabel('Original pattern', { exact: true }),
      ).toHaveValue('unfinished next question');
      await page.keyboard.press('Escape');
      await teach.click();
      await dialog
        .getByLabel('Node name', { exact: true })
        .fill('Orchard response');
      await dialog
        .getByLabel('Response or action', { exact: true })
        .fill('A small lesson remembered.');
      await dialog
        .getByRole('button', { name: 'Teach pattern', exact: true })
        .click();
      await expect(dialog).toHaveCount(0);
      await expect(prompt).toHaveValue('unfinished next question');
      await prompt.fill(input);
      const runBtn2 = page.getByRole('button', {
        name: 'Run cycle',
        exact: true,
      });
      await expect(runBtn2).toBeEnabled({ timeout: 5000 });
      await runBtn2.click();
      await expect(page.locator('.answer-text')).toHaveText(
        'A small lesson remembered.',
      );
      await expect(teach).toHaveCount(0);
      await expect(
        page.getByText('Stored on this device', { exact: true }),
      ).toBeVisible();
      await page.reload();
      await expect(page.locator('.answer-text')).toHaveText(
        'A small lesson remembered.',
      );
    },
  );

  await scenario(
    'creates proposals via review (multiple unmatched + Review context), dismisses and reconsiders them',
    async (page) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      // Teach a Type B node via the workflow to enable Type B proposal synthesis in review
      await page
        .getByRole('navigation', { name: 'Specimen workflow' })
        .getByRole('button')
        .filter({ hasText: 'Teach pattern' })
        .click();
      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      await dialog
        .getByLabel('Node name', { exact: true })
        .fill('Repeat Observer');
      await dialog.locator('#node-type').selectOption('B');
      await dialog
        .getByLabel('Original pattern', { exact: true })
        .fill('observer seed');
      await dialog
        .getByLabel('Response or action', { exact: true })
        .fill('Tracks repeated unmatched inputs for proposals.');
      await dialog
        .getByRole('button', { name: 'Teach pattern', exact: true })
        .click();
      await expect(dialog).toHaveCount(0);

      // Produce multiple unmatched prompts (distinct inputs repeated to trigger Type B >=2 counts)
      const prompt = page.getByLabel('Prompt your specimen', { exact: true });
      const probes = ['zephyr probe alpha', 'zephyr probe beta'];
      for (const probe of probes) {
        for (let k = 0; k < 2; k += 1) {
          await prompt.fill(probe);
          await expect(prompt).toHaveValue(probe, { timeout: 2000 });
          await prompt.press('Enter');
          await expect(page.locator('.answer-text').last()).toBeVisible({
            timeout: 10000,
          });
          await expect(prompt).toHaveValue('', { timeout: 5000 });
          await expect(
            page.getByText('Stored on this device', { exact: true }),
          ).toBeVisible();
        }
      }

      // Explicit view navigation to Activity, then explicit review trigger
      await page
        .locator('nav[aria-label="Main navigation"] a[href="?view=activity"]')
        .click();
      const reviewBtn = page.getByRole('button', {
        name: 'Review context',
        exact: true,
      });
      await expect(reviewBtn).toBeVisible();
      await reviewBtn.click();

      // Non-flaky waits for proposals created by review
      await page.waitForTimeout(500);
      await expect(
        page.getByText('Learning proposals', { exact: false }),
      ).toBeVisible({ timeout: 10000 });
      // Confirm count via summary (robust to render order)
      await expect(
        page.getByText('2 pending · 0 learned · 0 dismissed', { exact: false }),
      ).toBeVisible({ timeout: 10000 });
      for (const probe of probes) {
        await expect(page.locator('strong', { hasText: probe })).toBeVisible({
          timeout: 10000,
        });
      }

      // Dismiss first proposal
      const p1 = page.locator('li', { hasText: probes[0] });
      await p1.getByRole('button', { name: 'Dismiss', exact: true }).click();
      await expect(p1.locator('span', { hasText: 'dismissed' })).toBeVisible({
        timeout: 5000,
      });

      // Reconsider it back to pending
      await p1.getByRole('button', { name: 'Reconsider', exact: true }).click();
      await expect(p1.locator('span', { hasText: 'pending' })).toBeVisible({
        timeout: 5000,
      });

      // Second proposal remains pending
      const p2 = page.locator('li', { hasText: probes[1] });
      await expect(p2.locator('span', { hasText: 'pending' })).toBeVisible({
        timeout: 5000,
      });
    },
  );

  if (failures.length) throw Error(failures.join('\n\n'));
  console.log(`Usability regressions passed in ${engine} (${theme})`);
} finally {
  await browser.close();
}
