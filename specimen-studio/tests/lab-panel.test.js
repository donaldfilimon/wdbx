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
  for (const tab of ['Image', 'Generate', 'Models', 'Artifacts (0)'])
    expect(out).toContain(`>${tab}</button>`.replace('</button>', ''));
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
