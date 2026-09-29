import { expect, test } from 'bun:test';
import {
  atpNow,
  cycleSeries,
  phaseTimeline,
  voteBreakdown,
} from '../lib/specimen/insights';

const step = (phase, extra = {}) => ({
  id: phase,
  phase,
  detail: '',
  ...extra,
});

test('phaseTimeline collapses repeated Index Rafts checkpoints into coverage', () => {
  const t = phaseTimeline([
    step('Prepare', { count: 1 }),
    step('Retrieve', { count: 12 }),
    step('Index Rafts', { covered: 6, total: 12 }),
    step('Index Rafts', { covered: 12, total: 12 }),
    step('Index Rafts', { count: 12 }),
    step('Deep scan', { count: 3 }),
  ]);
  expect(t.map((p) => p.phase)).toEqual([
    'Prepare',
    'Retrieve',
    'Index Rafts',
    'Deep scan',
  ]);
  const rafts = t[2];
  expect(rafts.coverage).toBe(1);
  expect(rafts.checkpoints).toBe(2);
  expect(t[1].count).toBe(12);
});

test('phaseTimeline marks the last phase active while running', () => {
  const t = phaseTimeline([step('Prepare'), step('Retrieve')], true);
  expect(t.map((p) => p.state)).toEqual(['done', 'active']);
  const done = phaseTimeline([step('Prepare'), step('Compose')], false);
  expect(done.every((p) => p.state === 'done')).toBe(true);
});

test('voteBreakdown names nodes and sorts by confidence', () => {
  const nodes = [
    { ref: 'a', name: 'Calculate' },
    { ref: 'b', name: 'Greeting' },
  ];
  const cycle = {
    votes: [
      { nodeRef: 'b', confidence: 40, strength: 5 },
      { nodeRef: 'a', confidence: 90, strength: 6 },
      { nodeRef: 'gone', confidence: 10, strength: 1 },
    ],
  };
  expect(voteBreakdown(cycle, nodes)).toEqual([
    { node: 'Calculate', confidence: 90, strength: 6 },
    { node: 'Greeting', confidence: 40, strength: 5 },
    { node: 'Removed node', confidence: 10, strength: 1 },
  ]);
  expect(voteBreakdown(undefined, nodes)).toEqual([]);
});

test('cycleSeries summarizes the most recent cycles in time order', () => {
  const history = Array.from({ length: 5 }, (_, i) => ({
    id: `c${i}`,
    input: `q${i}`,
    createdAt: `2026-01-01T00:0${i}:00.000Z`,
    duration: i * 10,
    status: i % 2 ? 'unmatched' : 'complete',
    votes: i % 2 ? [] : [{ confidence: 50 + i }],
  }));
  const s = cycleSeries(history, 3);
  expect(s.map((p) => p.id)).toEqual(['c2', 'c3', 'c4']);
  expect(s[0]).toMatchObject({ confidence: 52, duration: 20, matched: 1 });
  expect(s[1]).toMatchObject({ confidence: null, matched: 0 });
});

test('atpNow applies the kernel decay to the stored state', () => {
  const atp = { valence: 0.5, intensity: 0.4, lastUpdate: 1_000_000 };
  expect(atpNow(atp, 1_000_000)).toEqual({ valence: 0.5, intensity: 0.4 });
  const later = atpNow(atp, 1_000_000 + 180_000);
  expect(later.valence).toBeCloseTo(0.5 * Math.exp(-1), 6);
  expect(later.intensity).toBeCloseTo(0.4 * Math.exp(-1), 6);
  // Like the kernel, a never-updated state (lastUpdate 0) has fully decayed.
  const fresh = atpNow({ ...atp, lastUpdate: 0 }, 1_767_225_600_000);
  expect(fresh.valence).toBe(0);
  expect(fresh.intensity).toBe(0);
});
