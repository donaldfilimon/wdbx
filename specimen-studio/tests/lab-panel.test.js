import { expect, mock, test } from 'bun:test';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

mock.module('@tauri-apps/api/core', () => ({
  Channel: class {},
  invoke: async () => [],
}));
const { BrowserLabIntro, LabPanel } = await import('../app/lab/lab-panel');

test('the desktop Lab keeps the native contract selectors and has four tabs', () => {
  const out = renderToStaticMarkup(
    h(LabPanel, { specimen: { name: 'S' }, onSnapshot: () => {} }),
  );
  expect(out).toMatch(/class="native-lab[ "]/);
  expect(out).toMatch(/class="runtime-pills[^"]*"[\s\S]*Native Rust engine/);
  const tabs = [...out.matchAll(/role="tab"[^>]*>([^<]*)</g)].map((m) => m[1]);
  expect(tabs).toEqual(['Image', 'Generate', 'Models', 'Artifacts (0)']);
  // Switching tabs keeps Generate's prompt, seed and model: it stays mounted.
  expect(out).toContain('Generate locally');
  expect(out).toContain('Ready to inspect or compose');
  // The image workspace is keyboard reachable and named.
  expect(out).toContain(
    'aria-label="Drop or paste an image here, or activate to open one"',
  );
});

test('the browser edition explains what the desktop adds', () => {
  const out = renderToStaticMarkup(h(BrowserLabIntro));
  expect(out).toContain('native-lab');
  expect(out).toContain('Bring your specimen to the desktop');
  expect(out).toContain('Browser runtime');
});

test('arrow keys nudge the focus box and keep it inside the image', async () => {
  const { nudgeFocus } = await import('../app/lab/image-workspace');
  const f = { x: 0, y: 0, width: 0.5, height: 0.5 };
  expect(nudgeFocus(f, 'ArrowRight')).toEqual({ ...f, x: 0.05 });
  expect(nudgeFocus(f, 'ArrowLeft')).toEqual(f);
  expect(nudgeFocus({ ...f, y: 0.49 }, 'ArrowDown').y).toBe(0.5);
  expect(nudgeFocus(f, 'Enter')).toBeNull();
});
