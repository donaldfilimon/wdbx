import { chromium, expect, firefox, webkit } from '@playwright/test';

// Bundle the existing components into a disposable browser fixture; no preview
// server, production route, or runtime store participates in these checks.
const listeners = `import { useCallback } from 'react';
const handlers = new Map();
let canPrev = false, canNext = true;
const api = {
  canScrollPrev: () => canPrev, canScrollNext: () => canNext,
  scrollPrev() {}, scrollNext() {},
  on(event, handler) { if (!handlers.has(event)) handlers.set(event, new Set()); handlers.get(event).add(handler); return api; },
  off(event, handler) { handlers.get(event)?.delete(handler); return api; }
};
window.setCarouselScrollability = (prev, next, event) => { canPrev = prev; canNext = next; for (const handler of handlers.get(event) ?? []) handler(api); };
window.carouselListeners = () => Object.fromEntries([...handlers].map(([event, values]) => [event, values.size]));
export default function useEmblaCarousel() { return [useCallback(() => {}, []), api]; }
`;
const build = await Bun.build({
  entrypoints: [
    new URL('./ui-components.fixture.tsx', import.meta.url).pathname,
  ],
  target: 'browser',
  format: 'iife',
  plugins: [
    {
      name: 'embla-events',
      setup(builder) {
        builder.onLoad({ filter: /embla-carousel-react\/.*\.js$/ }, () => ({
          contents: listeners,
          loader: 'js',
        }));
      },
    },
  ],
});
if (!build.success) throw new Error(build.logs.join('\n'));
const browserName = process.env.STUDIO_BROWSER ?? 'chrome';
const browserType = new Map([
  ['chrome', chromium],
  ['firefox', firefox],
  ['webkit', webkit],
]).get(browserName);
if (!browserType) {
  throw new Error(`Unsupported STUDIO_BROWSER: ${browserName}`);
}
const browser = await browserType.launch(
  browserName === 'chrome'
    ? { channel: 'chrome', headless: true }
    : { headless: true },
);
try {
  const page = await browser.newPage({ viewport: { width: 900, height: 600 } });
  await page.setContent('<div id="root"></div>');
  await page.addScriptTag({ content: await build.outputs[0].text() });
  await expect(page.getByTestId('mobile')).toHaveText('false');
  await page.setViewportSize({ width: 767, height: 600 });
  await expect(page.getByTestId('mobile')).toHaveText('true');
  await page.setViewportSize({ width: 768, height: 600 });
  await expect(page.getByTestId('mobile')).toHaveText('false');
  await page.getByTestId('addon').click({ position: { x: 3, y: 3 } });
  await expect(page.getByRole('textbox', { name: 'Notes' })).toBeFocused();
  await page.getByTestId('link').click();
  await expect(page).toHaveURL(/#help$/);
  await expect(page.getByRole('textbox', { name: 'Notes' })).not.toBeFocused();
  // WebKit's native pointer activation need not focus an anchor. Keyboard
  // activation must retain the link's focus rather than delegate to the input.
  await page.getByTestId('link').focus();
  await page.getByTestId('link').press('Enter');
  await expect(page.getByTestId('link')).toBeFocused();
  await expect(
    page.getByRole('button', { name: 'Previous slide' }),
  ).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Next slide' })).toBeEnabled();
  expect(await page.evaluate(() => window.carouselListeners())).toEqual({
    reInit: 1,
    select: 1,
  });
  await page.evaluate(() =>
    window.setCarouselScrollability(true, false, 'select'),
  );
  await expect(
    page.getByRole('button', { name: 'Previous slide' }),
  ).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Next slide' })).toBeDisabled();
  await page.evaluate(() =>
    window.setCarouselScrollability(false, true, 'reInit'),
  );
  await expect(
    page.getByRole('button', { name: 'Previous slide' }),
  ).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Next slide' })).toBeEnabled();
  await page.getByRole('button', { name: 'Unmount carousel' }).click();
  expect(await page.evaluate(() => window.carouselListeners())).toEqual({
    reInit: 0,
    select: 0,
  });
  console.log('UI component browser regressions passed');
} finally {
  await browser.close();
}
