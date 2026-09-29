import { expect, test } from 'bun:test';
import sections from '../lib/specification.json';
import {
  DIAGRAMS,
  LIFECYCLE,
  PIPELINE,
  entityLinks,
  parseEntityOutline,
  lifecycleState,
  reachedPhases,
} from '../lib/specimen/spec-figures';

const figures = new Map(
  sections.flatMap((c) =>
    c.segments.filter((s) => s.kind === 'figure').map((s) => [s.id, s]),
  ),
);

test('every diagram id names a real specification figure', () => {
  for (const id of DIAGRAMS) expect(figures.has(id)).toBe(true);
});

test('prose segments carry no fenced code; figures carry the source', () => {
  for (const c of sections)
    for (const s of c.segments)
      if (s.kind === 'html') expect(s.html).not.toContain('<pre>');
  expect(figures.get('ch11-1').code).toContain('run_cycle(input):');
  expect(figures.size).toBe(29);
});

test('parseEntityOutline reads names, types, notes, optional and list fields', () => {
  const model = parseEntityOutline(`Thing
  record_ref                  # unique internal reference
  kind: a | b
  items: Part[]
  maybe?                      # optional

Part
  slot_ref
`);
  expect(model.map((e) => e.name)).toEqual(['Thing', 'Part']);
  expect(model[0].fields).toEqual([
    {
      name: 'record_ref',
      note: 'unique internal reference',
      optional: false,
      many: false,
    },
    { name: 'kind', type: 'a | b', optional: false, many: false },
    { name: 'items', type: 'Part[]', optional: false, many: false },
    { name: 'maybe', note: 'optional', optional: true, many: false },
  ]);
  expect(parseEntityOutline('Name\n  refs[]\n')[0].fields[0]).toMatchObject({
    name: 'refs',
    many: true,
  });
});

test('entityLinks finds known entity names inside a type', () => {
  const known = new Set(['PatternEntry', 'Resource', 'VoteDefinition']);
  expect(entityLinks('PatternEntry[]', known)).toEqual(['PatternEntry']);
  expect(
    entityLinks('Map<SubsystemName, HybridTable<Resource>>', known),
  ).toEqual(['Resource']);
  expect(entityLinks('HybridTable<Node>', known)).toEqual([]);
  expect(entityLinks(undefined, known)).toEqual([]);
});

test('the ch5 model links resolve to entities defined in the model figures', () => {
  const model = ['ch5-1', 'ch5-4', 'ch5-5'].flatMap((id) =>
    parseEntityOutline(figures.get(id).code),
  );
  const known = new Set(model.map((e) => e.name));
  expect(known.has('PatternNode')).toBe(true);
  expect(known.has('Resource')).toBe(true);
  const links = model.flatMap((e) =>
    e.fields.flatMap((f) => entityLinks(f.type, known)),
  );
  expect(links).toContain('PatternEntry');
  expect(links).toContain('VoteAlternative');
});

test('pipeline stages link to real chapters', () => {
  const numbers = new Set(sections.map((s) => s.number));
  for (const stage of PIPELINE) expect(numbers.has(stage.chapter)).toBe(true);
  expect(PIPELINE[0].label).toBe('User input / permitted internal stimulus');
});

test('reachedPhases reports which lifecycle phases a trace reached', () => {
  expect(LIFECYCLE.map((p) => p.phase)).toEqual([
    'Prepare',
    'Retrieve',
    'Index Rafts',
    'Deep scan',
    'Vote',
    'Compose',
  ]);
  const trace = [
    { phase: 'Prepare' },
    { phase: 'Retrieve' },
    { phase: 'Index Rafts' },
    { phase: 'Index Rafts' },
  ];
  expect(reachedPhases(trace, true)).toEqual({
    done: ['Prepare', 'Retrieve'],
    active: 'Index Rafts',
  });
  expect(reachedPhases(trace, false)).toEqual({
    done: ['Prepare', 'Retrieve', 'Index Rafts'],
    active: undefined,
  });
  expect(reachedPhases([], false)).toEqual({ done: [], active: undefined });
  // Non-lifecycle steps (Processing, Failed) are ignored.
  expect(reachedPhases([{ phase: 'Failed' }], false).done).toEqual([]);
});

test('lifecycleState follows the run outcome, not the busy flag', () => {
  const trace = [
    { phase: 'Prepare' },
    { phase: 'Retrieve' },
    { phase: 'Index Rafts' },
  ];
  expect(lifecycleState(trace, 'running')).toEqual({
    done: ['Prepare', 'Retrieve'],
    active: 'Index Rafts',
    stopped: undefined,
    status: 'Running: Index Rafts',
  });
  // A cancelled or failed run did not complete the phase it stopped in.
  expect(lifecycleState(trace, 'cancelled')).toEqual({
    done: ['Prepare', 'Retrieve'],
    active: undefined,
    stopped: 'Index Rafts',
    status: 'Cancelled during Index Rafts',
  });
  expect(lifecycleState(trace, 'failed').status).toBe(
    'Failed during Index Rafts',
  );
  expect(lifecycleState([], 'failed').status).toBe(
    'The last run failed before Prepare.',
  );
  const full = [
    ...trace,
    { phase: 'Deep scan' },
    { phase: 'Vote' },
    { phase: 'Compose' },
  ];
  expect(lifecycleState(full, 'complete').status).toBe(
    'Last cycle reached Compose',
  );
  expect(lifecycleState(full, 'idle').done).toHaveLength(6);
  expect(lifecycleState([], 'idle').status).toMatch(/^No cycle yet/);
});
