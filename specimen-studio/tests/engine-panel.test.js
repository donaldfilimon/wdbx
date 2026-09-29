import { expect, test } from 'bun:test';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { EnginePanel } from '../app/panels/engine-panel';

const step = (phase, extra = {}) => ({
  id: phase,
  phase,
  detail: `${phase} detail`,
  ...extra,
});
const nodes = [
  { ref: 'a', name: 'Calculate' },
  { ref: 'b', name: 'Greeting' },
];
const cycle = {
  id: 'c1',
  input: 'What is 2 + 2?',
  createdAt: '2026-01-01T00:00:00.000Z',
  duration: 12,
  status: 'complete',
  votes: [
    { nodeRef: 'a', confidence: 90, strength: 6 },
    { nodeRef: 'b', confidence: 40, strength: 5 },
  ],
};
const render = (props = {}) =>
  renderToStaticMarkup(
    h(EnginePanel, {
      trace: [
        step('Prepare', { count: 1 }),
        step('Index Rafts', { covered: 1, total: 2 }),
      ],
      busy: true,
      cycle,
      history: [cycle],
      nodes,
      atp: { valence: 0.2, intensity: 0.25, lastUpdate: Date.now() },
      threshold: 62,
      ...props,
    }),
  );

test('the live trace marks the running phase and exposes raft coverage', () => {
  const out = render();
  expect(out).toContain('aria-label="Live cycle trace"');
  expect(out).toContain('aria-current="step"');
  expect(out).toMatch(
    /<progress[^>]*aria-label="Index Rafts coverage"[^>]*value="50"/,
  );
});

test('every chart has a data table beside it', () => {
  const out = render();
  for (const caption of ['Votes by node', 'Recent cycles'])
    expect(out).toContain(`<caption`);
  expect(out).toContain('Calculate');
  expect(out).toContain('90');
  expect(out).toContain('What is 2 + 2?');
});

test('ATP meters report the decayed state', () => {
  const out = render();
  expect(out).toMatch(/<meter[^>]*aria-label="Valence"/);
  expect(out).toMatch(/<meter[^>]*aria-label="Intensity"/);
});

test('an idle specimen with no cycles explains itself', () => {
  const out = render({ trace: [], busy: false, cycle: undefined, history: [] });
  expect(out).toContain('No cycle yet');
});
