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
      outcome: 'running',
      runInput: 'What is 2 + 2?',
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
  for (const caption of ['Votes by node', 'Recent cycles']) {
    expect(out).toContain(`<caption class="sr-only">${caption}</caption>`);
    // Each disclosure names its own table.
    expect(out).toContain(`Show data: ${caption}`);
  }
  expect(out).toContain('Calculate');
  expect(out).toContain('90');
  expect(out).toContain('What is 2 + 2?');
});

test('ATP meters show the stored state the next cycle uses, and the decay', () => {
  const out = render({
    atp: { valence: -0.5, intensity: 0.8, lastUpdate: Date.now() - 3_600_000 },
  });
  expect(out).toMatch(/<meter[^>]*aria-label="Valence"[^>]*value="-0.5"/);
  expect(out).toMatch(/<meter[^>]*aria-label="Intensity"[^>]*value="0.8"/);
  expect(out).toContain('modulates the next cycle');
  expect(out).toContain('Decayed now');
});

test('valence is drawn as a signed bar from the centre', () => {
  const out = render({
    atp: { valence: -0.5, intensity: 0.25, lastUpdate: Date.now() },
  });
  expect(out).toMatch(/data-signed="true"[^>]*style="left:25%;width:25%"/);
});

test('charts are named images, not unnamed application tab stops', () => {
  const out = render({ busy: false, outcome: 'complete' });
  expect(out).toContain('role="img" aria-label="Vote confidence by node"');
  expect(out).toContain(
    'role="img" aria-label="Confidence and duration of recent cycles"',
  );
});

test('a cancelled run is labelled as cancelled, not as the last cycle', () => {
  const out = render({
    busy: false,
    outcome: 'cancelled',
    runInput: 'second prompt',
  });
  expect(out).toContain('Cancelled cycle');
  expect(out).toContain('second prompt');
  expect(out).not.toContain('Last cycle');
  // Votes and stats still describe the previous completed cycle, and say so.
  expect(out).toContain('previous cycle');
  expect(render({ busy: false, outcome: 'failed' })).toContain('Failed cycle');
});

test('while running, earlier votes are labelled as the previous cycle', () => {
  const out = render();
  expect(out).toContain('Current cycle');
  expect(out).toContain('previous cycle');
});

test('an idle specimen with no cycles explains itself', () => {
  const out = render({
    trace: [],
    busy: false,
    cycle: undefined,
    history: [],
    outcome: 'idle',
  });
  expect(out).toContain('No cycle yet');
});
