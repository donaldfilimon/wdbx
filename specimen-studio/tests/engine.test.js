import { test, expect } from 'bun:test';
import {
  seedSpecimen,
  runCycle,
  addNode,
  addEntry,
  feedback,
  validateSpecimen,
  calculate,
  runTape,
  identify,
  confidence,
  raftFindAll,
  togglePin,
  removeNode,
  observeContext,
  saveResource,
  maintenance,
  deepScore,
  fanouts,
} from '../lib/specimen/engine';
import { uid } from '../lib/specimen/types';

test('UID generation survives browser runtimes without Web Crypto', () => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'crypto');
  Object.defineProperty(globalThis, 'crypto', {
    value: undefined,
    configurable: true,
  });
  try {
    const id = uid();
    expect(id).toMatch(/^[a-z0-9-]+$/i);
    expect(id.length).toBeGreaterThan(20);
  } finally {
    if (descriptor) Object.defineProperty(globalThis, 'crypto', descriptor);
    else delete globalThis.crypto;
  }
});

test('arithmetic parser enforces precedence and rejects unsafe or undefined input', () => {
  expect(calculate('What is 2 + 2?')).toBe(4);
  expect(calculate('2+3*4')).toBe(14);
  expect(calculate('(2+3)*4')).toBe(20);
  expect(calculate('2^3^2')).toBe(512);
  for (const s of ['1/0', '1+', 'globalThis.alert(1)', '1e999', '(1+2'])
    expect(() => calculate(s)).toThrow();
});
test('patterns are reusable, deduplicated, and gate action execution', async () => {
  const seed = seedSpecimen();
  const { state, cycle } = await runCycle(seed, 'What is 2 + 2?');
  expect(cycle.segments.map((s) => s.text)).toEqual(['4']);
  expect(cycle.votes.length).toBe(1);
  expect(seed.history.length).toBe(0);
  expect(state.history.length).toBe(1);
  expect(
    (await runCycle(seed, 'say hello 3 times')).cycle.segments[0].text,
  ).toBe('hello hello hello');
  expect(
    (await runCycle(seed, 'do not calculate 2+2')).cycle.segments.some(
      (s) => s.text === '4',
    ),
  ).toBe(false);
  expect((await runCycle(seed, 'an unseen structure')).cycle.status).toBe(
    'unmatched',
  );
});
test('custom bind patterns execute and preserve original user text', async () => {
  const s = addNode(seedSpecimen(), {
    name: 'Echo',
    pattern: 'echo &text',
    action: 'You said: &current_input',
  });
  expect((await runCycle(s, 'echo Silver sky')).cycle.segments[0].text).toBe(
    'You said: echo Silver sky',
  );
});
test('same Pattern-ID peers remain distinct and duplicate originals are rejected', () => {
  const s = addNode(seedSpecimen(), {
    name: 'Second calculation',
    pattern: '2+2',
    action: 'four',
  });
  expect(
    s.nodes.filter((n) => n.patternId === identify('2+2').id),
  ).toHaveLength(2);
  expect(() =>
    addNode(s, { name: 'Duplicate', pattern: '2+2', action: 'x' }),
  ).toThrow();
});
test('editing preserves sibling entries and vote weights', () => {
  let s = seedSpecimen();
  const node = s.nodes[0];
  s = addEntry(s, node.ref, '2+2', 'four');
  s.nodes[0].entries[0].alternatives[0].weight = 7;
  s = addNode(
    s,
    {
      name: 'Arithmetic',
      pattern: node.entries[0].pattern,
      action: '&calc(&current_input)',
    },
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
test('feedback is attributed once per contributing node across overlapping selections', async () => {
  const result = await runCycle(seedSpecimen(), '2+2');
  let { state } = result;
  const { cycle } = result;
  state = feedback(state, cycle.id, true, 'green').state;
  expect(() => feedback(state, cycle.id, true)).toThrow();
  expect(() => feedback(state, cycle.id, false, 'green')).toThrow();
});
test('confidence has stable endpoints and crystallized qualified values', () => {
  const s = seedSpecimen(),
    n = s.nodes[0];
  for (const base of [-100, 100])
    expect(confidence(base, n, s.settings, () => 0.8)).toBe(base);
  n.strength = s.settings.maxStrength;
  expect(confidence(80, n, s.settings, () => 0.8)).toBe(80);
  expect(deepScore('cat', 'dog')).toBe(-100);
});
test('raft chunks cover each record exactly once with stable order and cancellation', async () => {
  const s = seedSpecimen();
  Object.assign(s.settings, { chunkSize: 7, maxRafts: 4, raftThreshold: 5 });
  const hits = new Map();
  const xs = Array.from({ length: 31 }, (_, i) => i);
  expect(
    await raftFindAll(
      xs,
      (x) => {
        hits.set(x, (hits.get(x) ?? 0) + 1);
        return x % 3 === 0;
      },
      s.settings,
    ),
  ).toEqual(xs.filter((x) => x % 3 === 0));
  expect([...hits.values()].every((x) => x === 1)).toBe(true);
  const c = new AbortController();
  c.abort();
  await expect(
    raftFindAll(xs, () => true, s.settings, c.signal),
  ).rejects.toThrow();
  await expect(runCycle(s, '2+2', undefined, c.signal)).rejects.toThrow();
});
test('pins survive rolling history and original source remains unchanged', async () => {
  let s = seedSpecimen();
  s.settings.historyLimit = 1;
  const r = await runCycle(s, 'hello');
  s = togglePin(r.state, r.cycle.id);
  s = (await runCycle(s, '2+2')).state;
  s = (await runCycle(s, '3+3')).state;
  expect(s.history).toHaveLength(2);
  expect(s.history.find((h) => h.pinned).input).toBe('hello');
  expect(() => validateSpecimen(s)).not.toThrow();
});
test('complete saves round-trip and malformed nested records fail before replacement', async () => {
  const s = (await runCycle(seedSpecimen(), 'imagine a garden')).state;
  expect(JSON.stringify(validateSpecimen(JSON.parse(JSON.stringify(s))))).toBe(
    JSON.stringify(s),
  );
  const changes = [
    (x) => (x.history[0].createdAt = 'bad'),
    (x) => (x.events = [{}]),
    (x) => (x.nodes[0].entries[0].alternatives[0].weight = NaN),
    (x) => (x.history[0].visual.size = null),
    (x) => (x.settings.chunkSize = 0.5),
    (x) => (x.history[0].segments[0].contributors = 42),
    (x) =>
      (x.proposals = [
        { id: 'p', pattern: 'p', status: 'pending', evidence: 0 },
      ]),
  ];
  for (const change of changes) {
    const broken = structuredClone(s);
    change(broken);
    expect(() => validateSpecimen(broken)).toThrow();
  }
  expect(s.history[0].segments.length).toBe(1);
});
test('resource deletion detaches only the last ID peer', () => {
  let s = seedSpecimen();
  const first = s.nodes[4];
  s = addNode(s, {
    name: 'Other recall',
    pattern: 'remember things',
    action: 'remember',
  });
  s = removeNode(s, first.ref);
  expect(s.resources[0].resourceId).toBe(first.patternId);
  s = removeNode(s, s.nodes.find((n) => n.name === 'Other recall').ref);
  expect(s.resources[0].resourceId).toBe('');
  expect(() =>
    saveResource(s, {
      subsystem: 'chargebook',
      text: 'x',
      value: 'y',
      resourceId: '',
      valence: 2,
      intensity: 0,
    }),
  ).toThrow();
});
test('Type A residual review and Type B idle correlation propose explicit learning', async () => {
  let s = addNode(seedSpecimen(), {
    name: 'A review',
    type: 'A',
    pattern: 'unknown',
    action: 'review',
  });
  s = (await runCycle(s, 'unknown')).state;
  s = await observeContext(s);
  expect(s.proposals).toHaveLength(1);
  expect(s.history[0].reviewed).toBe(true);
  s = addNode(seedSpecimen(), {
    name: 'B observer',
    type: 'B',
    pattern: 'observer',
    action: 'review',
    contextId: 'new',
  });
  s = (await runCycle(s, 'new question')).state;
  s = (await runCycle(s, 'new question')).state;
  s = await observeContext(s);
  expect(s.proposals[0].evidence).toBe(2);
  expect(s.nodes.some((n) => n.entries[0].pattern === 'new question')).toBe(
    false,
  );
});
test('automata halt at bounded budget and PHAGY preserves mutation history', () => {
  expect(runTape('++++++++[>++++++++<-]>+.')).toBe('A');
  expect(() => runTape('+[]', '', 20)).toThrow();
  expect(() => runTape('[')).toThrow();
  const s = seedSpecimen();
  const cleaned = maintenance(s, 'phagy');
  expect(cleaned.nodes).toEqual(s.nodes);
  expect(cleaned.mutations).toEqual(s.mutations);
});

test('coarse ID collisions are independently scored', () => {
  expect(identify('2+2').id).toBe(identify('9+9').id);
  expect(deepScore('2+2', '9+9')).toBeLessThan(62);
});

test('lexical negation suppresses only its semicolon-delimited clause', async () => {
  for (const prompt of [
    "don't calculate 2+2; hello",
    'don’t calculate 2+2; hello',
    "hello; don't calculate 2+2",
  ]) {
    const { cycle } = await runCycle(seedSpecimen(), prompt);
    expect(cycle.segments.some((s) => s.text.startsWith('Hello'))).toBe(true);
    expect(cycle.segments.some((s) => s.text === '4')).toBe(false);
    expect(cycle.status).toBe('complete');
    const scope = cycle.trace.find((step) => step.phase === 'Scope');
    expect(scope.count).toBe(1);
    expect(scope.detail).toContain('semicolon');
    expect(scope.detail).toContain('independent');
  }
});

test('thesaurus rewrites cannot erase the originating prohibition', async () => {
  const seed = saveResource(seedSpecimen(), {
    subsystem: 'thesaurus',
    text: "don't calculate",
    value: 'calculate',
    valence: 0,
    intensity: 0,
  });
  const prompt = "don't calculate 2+2; hello";
  expect(fanouts(prompt, seed)).toContain('calculate 2+2; hello');
  const { cycle } = await runCycle(seed, prompt);
  expect(cycle.segments.some((s) => s.text === '4')).toBe(false);
  expect(cycle.segments.some((s) => s.text.startsWith('Hello'))).toBe(true);
});

test('cross-clause rewrites conservatively inhibit ambiguous generated scopes', async () => {
  const seed = saveResource(seedSpecimen(), {
    subsystem: 'thesaurus',
    text: "don't calculate 2+2; hello",
    value: 'hello; calculate 2+2',
    valence: 0,
    intensity: 0,
  });
  const { cycle } = await runCycle(seed, "don't calculate 2+2; hello");
  expect(cycle.segments.some((s) => s.text === '4')).toBe(false);
  expect(cycle.segments.some((s) => s.text.startsWith('Hello'))).toBe(true);
  expect(
    cycle.trace.some(
      (s) => s.phase === 'Scope' && s.detail.includes('ambiguous'),
    ),
  ).toBe(true);
});

test('cyclic thesaurus variants cannot retroactively inhibit the original clauses', async () => {
  for (const [prompt, term, replacement, arithmetic] of [
    ['hello; 2+2', 'hello', 'never hello', true],
    ['do not calculate 2+2; hello', 'hello', 'never hello', false],
    [
      'do not calculate 2+2; hello',
      'do not calculate 2+2; hello',
      'hello; 2+2',
      false,
    ],
  ]) {
    let seed = seedSpecimen();
    seed.resources = seed.resources.filter(
      (resource) => resource.subsystem !== 'thesaurus',
    );
    for (const [text, value] of [
      [term, replacement],
      [replacement, term],
    ]) {
      seed = saveResource(seed, {
        subsystem: 'thesaurus',
        text,
        value,
        valence: 0,
        intensity: 0,
      });
    }
    const { cycle } = await runCycle(seed, prompt);
    expect(
      cycle.segments.some((segment) => segment.text.startsWith('Hello')),
    ).toBe(true);
    expect(cycle.segments.some((segment) => segment.text === '4')).toBe(
      arithmetic,
    );
    expect(cycle.status).toBe('complete');
    expect(fanouts(prompt, seed).length).toBeLessThanOrEqual(
      seed.settings.fanoutLimit,
    );
  }
});

test('mixed clause matrix retains active votes and contributor provenance', async () => {
  for (const [prompt, numbers, greeting, status] of [
    ['hello; do not calculate 2+2; 3+3', ['6'], true, 'complete'],
    ['never calculate 2+2; hello; do not calculate 3+3', [], true, 'complete'],
    ['never calculate 2+2; do not calculate 3+3', [], false, 'inhibited'],
    ['do not calculate 2+2; xyzzy unmatched', [], false, 'unmatched'],
    ['never hello then 2+2; 3+3', ['6'], false, 'complete'],
    ["don't hello and then 2+2; 3+3", ['6'], false, 'complete'],
    ["don't calculate 2+2 then hello; hello", [], true, 'complete'],
    ["hello; don't calculate 2+2 then hello", [], true, 'complete'],
    ['; ;don’t calculate 2+2;;hello;', [], true, 'complete'],
  ]) {
    const seed = seedSpecimen();
    const { cycle } = await runCycle(seed, prompt);
    expect(
      cycle.segments.filter((s) => /^\d+$/.test(s.text)).map((s) => s.text),
    ).toEqual(numbers);
    expect(cycle.segments.some((s) => s.text.startsWith('Hello'))).toBe(
      greeting,
    );
    expect(cycle.status).toBe(status);
    for (const segment of cycle.segments) {
      const votes = cycle.votes.filter((vote) =>
        segment.sourceVotes.includes(vote.id),
      );
      expect(segment.contributors).toEqual(votes.map((vote) => vote.nodeRef));
      expect(
        votes.every((vote) =>
          seed.nodes.some((node) => node.ref === vote.nodeRef),
        ),
      ).toBe(true);
    }
  }
});

test('negation acknowledgements cannot delegate uninhibited attachment actions', async () => {
  const seed = addNode(seedSpecimen(), {
    name: 'Attached greeting',
    pattern: '&hello*',
    action: 'escaped attachment',
  });
  const negation = seed.nodes.find((node) => node.name === 'Negation');
  const target = seed.nodes.find((node) => node.name === 'Attached greeting');
  seed.attachments.push({
    id: uid(),
    from: negation.ref,
    to: target.ref,
    hard: true,
    bidirectional: false,
    affinity: 1,
  });
  const { cycle } = await runCycle(seed, "don't hello");
  expect(cycle.votes.some((vote) => vote.nodeRef === negation.ref)).toBe(true);
  expect(cycle.votes.some((vote) => vote.nodeRef === target.ref)).toBe(false);
  expect(cycle.segments.map((segment) => segment.text)).toEqual([
    'I will leave that action alone.',
  ]);
  expect(cycle.status).toBe('inhibited');
});

test('positive fan-outs retain their global budget and ordinary trace', async () => {
  const seed = seedSpecimen();
  seed.settings.fanoutLimit = 3;
  expect(fanouts('hello; hello; hello', seed)).toHaveLength(3);
  const { cycle } = await runCycle(seed, 'hello; hello; hello');
  expect(cycle.trace.some((step) => step.phase === 'Scope')).toBe(false);
  expect(cycle.trace.find((step) => step.phase === 'Prepare').count).toBe(9);
  expect(
    cycle.segments.some((segment) => segment.text.startsWith('Hello')),
  ).toBe(true);
});

test('unmatched inhibited scopes do not claim candidate suppression', async () => {
  const seed = seedSpecimen();
  seed.nodes = [];
  const { cycle } = await runCycle(seed, 'never xyzzy; hello');
  expect(cycle.votes).toHaveLength(0);
  expect(
    cycle.trace.find((step) => step.phase === 'Scope').detail,
  ).not.toContain('suppressed matches');
});

test('lexical negation remains conservative without a semicolon boundary', async () => {
  const { cycle } = await runCycle(
    seedSpecimen(),
    "don't calculate 2+2 then hello",
  );
  expect(cycle.segments.some((s) => s.text.startsWith('Hello'))).toBe(false);
  expect(cycle.segments.some((s) => s.text === '4')).toBe(false);
  expect(cycle.status).toBe('inhibited');
  const scope = cycle.trace.find((step) => step.phase === 'Scope');
  expect(scope.count).toBe(1);
  expect(scope.detail).toContain('full prompt');
});
