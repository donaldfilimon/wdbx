import { test, expect } from 'bun:test';
import { readFile } from 'node:fs/promises';
import {
  THEME_KEY,
  applyTheme,
  resolveTheme,
  storeTheme,
} from '../app/shell/theme';

const mem = (v) => ({ getItem: (k) => (k === THEME_KEY ? v : null) });

test('dark is the default', () => {
  expect(resolveTheme(undefined)).toBe('dark');
  expect(resolveTheme(mem(null))).toBe('dark');
  expect(resolveTheme(mem('purple'))).toBe('dark');
});

test('a stored light choice wins', () => {
  expect(resolveTheme(mem('light'))).toBe('light');
});

test('throwing storage falls back without throwing', () => {
  const broken = {
    getItem() {
      throw new Error('blocked');
    },
    setItem() {
      throw new Error('blocked');
    },
  };
  expect(resolveTheme(broken)).toBe('dark');
  expect(() => storeTheme(broken, 'light')).not.toThrow();
});

test('applyTheme toggles the dark class', () => {
  const set = new Set();
  const root = {
    classList: {
      toggle: (c, on) => (on ? set.add(c) : set.delete(c)),
    },
  };
  applyTheme(root, 'dark');
  expect(set.has('dark')).toBe(true);
  applyTheme(root, 'light');
  expect(set.has('dark')).toBe(false);
});

const TOKENS = [
  '--bg',
  '--surface',
  '--surface-2',
  '--raised',
  '--line',
  '--line-strong',
  '--ink',
  '--ink-soft',
  '--muted',
  '--teal',
  '--teal-strong',
  '--teal-soft',
  '--on-teal',
  '--amber',
  '--amber-soft',
  '--danger',
  '--danger-soft',
  '--violet',
  '--violet-soft',
  '--info',
  '--info-soft',
  '--shadow',
  '--sidebar-bg',
  '--sidebar-ink',
  '--sidebar-muted',
  '--sidebar-active',
  '--sidebar-accent',
  '--knob',
  '--focus',
  '--background',
  '--foreground',
  '--popover',
  '--popover-foreground',
  '--border',
  '--input',
  '--ring',
  '--muted-foreground',
  '--accent',
  '--accent-foreground',
  '--primary',
  '--primary-foreground',
];

test('both theme blocks define every token', async () => {
  const css = await readFile(
    new URL('../app/theme.css', import.meta.url),
    'utf8',
  );
  const block = (sel) => {
    const start = css.indexOf(`${sel} {`);
    expect(start).toBeGreaterThanOrEqual(0);
    return css.slice(start, css.indexOf('}', start));
  };
  for (const sel of [':root', '.dark']) {
    for (const t of TOKENS) expect(block(sel)).toContain(`${t}:`);
  }
});
