import { test, expect } from 'bun:test';
import { readFile } from 'node:fs/promises';

const css = await readFile(
  new URL('../app/studio.css', import.meta.url),
  'utf8',
);

test('studio.css uses theme tokens instead of color literals', () => {
  const literals =
    css.match(/#[0-9a-fA-F]{3,8}\b|\b(?:rgb|rgba|hsl|hsla|oklch)\(/g) ?? [];
  expect(literals).toEqual([]);
});

test('studio.css defines no color layer of its own', () => {
  expect(css).not.toMatch(/:root\s*\{[^}]*--(?:ink|teal|line|pale)\s*:/);
});
