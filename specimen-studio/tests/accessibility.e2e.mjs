import AxeBuilder from '@axe-core/playwright';
import { chromium, expect, firefox, webkit } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';

const base = process.env.STUDIO_URL ?? 'http://localhost:3000';
const engine = process.env.STUDIO_BROWSER ?? 'chrome';
const launchers = { chrome: chromium, firefox, webkit };
if (!(engine in launchers))
  throw Error(`Unsupported STUDIO_BROWSER: ${engine}`);
const out = new URL('../work/', import.meta.url).pathname;
// STUDIO_THEME=light|dark seeds the stored theme; unset keeps the default and
// the CI receipt names.
const theme = process.env.STUDIO_THEME;
const suffix = theme ? `-${theme}` : '';
await mkdir(out, { recursive: true });
const browser = await launchers[engine].launch(
  engine === 'chrome'
    ? { channel: 'chrome', headless: true }
    : { headless: true },
);
const evidence = {
  schema: 1,
  engine,
  headSha: process.env.GITHUB_SHA ?? null,
  widths: {},
  errors: [],
};

async function audit(page, width, state) {
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze();
  const violations = result.violations.filter((violation) =>
    ['serious', 'critical'].includes(violation.impact),
  );
  const unnamed = await page
    .locator('button, a[href], input, select, textarea, summary')
    .evaluateAll((elements) =>
      elements
        .filter((element) => {
          const style = getComputedStyle(element);
          const bounds = element.getBoundingClientRect();
          return (
            style.visibility !== 'hidden' &&
            style.display !== 'none' &&
            bounds.width > 0 &&
            bounds.height > 0
          );
        })
        .filter((element) => {
          const labelledBy = element.getAttribute('aria-labelledby');
          const labelledText = labelledBy
            ? labelledBy
                .split(/\s+/)
                .map((id) => document.getElementById(id)?.textContent ?? '')
                .join(' ')
            : '';
          const labels =
            'labels' in element
              ? [...(element.labels ?? [])]
                  .map((label) => label.textContent ?? '')
                  .join(' ')
              : '';
          return ![
            element.getAttribute('aria-label') ?? '',
            labelledText,
            labels,
            element.textContent ?? '',
            element.getAttribute('title') ?? '',
            element.getAttribute('placeholder') ?? '',
            element.getAttribute('alt') ?? '',
          ].some((value) => value.trim());
        })
        .map((element) => element.outerHTML.slice(0, 180)),
    );
  const orphanedControls = await page
    .locator('input:not([type="hidden"]), select, textarea')
    .evaluateAll((elements) =>
      elements
        .filter(
          (element) =>
            !element.labels?.length &&
            !element.getAttribute('aria-label') &&
            !element.getAttribute('aria-labelledby') &&
            !element.getAttribute('title'),
        )
        .map((element) => element.outerHTML.slice(0, 180)),
    );
  if (violations.length || unnamed.length || orphanedControls.length) {
    throw Error(
      JSON.stringify(
        { width, state, violations, unnamed, orphanedControls },
        null,
        2,
      ),
    );
  }
  evidence.widths[width].audits.push({
    state,
    seriousOrCriticalViolations: 0,
    unnamedControls: 0,
    orphanedControls: 0,
  });
}

async function requireTarget(page, role, name) {
  const control = page.getByRole(role, { name, exact: true });
  const box = await control.boundingBox();
  if (!box || box.width < 43.99 || box.height < 43.99) {
    throw Error(
      `${name} is below the 44-pixel target: ${box?.width ?? 0}×${box?.height ?? 0}`,
    );
  }
}

async function navigate(page, width, href) {
  if (width <= 760) {
    await page.getByRole('button', { name: 'Open navigation' }).click();
  }
  await page
    .locator(`nav[aria-label="Main navigation"] a[href="${href}"]`)
    .click();
  if (width <= 760) {
    await expect(page.locator('.sidebar')).not.toHaveClass(/is-open/);
    await expect(
      page.getByRole('button', { name: 'Open navigation' }),
    ).toBeFocused();
  }
}

