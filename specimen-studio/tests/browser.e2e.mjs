import { chromium, expect, firefox, webkit } from '@playwright/test';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
const base = process.env.STUDIO_URL ?? 'http://localhost:3000';
const engine = process.env.STUDIO_BROWSER ?? 'chrome';
const launchers = { chrome: chromium, firefox, webkit };
if (!(engine in launchers))
  throw Error(`Unsupported STUDIO_BROWSER: ${engine}`);
const out = new URL('../work/', import.meta.url).pathname;
await mkdir(out, { recursive: true });
const browser = await launchers[engine].launch(
  engine === 'chrome'
    ? { channel: 'chrome', headless: true }
    : { headless: true },
);
// STUDIO_THEME=light|dark seeds the stored theme; unset keeps the default and
// the CI receipt names.
const theme = process.env.STUDIO_THEME;
const suffix = theme ? `-${theme}` : '';
const artifact = (name, extension = 'png') =>
  `${out}${name}-${engine}${suffix}.${extension}`;
const context = await browser.newContext({
  viewport: { width: 1568, height: 1000 },
  reducedMotion: 'reduce',
});
if (theme) {
  await context.addInitScript((t) => {
    try {
      localStorage.setItem('wdbx-studio-theme', t);
    } catch {}
  }, theme);
}
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
  // Native kernel phases, in order; Index Rafts repeats once per chunk.
  const phases = (
    await page.locator('.trace-step strong').allTextContents()
  ).filter((phase, i, all) => phase !== all[i - 1]);
  expect(phases).toEqual([
    'Prepare',
    'Retrieve',
    'Index Rafts',
    'Deep scan',
    'Vote',
    'Compose',
  ]);
  await expect(page.locator('.trace-outcome')).toContainText(
    'Completed: What is 2 + 2?',
  );
  await page.getByRole('button', { name: 'Network', exact: true }).click();
  await page
    .getByRole('button', { name: 'Zoom in topology', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Fit topology to canvas', exact: true }),
  ).toContainText('110%');
  await page
    .getByRole('button', { name: 'Fit topology to canvas', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Fit topology to canvas', exact: true }),
  ).toContainText('100%');
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
  await expect(
    page.getByRole('button', { name: 'Run dossier', exact: true }),
  ).toHaveAttribute('aria-expanded', 'true');
  await page.locator('.native-evidence summary').click();
  await expect(page.locator('.native-evidence')).toContainText(
    'matchedPattern',
  );
  await expect(page.locator('.native-evidence')).toContainText(
    'confidenceScore',
  );
  await page
    .getByRole('button', { name: 'Right green segment', exact: true })
    .click();
  await expect(page.locator('.toast')).toContainText('independent coin flips');
  await expect(
    page.getByRole('button', { name: 'Right', exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole('button', { name: 'Right green segment', exact: true }),
  ).toBeDisabled();
  await page.getByRole('button', { name: 'Pin answer', exact: true }).click();
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=nodes"]')
    .click();
  // The node table windows rows by a fixed 76 px height; every rendered row
  // must be exactly that tall (a wrapped name once made rows much taller).
  // Measured at 800 px, where automatic table layout used to squeeze names.
  const wide = page.viewportSize();
  await page.setViewportSize({ width: 800, height: 900 });
  const nodeRows = page
    .getByRole('region', { name: 'Learned nodes' })
    .locator('tbody tr:not([aria-hidden])');
  await expect(nodeRows.first()).toBeVisible();
  const rowHeights = await nodeRows.evaluateAll((rows) =>
    rows.map((r) => Math.round(r.getBoundingClientRect().height)),
  );
  if (!rowHeights.length || rowHeights.some((h) => h !== 76))
    throw Error(`Node rows must be 76 px tall, got ${rowHeights.join(', ')}`);
  await page.setViewportSize(wide);
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
  const savedSpecimen = artifact('qa-specimen', 'json');
  await file.saveAs(savedSpecimen);
  const saved = JSON.parse(await readFile(savedSpecimen, 'utf8'));
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
  await page.locator('input[type=file]').setInputFiles(savedSpecimen);
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
  await page.screenshot({ path: artifact('studio-reader-desktop') });
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
  await page
    .getByRole('button', { name: 'What is 2 + 2?', exact: true })
    .click();
  await expect(page.locator('.answer-text')).toHaveText('4');
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: artifact('studio-desktop-final') });
  // Live engine: the last cycle's trace, votes and ATP, with data tables.
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=engine"]')
    .click();
  const liveTrace = page.getByRole('list', { name: 'Live cycle trace' });
  await expect(liveTrace).toContainText('Compose');
  await expect(page.getByRole('meter', { name: 'Valence' })).toBeVisible();
  await page.getByText('Show data').first().click();
  await expect(
    page.locator('caption', { hasText: 'Votes by node' }),
  ).toHaveCount(1);
  // Specification diagrams: the lifecycle shows the last cycle, the raft
  // planner asks the kernel, and pipeline stages open their chapters.
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=specification"]')
    .click();
  const chapters = page.locator('nav[aria-label="Specification chapters"] a');
  await chapters.nth(10).click();
  await expect(page.locator('[data-figure="ch11-1"]')).toContainText(
    'Last cycle reached Compose',
  );
  await chapters.nth(19).click();
  const planner = page.locator('[data-figure="ch20-2"]');
  await planner.getByLabel('Candidates (N)').fill('10');
  await expect(planner.locator('output')).toHaveText(
    /^10 candidates: 1 checkpoint of up to 256, each split across up to 5 rafts\.$/,
  );
  // Network: the kernel edits layers, traces a prompt, and the topology
  // tab pairs its 3D view with tables.
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=network"]')
    .click();
  await expect(page.getByLabel('Layer 1 neurons')).toBeVisible();
  await page.getByLabel('Neurons', { exact: true }).fill('48');
  await page.getByRole('button', { name: 'Add layer', exact: true }).click();
  await expect(
    page.locator('output').filter({ hasText: 'Added a 48-neuron layer.' }),
  ).toBeVisible();
  await expect(page.getByLabel('Layer 3 neurons')).toBeDisabled();
  await page.getByRole('button', { name: 'Trace', exact: true }).click();
  await expect(
    page
      .locator('table', {
        has: page.locator('caption', { hasText: 'Trace: output per layer' }),
      })
      .locator('tbody tr'),
  ).toHaveCount(3);
  // An edit clears the trace (it described the old network) and says which
  // other layer the kernel rewired.
  await page.getByLabel('Layer 1 neurons').fill('32');
  await page
    .getByRole('button', { name: 'Resize', exact: true })
    .first()
    .click();
  await expect(
    page
      .locator('output')
      .filter({ hasText: 'Layer 1 now has 32 neurons. Layer 2 was rewired' }),
  ).toBeVisible();
  await expect(
    page.locator('caption', { hasText: 'Trace: output per layer' }),
  ).toHaveCount(0);
  await page.getByRole('tab', { name: 'Topology' }).click();
  await expect(page.getByRole('region', { name: 'Nodes' })).toBeVisible();
  // Vision & models: the browser edition explains what the desktop adds.
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=lab"]')
    .click();
  await expect(page.locator('.native-lab')).toContainText(
    'Bring your specimen to the desktop',
  );
  await expect(page.locator('.runtime-pills')).toContainText('Browser runtime');
  // Store explorer: the browser edition reports its IndexedDB record.
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=store"]')
    .click();
  await expect(page.getByText('Browser · IndexedDB')).toBeVisible();
  await expect(
    page
      .getByRole('region', { name: 'Records' })
      .getByRole('cell', { name: 'nodes', exact: true }),
  ).toBeVisible();
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=specification"]')
    .click();
  await chapters.nth(1).click();
  await page
    .locator('[data-figure="ch2-1"]')
    .getByRole('link', { name: /Transient Memory when needed/ })
    .click();
  await expect(page.locator('article h2')).toHaveText(
    'Transient Memory and imagination',
  );
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
  // Command palette: keyboard open, filter, run; then a side pane opens.
  await page.keyboard.press('ControlOrMeta+k');
  const palette = page.getByRole('dialog', { name: 'Command palette' });
  await expect(palette).toBeVisible();
  await page.keyboard.type('go to memory');
  await expect(
    palette.getByRole('option', { name: 'Go to Memory', exact: true }),
  ).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.press('Enter');
  await expect(palette).toHaveCount(0);
  await expect(
    page.getByRole('heading', { level: 1, name: 'Memory', exact: true }),
  ).toBeVisible();
  await page.keyboard.press('ControlOrMeta+k');
  await expect(palette).toBeVisible();
  await page.keyboard.type('specification beside memory');
  // Click rather than Enter: cmdk selects the row under a resting pointer.
  await palette
    .getByRole('option', { name: 'Specification beside Memory', exact: true })
    .click();
  await expect(
    page.getByRole('region', { name: 'Side pane: Specification' }),
  ).toBeVisible();
  // A chapter clicked in the side pane keeps the split and the primary view.
  await page
    .getByRole('region', { name: 'Side pane: Specification' })
    .locator('nav[aria-label="Specification chapters"] a')
    .nth(3)
    .click();
  await expect(
    page.getByRole('region', { name: 'Side pane: Specification' }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { level: 1, name: 'Memory', exact: true }),
  ).toBeVisible();
  await expect(page).toHaveURL(/view=memory.*split=specification/);
  await page.getByRole('button', { name: 'Close side pane' }).click();
  // Kernel-backed views work on a direct load, before the kernel is ready.
  await page.goto(`${base}?view=network`);
  await expect(page.getByLabel('Layer 1 neurons')).toBeVisible();
  await expect(page.getByText('still loading')).toHaveCount(0);
  await page.goto(`${base}?view=specification&chapter=20`);
  await expect(page.locator('[data-figure="ch20-2"] output')).toHaveText(
    /^1,000 candidates: /,
  );
  // Each pane keeps its own search text.
  await page.goto(`${base}?view=nodes&split=memory`);
  await page.getByPlaceholder('Find a pattern or node…').fill('zz');
  await expect(page.getByPlaceholder('Search your memory…')).toHaveValue('');
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
  await expect(
    page.getByRole('region', { name: 'Side pane: Specification' }),
  ).toHaveCount(0);
  await page
    .locator('nav[aria-label="Main navigation"] a[href="?view=studio"]')
    .click();
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
    path: artifact('studio-mobile-final'),
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
  await page.goto(base);
  await expect(
    page.getByText('Stored on this device', { exact: true }),
  ).toBeVisible();
  for (const width of [390, 768, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page
      .getByRole('button', { name: 'Fit topology to canvas', exact: true })
      .click();
    await expect
      .poll(() =>
        page.locator('.topology').evaluate((canvas) => {
          const bounds = canvas.getBoundingClientRect();
          return [...canvas.querySelectorAll('.graph-node')].every((node) => {
            const rect = node.getBoundingClientRect();
            return (
              rect.left >= bounds.left - 1 &&
              rect.right <= bounds.right + 1 &&
              rect.top >= bounds.top - 1 &&
              rect.bottom <= bounds.bottom + 1
            );
          });
        }),
      )
      .toBe(true);
    await expect(
      page.getByRole('button', { name: 'Fit topology to canvas', exact: true }),
    ).toContainText('100%');
    await page
      .getByRole('button', { name: 'Zoom in topology', exact: true })
      .click();
    await page
      .getByRole('button', { name: 'Zoom in topology', exact: true })
      .click();
    await page
      .getByRole('button', { name: 'Zoom in topology', exact: true })
      .click();
    await expect(
      page.getByRole('button', { name: 'Zoom in topology', exact: true }),
    ).toBeDisabled();
    await expect
      .poll(() =>
        page
          .locator('.topology')
          .evaluate((canvas) => canvas.scrollWidth > canvas.clientWidth),
      )
      .toBe(true);
  }
  // Native clause scoping: ';' and 'then' both start a new clause, so the
  // negation inhibits only the calculation and the greeting still answers.
  for (const prompt of [
    "don't calculate 2+2; hello",
    "don't calculate 2+2 then hello",
  ]) {
    await page.locator('#prompt').fill(prompt);
    await page.getByRole('button', { name: 'Run cycle', exact: true }).click();
    await page
      .getByRole('button', { name: 'Cycle trace', exact: true })
      .click();
    await expect(
      page.locator('.answer-text').filter({ hasText: /^4$/ }),
    ).toHaveCount(0);
    await expect(
      page
        .locator('.answer-text')
        .filter({ hasText: /^(Hello|Hi|Hey|Greetings)\b/ }),
    ).toHaveCount(1);
    await expect(
      page
        .locator('.answer-text')
        .filter({ hasText: /leave that action alone/ }),
    ).toHaveCount(1);
  }
  const webmcp = await page.evaluate(() =>
    Boolean(document.modelContext?.registerTool),
  );
  if (errors.length) throw Error(errors.join('\n'));
  await writeFile(
    artifact('browser-checks', 'json'),
    JSON.stringify(
      {
        passed: true,
        headSha: process.env.GITHUB_SHA ?? null,
        engine,
        desktop: '1568×1000',
        mobile: '390×844',
        checks: [
          'arithmetic',
          'semicolon negation and conservative connective scope',
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
          'command palette and side pane',
          'live engine views',
          'specification diagrams',
          'store explorer',
          'network layers and topology',
          'lab browser intro',
        ],
        webmcpSupported: webmcp,
        errors,
      },
      null,
      2,
    ),
  );
  console.log(
    `Browser acceptance passed in ${engine}: all core workflows; desktop and mobile; zero runtime errors. WebMCP native support: ` +
      webmcp,
  );
} finally {
  await browser.close();
}
