import { test, expect } from 'bun:test';
import { parseRoute, routeSearch } from '../app/state/navigation';

test('defaults to studio with no split', () => {
  expect(parseRoute('')).toEqual({ view: 'studio' });
});

test('keeps existing view and chapter URLs working', () => {
  expect(parseRoute('?view=specification&chapter=9')).toEqual({
    view: 'specification',
    chapter: 9,
  });
  expect(parseRoute('?view=settings')).toEqual({ view: 'settings' });
});

test('rejects unknown views and out-of-range chapters', () => {
  expect(parseRoute('?view=nope&chapter=27')).toEqual({ view: 'studio' });
  expect(parseRoute('?view=specification&chapter=0')).toEqual({
    view: 'specification',
  });
});

test('accepts a split view and drops invalid ones', () => {
  expect(parseRoute('?view=studio&split=nodes')).toEqual({
    view: 'studio',
    split: 'nodes',
  });
  expect(parseRoute('?view=studio&split=studio')).toEqual({ view: 'studio' });
  expect(parseRoute('?view=studio&split=bogus')).toEqual({ view: 'studio' });
  expect(parseRoute('?view=studio&split=settings')).toEqual({ view: 'studio' });
});

test('serializes in a stable order that round-trips', () => {
  const route = { view: 'studio', split: 'specification', chapter: 3 };
  expect(routeSearch(route)).toBe('?view=studio&chapter=3&split=specification');
  expect(parseRoute(routeSearch(route))).toEqual(route);
  expect(routeSearch({ view: 'nodes' })).toBe('?view=nodes');
});
