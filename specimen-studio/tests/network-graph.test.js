import { expect, test } from 'bun:test';
import {
  nextCamera,
  parseNetworkViewPrefs,
  visibleNodeRefs,
} from '../app/network/view-prefs';

test('network view prefs fall back when storage is empty or corrupt', () => {
  expect(parseNetworkViewPrefs(null).camera).toBe('orbit');
  expect(parseNetworkViewPrefs('nope').edgeBudget).toBe(600);
  expect(
    parseNetworkViewPrefs(
      JSON.stringify({
        camera: 'top',
        edges: 'local',
        follow: false,
        edgeBudget: 200,
      }),
    ),
  ).toEqual({
    camera: 'top',
    edges: 'local',
    follow: false,
    edgeBudget: 200,
  });
});

test('next camera cycles orbit, top, then side', () => {
  expect(nextCamera('orbit')).toBe('top');
  expect(nextCamera('top')).toBe('side');
  expect(nextCamera('side')).toBe('orbit');
});

test('neighborhood filter keeps the selection and its neighbors', () => {
  const links = [
    { from: 'a', to: 'b', hard: true },
    { from: 'c', to: 'd', hard: false },
  ];
  expect(visibleNodeRefs(['a', 'b', 'c', 'd'], 'a', 'local', links)).toEqual([
    'a',
    'b',
  ]);
  expect(visibleNodeRefs(['a', 'b'], undefined, 'local', links)).toEqual([
    'a',
    'b',
  ]);
});
