import { chromium, expect } from '@playwright/test';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
const base = process.env.STUDIO_URL ?? 'http://localhost:3000';
const out = new URL('../../../work/', import.meta.url).pathname;
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const context = await browser.newContext({
  viewport: { width: 1568, height: 1000 },
  reducedMotion: 'reduce',
});
const page = await context.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
page.on('console', (message) => {
  if (message.type() === 'error') errors.push(message.text());
});
try {
  await page.goto(base);
  await expect(
    page.getByText('Stored on this device', { exact: true }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'What is 2 + 2?', exact: true })
    .click();
  await expect(page.locator('.answer-text')).toHaveText('4');
  await page.getByRole('button', { name: 'Cycle trace', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Cycle trace', exact: true }),
  ).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.trace-step').first()).toBeVisible();
  await page.getByRole('button', { name: 'Network', exact: true }).click();
  await page
    .getByRole('button', { name: 'Inspect Greeting', exact: true })
    .click();
  await expect(
    page.locator('.inspector').getByRole('heading', { name: 'Greeting' }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Inspect Calculate', exact: true })
    .click();
  await expect(
    page.locator('.inspector').getByRole('heading', { name: 'Calculate' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Run dossier', exact: true }).click();
  await expect(page.locator('.provenance-detail')).toContainText('Calculate');
  await page
    .getByRole('button', { name: 'Right green segment', exact: true })
    .click();
  await expect(page.locator('.toast')).toContainText('independent coin flips');
  await page.getByRole('button', { name: 'Right', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText(
    'already received feedback',
  );
  await page.getByRole('button', { name: 'Dismiss error' }).click();
  await page.getByRole('button', { name: 'Pin answer', exact: true }).click();
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=nodes"]')
    .click();
  await page.getByRole('button', { name: 'Add pattern', exact: true }).click();
  await page.getByLabel('Node name', { exact: true }).fill('Color check');
  await page
    .getByLabel('Original pattern', { exact: true })
    .fill('favorite color');
  await page
    .getByLabel('Response or action', { exact: true })
    .fill('Teal, like this studio.');
  await page
    .getByRole('button', { name: 'Teach pattern', exact: true })
    .click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
  await page
    .getByLabel('Prompt your specimen', { exact: true })
    .fill('favorite color');
  await page.getByRole('button', { name: 'Run cycle', exact: true }).click();
  await expect(page.locator('.answer-text')).toHaveText(
    'Teal, like this studio.',
  );
  await page.locator('a[href="?view=settings"]').click();
  await page.getByLabel('Specimen name', { exact: true }).fill('QA specimen');
  await page.getByLabel('Vote threshold', { exact: true }).fill('70');
  await page
    .getByRole('button', { name: 'Save settings', exact: true })
    .click();
  await expect(page.locator('.toast')).toContainText('Settings saved');
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
  await expect(
    page.getByText('Stored on this device', { exact: true }),
  ).toBeVisible();
  await page.reload();
  await expect(page.locator('.specimen-name')).toContainText('QA specimen');
  await expect(page.locator('.answer-text')).toHaveText(
    'Teal, like this studio.',
  );
  await page.locator('a[href="?view=settings"]').click();
  await expect(page.getByLabel('Vote threshold', { exact: true })).toHaveValue(
    '70',
  );
  const download = page.waitForEvent('download');
  await page
    .getByRole('button', { name: 'Save specimen', exact: true })
    .click();
  const file = await download;
  await file.saveAs(out + 'qa-specimen.json');
  const saved = JSON.parse(await readFile(out + 'qa-specimen.json', 'utf8'));
  if (
    saved.name !== 'QA specimen' ||
    saved.settings.voteThreshold !== 70 ||
    saved.nodes.length !== 9
  )
    throw Error('Save mismatch');
  await page.locator('input[type=file]').setInputFiles({
    name: 'broken.json',
    mimeType: 'application/json',
    buffer: Buffer.from('{"schema":"broken"}'),
  });
  await expect(page.getByRole('alert')).toContainText('not a supported');
  await page.getByRole('button', { name: 'Dismiss error' }).click();
  await page
    .locator('input[type=file]')
    .setInputFiles(out + 'qa-specimen.json');
  await expect(page.getByRole('dialog')).toContainText('QA specimen');
  await page
    .getByRole('button', { name: 'Load specimen', exact: true })
    .click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=specification"]')
    .click();
  await expect(page.locator('.chapter-count')).toHaveText('26 chapters');
  await page
    .locator('nav[aria-label="Specification chapters"] a')
    .nth(8)
    .click();
  await expect(page.locator('article')).toContainText(
    'Pattern-node activation',
  );
  await expect(page.locator('.reference-content')).toContainText(
    'Native implementation profile',
  );
  await page.screenshot({ path: out + 'studio-reader-desktop.png' });
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
  await page
    .getByRole('button', { name: 'What is 2 + 2?', exact: true })
    .click();
  await expect(page.locator('.answer-text')).toHaveText('4');
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: out + 'studio-desktop-final.png' });
  const desktopLayout = await page.evaluate(() => {
    const prompt = document.querySelector('.composer')?.getBoundingClientRect();
    return {
      overflow:
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth,
      promptVisible: Boolean(
        prompt && prompt.top < innerHeight && prompt.bottom > 0,
      ),
    };
  });
  if (desktopLayout.overflow || !desktopLayout.promptVisible)
    throw Error('Desktop console layout failed');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: out + 'studio-mobile-final.png',
    fullPage: true,
  });
  if (
    await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
  )
    throw Error('Mobile overflow');
  await expect(page.locator('.sidebar')).not.toHaveClass(/is-open/);
  await page
    .getByRole('button', { name: 'Open navigation', exact: true })
    .click();
  await expect(page.locator('.sidebar')).toHaveClass(/is-open/);
  await expect(page.locator('.nav-backdrop')).toBeVisible();
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=specification"]')
    .click();
  await expect(page.locator('.sidebar')).not.toHaveClass(/is-open/);
  await expect(page.locator('.nav-backdrop')).toHaveCount(0);
  await expect(
    page.getByRole('heading', { name: 'Specification', exact: true }),
  ).toBeVisible();
  if (
    await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
  )
    throw Error('Reader mobile overflow');
  const webmcp = await page.evaluate(() =>
    Boolean(document.modelContext?.registerTool),
  );
  if (errors.length) throw Error(errors.join('\n'));
  await writeFile(
    out + 'browser-checks.json',
    JSON.stringify(
      {
        passed: true,
        desktop: '1568×1000',
        mobile: '390×844',
        checks: [
          'arithmetic',
          'topology and trace switching',
          'node inspection',
          'provenance',
          'feedback deduplication',
          'pinning',
          'node creation',
          'custom response',
          'settings and rename',
          'autosave reload',
          'JSON download',
          'malformed import rejection',
          'JSON restore',
          'chapter navigation',
          'desktop console fit',
          'mobile overflow',
          'mobile drawer lifecycle',
        ],
        webmcpSupported: webmcp,
        errors,
      },
      null,
      2,
    ),
  );
  console.log(
    'Browser acceptance passed: all core workflows; desktop and mobile; zero runtime errors. WebMCP native support: ' +
      webmcp,
  );
} finally {
  await browser.close();
}
