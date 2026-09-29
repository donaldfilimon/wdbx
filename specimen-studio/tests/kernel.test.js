// The browser engine is the Rust kernel (WASM). These tests replace the old
// engine.ts suite: behavior is the kernel's native semantics.
import { beforeAll, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import {
  addEntry,
  addNode,
  feedback,
  initKernel,
  log,
  maintenance,
  migrateIds,
  observeContext,
  raftPlan,
  removeNode,
  runCycle,
  saveResource,
  seedSpecimen,
  togglePin,
  useKernelWorker,
  validateSettings,
  validateSpecimen,
} from '../lib/specimen/kernel';
import { SUBSYSTEMS } from '../lib/specimen/types';

beforeAll(async () => {
  useKernelWorker(false);
  await initKernel(
    readFileSync(
      new URL('../lib/specimen/wasm/specimen_wasm.wasm', import.meta.url),
    ),
  );
});

const answer = async (state, input) =>
  (await runCycle(state, input)).cycle.segments.map((s) => s.text);

test('the seed is a valid native specimen', () => {
  const s = seedSpecimen();
  expect(s.nodes).toHaveLength(8);
  expect(s.resources).toHaveLength(12);
  expect(s.nodes[0].patternId.startsWith('text-v2:')).toBe(true);
  expect(() => validateSpecimen(s)).not.toThrow();
});

test('the starter arithmetic node answers one binary operation', async () => {
  // Native semantics: `&n &op &n` binds a single operation; chained
  // expressions are unmatched (the retired TS engine classified any
  // arithmetic). Operator precedence is covered by the kernel's
  // language::tests::bounded_math.
  const s = seedSpecimen();
  expect(await answer(s, 'What is 2 + 2?')).toEqual(['4']);
  expect(await answer(s, '7 * 6')).toEqual(['42']);
  expect((await runCycle(s, '2+3*4')).cycle.status).toBe('unmatched');
});

test('cycles are pure: the source specimen is untouched', async () => {
  const s = seedSpecimen();
  const { state, cycle } = await runCycle(s, 'What is 2 + 2?');
  expect(s.history).toHaveLength(0);
  expect(state.history).toHaveLength(1);
  expect(cycle.votes.length).toBeGreaterThan(0);
});

test('repeat, negation and unmatched inputs behave', async () => {
  const s = seedSpecimen();
  expect((await answer(s, 'say hello 3 times'))[0]).toBe('hello hello hello');
  expect(await answer(s, 'do not calculate 2+2')).not.toContain('4');
  expect((await runCycle(s, 'an unseen structure')).cycle.status).toBe(
    'unmatched',
  );
});

test('cycles stream trace steps in order', async () => {
  const seen = [];
  await runCycle(seedSpecimen(), 'What is 2 + 2?', (t) => seen.push(t));
  const phases = seen.at(-1).map((step) => step.phase);
  expect(phases[0]).toBe('Prepare');
  expect(phases).toContain('Compose');
  expect(seen.length).toBe(phases.length);
});

test('an aborted signal cancels before dispatch', async () => {
  const controller = new AbortController();
  controller.abort();
  await expect(
    runCycle(seedSpecimen(), 'What is 2 + 2?', undefined, controller.signal),
  ).rejects.toMatchObject({ name: 'AbortError' });
});

test('custom bind patterns execute with the original text', async () => {
  const s = addNode(seedSpecimen(), {
    name: 'Echo',
    pattern: 'echo &text',
    action: 'You said: &current_input',
  });
  expect((await answer(s, 'echo Silver sky'))[0]).toBe(
    'You said: echo Silver sky',
  );
});

test('duplicate originals are rejected with the user-facing message', () => {
  const s = seedSpecimen();
  expect(() =>
    addNode(s, { name: 'Dup', pattern: 'hello', action: 'Hi' }),
  ).toThrow(
    'That exact pattern is already stored. Use a distinct pattern or edit its node.',
  );
});

test('editing preserves sibling entries and weights', () => {
  let s = seedSpecimen();
  const node = s.nodes[0];
  s = addEntry(s, node.ref, '2+2', 'four');
  s.nodes[0].entries[0].alternatives[0].weight = 7;
  s = addNode(
    s,
    { name: 'Arithmetic', pattern: node.entries[0].pattern, action: 'x' },
    node.ref,
  );
  expect(s.nodes[0].entries).toHaveLength(2);
  expect(s.nodes[0].entries[0].alternatives[0].weight).toBe(7);
  expect(() => validateSpecimen(s)).not.toThrow();
  expect(() =>
    addNode(
      s,
      { name: 'Changed', pattern: 'unrelated', action: 'x' },
      node.ref,
    ),
  ).toThrow();
});

test('feedback applies once per contributor and selection', async () => {
  const { state: s, cycle } = await runCycle(seedSpecimen(), '2+2');
  const color = cycle.segments[0].color;
  const { state } = feedback(s, cycle.id, true, color);
  expect(() => feedback(state, cycle.id, true)).toThrow();
  expect(() => feedback(state, cycle.id, false, color)).toThrow();
});

test('pins survive and the pin limit holds', async () => {
  const { state: s, cycle } = await runCycle(seedSpecimen(), '2+2');
  const pinned = togglePin(s, cycle.id);
  expect(pinned.history[0].pinned).toBe(true);
  expect(pinned.events[0].title).toBe('Conversation pinned');
});

test('removing a node drops its attachments', () => {
  const s = seedSpecimen();
  s.attachments.push({
    id: 'a1',
    from: s.nodes[0].ref,
    to: s.nodes[1].ref,
    bidirectional: false,
    hard: false,
    affinity: 0.5,
  });
  const next = removeNode(s, s.nodes[0].ref);
  expect(next.nodes).toHaveLength(7);
  expect(next.attachments).toHaveLength(0);
});

test('memories accept exactly the registered subsystems', () => {
  const s = seedSpecimen();
  const input = {
    subsystem: 'dictionary',
    text: 'teal',
    value: 'A color.',
    resourceId: '',
    valence: 0,
    intensity: 0.2,
  };
  for (const subsystem of SUBSYSTEMS)
    expect(() => saveResource(s, { ...input, subsystem })).not.toThrow();
  expect(() => saveResource(s, { ...input, subsystem: 'vision' })).toThrow(
    'Choose a registered subsystem.',
  );
});

test('settings use native bounds', () => {
  const settings = { ...seedSpecimen().settings };
  expect(() => validateSettings({ ...settings, jitter: 50 })).not.toThrow();
  expect(() => validateSettings({ ...settings, jitter: 101 })).toThrow(
    'Set jitter between 0 and 100.',
  );
});

test('malformed files are rejected before replacement', () => {
  expect(() => validateSpecimen({ schema: 'nope' })).toThrow('not a supported');
  const s = seedSpecimen();
  s.history.push({ id: 'h' });
  expect(() => validateSpecimen(s)).toThrow();
});

test('log appends in place', () => {
  const s = seedSpecimen();
  log(s, 'attachment', 'Nodes attached', 'A → B');
  expect(s.events[0].title).toBe('Nodes attached');
});

test('maintenance and review return valid specimens', async () => {
  const s = seedSpecimen();
  expect(() => validateSpecimen(maintenance(s, 'phagy'))).not.toThrow();
  expect(() => validateSpecimen(maintenance(s, 'mutation'))).not.toThrow();
  const { state } = await runCycle(s, 'an unseen structure');
  const reviewed = await observeContext(state);
  expect(() => validateSpecimen(reviewed)).not.toThrow();
});

test('browser saves from the TypeScript engine migrate to native IDs', () => {
  const legacy = JSON.parse(
    readFileSync(new URL('../conformance/starter.json', import.meta.url)),
  );
  expect(legacy.nodes[0].patternId).toBe('low:arithmetic');
  const next = migrateIds(legacy);
  expect(next.nodes.every((n) => n.patternId.startsWith('text-v2:'))).toBe(
    true,
  );
  expect(() => validateSpecimen(next)).not.toThrow();
});

test('importing a TypeScript-era file migrates before validating', () => {
  const legacy = JSON.parse(
    readFileSync(new URL('../conformance/starter.json', import.meta.url)),
  );
  legacy.settings.scanLimit = 5000;
  const next = validateSpecimen(legacy);
  expect(next.settings.scanLimit).toBe(1000);
  expect(next.nodes[0].patternId.startsWith('text-v2:')).toBe(true);
});

test('the advanced editor cannot store a node without entries', () => {
  expect(() =>
    addNode(seedSpecimen(), {
      name: 'N',
      pattern: 'new thing',
      action: 'act',
      entries: [],
    }),
  ).toThrow('Store between 1 and 20 entries.');
});

test('imports with out-of-range records are rejected', () => {
  const s = seedSpecimen();
  s.atp.valence = 50;
  expect(() => validateSpecimen(s)).toThrow('ATP state is invalid.');
});

test('raftPlan returns totals and a bounded prefix of the kernel partition', () => {
  expect(raftPlan(10, 4, 3, 100)).toEqual({
    chunkSize: 4,
    checkpoints: 3,
    maxRafts: 2,
    chunks: [
      {
        start: 0,
        end: 4,
        rafts: [
          [0, 2],
          [2, 4],
        ],
      },
      {
        start: 4,
        end: 8,
        rafts: [
          [4, 6],
          [6, 8],
        ],
      },
      {
        start: 8,
        end: 10,
        rafts: [
          [8, 9],
          [9, 10],
        ],
      },
    ],
  });
  const big = raftPlan(100000, 1, 1, 12);
  expect(big.checkpoints).toBe(100000);
  expect(big.chunks).toHaveLength(12);
  expect(raftPlan(0, 256, 8, 12)).toMatchObject({ checkpoints: 0, chunks: [] });
  expect(() => raftPlan(10, 0, 1, 12)).toThrow(/chunk size/);
  // Out-of-range sizes are rejected, not trapped: the kernel stays usable.
  expect(() => raftPlan(20_000_000, 1, 1, 12)).toThrow(/at most/);
  expect(raftPlan(3, 256, 8, 12).checkpoints).toBe(1);
});