try {
  for (const width of [390, 768, 1440]) {
    evidence.widths[width] = { audits: [], touchTargets: [] };
    const context = await browser.newContext({
      viewport: { width, height: 1000 },
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
    page.on('pageerror', (error) => evidence.errors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') evidence.errors.push(message.text());
    });
    await page.goto(base);
    await expect(
      page.getByText('Stored on this device', { exact: true }),
    ).toBeVisible();
    await audit(page, width, 'studio');

    await page
      .getByRole('button', { name: 'What is 2 + 2?', exact: true })
      .click();
    await expect(page.locator('.answer-text')).toHaveText('4');
    await page.getByRole('button', { name: 'Run dossier' }).click();
    await expect(
      page.getByRole('button', { name: 'Run dossier' }),
    ).toHaveAttribute('aria-expanded', 'true');
    await page.locator('.native-evidence summary').click();
    await audit(page, width, 'open-provenance-dossier');

    await page.getByRole('button', { name: 'Cycle trace' }).click();
    await expect(
      page.getByRole('button', { name: 'Cycle trace' }),
    ).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('.trace-outcome')).toContainText('Completed:');
    await audit(page, width, 'cycle-trace-completed');

    if (width === 1440) {
      await page.evaluate(() => {
        window.__wdbxAcceptanceSetTimeout = window.setTimeout;
        window.__wdbxAcceptancePending = [];
        window.setTimeout = (callback, delay, ...arguments_) => {
          if (delay === 0) {
            window.__wdbxAcceptancePending.push(() => callback(...arguments_));
            return 0;
          }
          return window.__wdbxAcceptanceSetTimeout(
            callback,
            delay,
            ...arguments_,
          );
        };
      });
      const longPrompt = Array.from(
        { length: 450 },
        () => 'what is 2 + 2',
      ).join('; ');
      await page.getByLabel('Prompt your specimen').fill(longPrompt);
      await page.getByRole('button', { name: 'Run cycle' }).click();
      const cancel = page.getByRole('button', { name: 'Cancel' });
      await expect(cancel).toBeVisible();
      await expect(page.locator('.trace-outcome')).toContainText('Processing:');
      await cancel.click();
      await page.evaluate(() => {
        for (const resume of window.__wdbxAcceptancePending.splice(0)) resume();
      });
      await expect(page.locator('.trace-outcome')).toContainText('Cancelled ·');
      evidence.widths[width].traceOutcomes = ['Processing', 'Cancelled'];

      await page.getByLabel('Prompt your specimen').fill(longPrompt);
      await page.getByRole('button', { name: 'Run cycle' }).click();
      await expect(cancel).toBeVisible();
      await page.getByRole('switch', { name: /^Jitter for / }).click();
      await page.evaluate(() => {
        for (const resume of window.__wdbxAcceptancePending.splice(0)) resume();
      });
      await expect(page.locator('.trace-outcome')).toContainText('Failed ·');
      evidence.widths[width].traceOutcomes.push('Failed');
      await page.getByRole('button', { name: 'Dismiss error' }).click();

      await page.evaluate(() => {
        window.setTimeout = window.__wdbxAcceptanceSetTimeout;
        delete window.__wdbxAcceptanceSetTimeout;
        delete window.__wdbxAcceptancePending;
      });

      await page
        .getByRole('button', { name: 'What is 2 + 2?', exact: true })
        .click();
      await expect(page.locator('.trace-outcome')).toContainText('Completed:');
      evidence.widths[width].traceOutcomes.push('Completed');
    }

    await page.getByRole('button', { name: 'Network' }).click();
    await page
      .getByRole('button', { name: 'Right green segment', exact: true })
      .click();
    await expect(
      page.getByRole('button', { name: 'Right', exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByRole('button', { name: 'Right green segment', exact: true }),
    ).toBeDisabled();
    await expect(page.locator('.toast')).toContainText(
      'independent coin flips',
    );
    await audit(page, width, 'disabled-contributor-feedback');

    await page
      .getByRole('button', { name: 'Fit topology to canvas', exact: true })
      .click();
    for (let index = 0; index < 3; index += 1) {
      await page
        .getByRole('button', { name: 'Zoom in topology', exact: true })
        .click();
    }
    const topology = page.getByRole('region', {
      name: 'Scrollable specimen topology',
    });
    await topology.focus();
    const beforeScroll = await topology.evaluate(
      (element) => element.scrollLeft,
    );
    await page.keyboard.press('ArrowRight');
    await expect
      .poll(() => topology.evaluate((element) => element.scrollLeft))
      .toBeGreaterThan(beforeScroll);
    for (const [role, name] of [
      ['button', 'Network'],
      ['button', 'Cycle trace'],
      ['button', 'Zoom out topology'],
      ['button', 'Fit topology to canvas'],
      ['button', 'Zoom in topology'],
      ['button', 'Run dossier'],
      ['button', 'Right'],
      ['button', 'Wrong'],
    ]) {
      await requireTarget(page, role, name);
      evidence.widths[width].touchTargets.push(name);
    }
    await audit(page, width, 'zoomed-keyboard-scrollable-topology');

    if (width <= 760) {
      const opener = page.getByRole('button', { name: 'Open navigation' });
      await opener.click();
      await expect(
        page.getByRole('button', { name: 'Close navigation', exact: true }),
      ).toBeFocused();
      for (let index = 0; index < 12; index += 1) {
        await page.keyboard.press('Tab');
        await expect
          .poll(() =>
            page.evaluate(() =>
              Boolean(document.activeElement?.closest('#workspace-navigation')),
            ),
          )
          .toBe(true);
      }
      await audit(page, width, 'mobile-drawer');
      await page.keyboard.press('Escape');
      await expect(opener).toBeFocused();
    }

    await navigate(page, width, '?view=specification');
    await expect(
      page.getByRole('heading', { name: 'Specification', exact: true }),
    ).toBeVisible();
    await audit(page, width, 'specification-reader');

    if (width === 1440) {
      await page.goto(`${base}?view=studio&split=nodes`);
      await expect(
        page.getByRole('region', { name: 'Side pane: Node library' }),
      ).toBeVisible();
      await audit(page, width, 'split-studio-nodes');
      await page.goto(base);
      await expect(
        page.getByText('Stored on this device', { exact: true }),
      ).toBeVisible();
    }

    await navigate(page, width, '?view=nodes');
    const addPattern = page.getByRole('button', { name: 'Add pattern' });
    await addPattern.focus();
    await page.keyboard.press('Enter');
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    for (let index = 0; index < 10; index += 1) {
      await page.keyboard.press('Tab');
      await expect
        .poll(() =>
          page.evaluate(() =>
            Boolean(
              document.activeElement?.closest('[role="dialog"]') ||
              document.activeElement?.matches('[data-base-ui-focus-guard]'),
            ),
          ),
        )
        .toBe(true);
    }
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);
    await expect(addPattern).toBeFocused();
    evidence.widths[width].keyboardContainment = 'passed';
    await page.screenshot({
      path: `${out}accessibility-${width}-${engine}${suffix}.png`,
      fullPage: true,
    });
    await context.close();
  }
  if (evidence.errors.length) throw Error(evidence.errors.join('\n'));
  evidence.passed = true;
  console.log(
    `Accessibility acceptance passed in ${engine} at 390, 768, and 1440 pixels.`,
  );
} finally {
  await writeFile(
    `${out}accessibility-checks-${engine}${suffix}.json`,
    `${JSON.stringify(evidence, null, 2)}\n`,
  );
  await browser.close();
}
