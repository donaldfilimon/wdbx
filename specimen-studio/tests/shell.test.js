import { test, expect } from 'bun:test';
import { matchShortcut } from '../app/shell/shortcuts';
import { buildCommands } from '../app/shell/commands';

const key = (k, mods = {}, tag = 'BODY') => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  target: { tagName: tag, isContentEditable: false },
  ...mods,
});

test('palette opens from anywhere, including inputs', () => {
  expect(matchShortcut(key('k', { metaKey: true }))).toBe('palette');
  expect(matchShortcut(key('k', { ctrlKey: true }, 'INPUT'))).toBe('palette');
});

test('layout shortcuts never fire while typing', () => {
  expect(matchShortcut(key('`'))).toBe('dock');
  expect(matchShortcut(key('`', {}, 'INPUT'))).toBeNull();
  expect(matchShortcut(key('`', {}, 'TEXTAREA'))).toBeNull();
  expect(matchShortcut(key('b', { metaKey: true }))).toBe('sidebar');
  expect(matchShortcut(key('b', { metaKey: true }, 'INPUT'))).toBeNull();
  expect(matchShortcut(key('\\', { ctrlKey: true }))).toBe('split');
  expect(matchShortcut(key('a'))).toBeNull();
});

const noop = () => {};
const ctx = (over = {}) => ({
  view: 'studio',
  go: noop,
  openSide: noop,
  closeSide: noop,
  save: noop,
  load: noop,
  reset: noop,
  toggleTheme: noop,
  toggleSidebar: noop,
  toggleDock: noop,
  ...over,
});

test('every view is reachable; side targets exclude the current view and settings', () => {
  const cmds = buildCommands(ctx());
  const go = cmds.filter((c) => c.group === 'Go to').map((c) => c.id);
  expect(go).toEqual([
    'go:studio',
    'go:nodes',
    'go:memory',
    'go:activity',
    'go:engine',
    'go:store',
    'go:lab',
    'go:specification',
    'go:settings',
  ]);
  const side = cmds
    .filter((c) => c.group === 'Open to the side')
    .map((c) => c.id);
  expect(side).not.toContain('side:studio');
  expect(side).not.toContain('side:settings');
  expect(side).toContain('side:nodes');
});

test('close side pane appears only with a split, and shortcuts are unique', () => {
  expect(buildCommands(ctx()).some((c) => c.id === 'close-side')).toBe(false);
  const cmds = buildCommands(ctx({ split: 'nodes' }));
  expect(cmds.some((c) => c.id === 'close-side')).toBe(true);
  const keys = cmds.map((c) => c.shortcut).filter(Boolean);
  expect(new Set(keys).size).toBe(keys.length);
});

test('no command label collides with a pinned e2e button name', () => {
  const pinned =
    /network|cancel|cycle trace|run dossier|add pattern|open navigation|dismiss error/i;
  for (const c of buildCommands(ctx({ split: 'nodes' }))) {
    expect(c.label).not.toMatch(pinned);
  }
});
