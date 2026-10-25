// Authored synthetic inputs; independent exact arithmetic and transition checks.
// No substitute product engine is used. All modes share validate().
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const eq = (a, b, m) => assert.deepEqual(a, b, m);
const keys = (o, ks) => {
  assert(o && typeof o === 'object' && !Array.isArray(o));
  eq(Object.keys(o).sort(), ks.split(' ').sort(), 'strict named trace fields');
};
const integer = (s) => {
  assert(
    typeof s === 'string' && /^(0|-?[1-9][0-9]*)$/.test(s),
    'canonical decimal string',
  );
  return BigInt(s);
};
const id = (s) => {
  const n = integer(s);
  assert(n >= 0n && n < 1n << 128n);
  return s;
};
const u64 = (s) => {
  const n = integer(s);
  assert(n >= 0n && n < 1n << 64n);
  return s;
};
const gcd = (a, b) => (b ? gcd(b, a % b) : a < 0n ? -a : a);
const rat = (n, d = 1n) => {
  assert(d > 0n);
  const g = gcd(n, d);
  return [n / g, d / g];
};
const read = (r) => {
  keys(r, 'n d');
  const n = integer(r.n),
    d = integer(r.d);
  assert(n >= -(1n << 127n) && n < 1n << 127n && d > 0n && d < 1n << 127n);
  eq([n, d], rat(n, d), 'reduced bounded rational');
  return [n, d];
};
const add = (a, b) => rat(a[0] * b[1] + b[0] * a[1], a[1] * b[1]);
const sub = (a, b) => add(a, [-b[0], b[1]]);
const mul = (a, b) => rat(a[0] * b[0], a[1] * b[1]);
const cmp = (a, b) => a[0] * b[1] - b[0] * a[1];
const clamp = (a) =>
  cmp(a, [-100n, 1n]) < 0n
    ? [-100n, 1n]
    : cmp(a, [100n, 1n]) > 0n
      ? [100n, 1n]
      : a;
const fraction = (a) => ({ n: a[0].toString(), d: a[1].toString() });
const MAX = 4294967295n;
const checkDraw = (d, ctx, index, purpose) => {
  keys(d, 'seed stream index value purpose');
  u64(d.seed);
  eq(d.seed, ctx.seed);
  eq(d.stream, ctx.stream);
  eq(d.index, index);
  eq(d.value, ctx.draws[index]);
  assert(Number.isInteger(d.value) && d.value >= 0 && d.value <= Number(MAX));
  eq(d.purpose, purpose);
};
function numericExpected(scan, p, ctx) {
  assert(
    cmp(scan, [-100n, 1n]) >= 0n && cmp(scan, [100n, 1n]) <= 0n,
    'numeric scan profile bound',
  );
  u64(ctx.seed);
  assert(
    typeof ctx.stream === 'string' &&
      ctx.stream.length > 0 &&
      Buffer.byteLength(ctx.stream) <= 128,
  );
  assert(
    Array.isArray(ctx.draws) &&
      ctx.draws.every((d) => Number.isInteger(d) && d >= 0 && d <= Number(MAX)),
    'full supplied u32 draw grid',
  );
  keys(p, 'strength threshold jitter cap valence intensity');
  assert(Number.isInteger(p.strength) && p.strength >= 0 && p.strength <= 100);
  assert(
    Number.isInteger(p.threshold) && p.threshold >= -100 && p.threshold <= 100,
  );
  assert(typeof p.jitter === 'boolean');
  const v = read(p.valence),
    a = read(p.intensity),
    cap = read(p.cap);
  for (const r of [v, a, cap]) assert(r[1] <= 1000000n);
  assert(
    cmp(v, [-1n, 1n]) >= 0n &&
      cmp(v, [1n, 1n]) <= 0n &&
      a[0] >= 0n &&
      cmp(a, [1n, 1n]) <= 0n &&
      cap[0] >= 0n &&
      cmp(cap, [5n, 1n]) <= 0n,
  );
  const base = clamp(
    sub(scan, mul([10n, 1n], mul(a, v[0] < 0n ? [-v[0], v[1]] : [0n, 1n]))),
  );
  const bq = cmp(base, [BigInt(p.threshold), 1n]) >= 0n;
  const reason = !p.jitter
    ? 'Disabled'
    : base[1] === 1n && (base[0] === 100n || base[0] === -100n)
      ? 'Endpoint'
      : p.strength === 100 && bq
        ? 'MaximumQualified'
        : cap[0] === 0n
          ? 'ZeroCap'
          : 'Eligible';
  const delta =
    reason === 'Eligible'
      ? mul(cap, rat(BigInt(ctx.draws[0]) * (ctx.draws[1] & 1 ? 1n : -1n), MAX))
      : [0n, 1n];
  const effective = clamp(add(base, delta));
  const refusals = [];
  if (cmp(effective, [BigInt(p.threshold), 1n]) < 0n)
    refusals.push('BelowThreshold');
  if (p.strength === 0) refusals.push('ZeroStrength');
  return {
    scan: fraction(scan),
    baseline: fraction(base),
    delta: fraction(delta),
    effective: fraction(effective),
    baseline_qualified: bq,
    qualified: refusals.length === 0,
    reason,
    refusals,
  };
}
function checkScore(s, scan, p, ctx, guards = []) {
  keys(
    s,
    'seed stream scan baseline delta effective baseline_qualified qualified reason refusals draws',
  );
  u64(s.seed);
  eq(s.seed, ctx.seed);
  eq(s.stream, ctx.stream);
  for (const k of ['scan', 'baseline', 'delta', 'effective']) read(s[k]);
  const expected = numericExpected(scan, p, ctx);
  if (ctx.bypass)
    expected.refusals = expected.refusals.filter((x) => x !== 'BelowThreshold');
  expected.refusals.push(...guards);
  expected.qualified = expected.refusals.length === 0;
  for (const [k, v] of Object.entries(expected))
    eq(s[k], v, `exact confidence/gate ${k}`);
  eq(
    s.draws.length,
    expected.reason === 'Eligible' ? 2 : 0,
    'exact jitter consumption',
  );
  s.draws.forEach((d, i) =>
    checkDraw(d, ctx, i, i === 0 ? 'JitterMagnitude' : 'JitterSign'),
  );
  return s.draws.length;
}
function weight(hex) {
  assert(typeof hex === 'string' && /^[0-9a-f]{16}$/.test(hex));
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt('0x' + hex));
  const w = b.readDoubleLE();
  assert(
    Number.isFinite(w) && w >= 0,
    'finite nonnegative original weight bits',
  );
  return w;
}
function checkElection(e, ctx, purpose = 'Alternative', offset = 0) {
  keys(e, 'seed stream weight_bits refused draw winner');
  u64(e.seed);
  assert(
    typeof ctx.stream === 'string' &&
      ctx.stream.length > 0 &&
      Buffer.byteLength(ctx.stream) <= 128,
  );
  assert(ctx.eligible.every((v) => typeof v === 'boolean'));
  assert(
    ctx.draws.every((d) => Number.isInteger(d) && d >= 0 && d <= Number(MAX)),
  );
  eq(e.seed, ctx.seed);
  eq(e.stream, ctx.stream);
  eq(e.weight_bits, ctx.weight_bits, 'original f64 bits');
  const ws = ctx.weight_bits.map(weight);
  eq(ws.length, ctx.eligible.length);
  const positives = ws.flatMap((w, i) => (w > 0 && ctx.eligible[i] ? [i] : []));
  eq(
    e.refused,
    ctx.eligible.flatMap((v, i) => (v ? [] : [i])),
  );
  let winner = null;
  if (positives.length === 0) {
    eq(e.draw, null);
  } else if (positives.length === 1) {
    winner = positives[0];
    eq(e.draw, null);
  } else {
    checkDraw(e.draw, ctx, offset, purpose);
    const d = ctx.draws[offset];
    if (d === 0) winner = positives[0];
    else if (d === Number(MAX)) winner = positives.at(-1);
    else {
      // All authored interior weights are small exact integers; compare intervals by
      // integer arithmetic, independently of the normalized compensated f64 code.
      assert(
        positives.every((i) => Number.isSafeInteger(ws[i])),
        'authored safe interior weight domain',
      );
      const total = positives.reduce((n, i) => n + BigInt(ws[i]), 0n);
      let acc = 0n;
      winner = positives.at(-1);
      for (const i of positives) {
        acc += BigInt(ws[i]);
        if (BigInt(d) * total < acc * MAX) {
          winner = i;
          break;
        }
      }
    }
  }
  eq(e.winner, winner, 'independent proportional election');
  return positives.length > 1 ? 1 : 0;
}
function guards(c, input) {
  keys(c, 'inhibited uncertain unresolved conditional executable');
  for (const k of ['inhibited', 'uncertain', 'unresolved', 'executable'])
    assert(typeof c[k] === 'boolean');
  assert(
    ['None', 'Satisfied', 'Unresolved', 'Refuted'].includes(c.conditional),
  );
  eq(
    c.executable,
    !c.inhibited &&
      !c.unresolved &&
      ['None', 'Satisfied'].includes(c.conditional),
  );
  const r = [];
  if (c.inhibited) r.push(input ? 'InputInhibited' : 'TargetInhibited');
  if (c.uncertain) r.push('UncertainNegation');
  if (c.unresolved || c.conditional === 'Unresolved') r.push('Unresolved');
  if (c.conditional === 'Refuted') r.push('Refuted');
  return r;
}
function checkOrigin(o) {
  keys(o, 'input clause action fanout rules');
  assert(typeof o.input === 'string' && o.input.length);
  for (const k of ['clause', 'action', 'fanout'])
    assert(Number.isSafeInteger(o[k]) && o[k] >= 0);
  assert(Array.isArray(o.rules) && o.rules.every((x) => typeof x === 'string'));
}
function spans(ss) {
  assert(Array.isArray(ss) && ss.length < 200);
  for (const s of ss) {
    assert(
      Array.isArray(s) &&
        s.length === 2 &&
        s.every(Number.isSafeInteger) &&
        s[0] >= 0 &&
        s[0] <= s[1],
    );
  }
}
function checkPrepared(p) {
  keys(p, 'original origin span token_spans constraints key');
  assert(
    typeof p.original === 'string' && Buffer.byteLength(p.original) <= 8000,
  );
  checkOrigin(p.origin);
  spans([p.span]);
  eq(p.span[1] - p.span[0], Buffer.byteLength(p.original));
  spans(p.token_spans);
  guards(p.constraints, true);
  assert(typeof p.key === 'string' && p.key.length < 8000);
}
function checkCaptures(cs) {
  assert(Array.isArray(cs) && cs.length < 20);
  assert(new Set(cs.map((c) => c.name)).size === cs.length);
  for (const c of cs) {
    keys(c, 'name span value');
    assert(typeof c.name === 'string' && c.name.length);
    spans([c.span]);
    const v = c.value;
    switch (v.tag) {
      case 'Number':
        keys(v, 'tag coefficient scale');
        integer(v.coefficient);
        assert(Number.isInteger(v.scale) && v.scale >= 0);
        break;
      case 'Text':
      case 'Operator':
        keys(v, 'tag value');
        assert(typeof v.value === 'string');
        break;
      case 'Bytes':
        keys(v, 'tag value');
        assert(
          Array.isArray(v.value) &&
            v.value.every((x) => Number.isInteger(x) && x >= 0 && x <= 255),
        );
        break;
      default:
        assert.fail('unknown typed capture');
    }
  }
}
function tree(xs, depth = 0) {
  assert(Array.isArray(xs) && xs.length < 50 && depth < 16);
  for (const x of xs) {
    if (x.kind === 'text') {
      keys(x, 'kind value');
      assert(typeof x.value === 'string' && Buffer.byteLength(x.value) < 8000);
    } else {
      keys(x, 'kind name args');
      eq(x.kind, 'call');
      assert(typeof x.name === 'string' && Array.isArray(x.args));
      for (const a of x.args) tree(a, depth + 1);
    }
  }
  return xs;
}
function checkInput(i) {
  keys(
    i,
    'id source target population node entry record_original record_key binding route bypass origin span token_spans guards captures parent inhibitions dependencies group kind policy alternatives',
  );
  id(i.id);
  id(i.node);
  id(i.entry);
  checkPrepared(i.source);
  checkPrepared(i.target);
  keys(i.population, 'tenant specimen node_type');
  eq(i.population, {
    tenant: 'parity-tenant',
    specimen: 'parity-specimen',
    node_type: 'text',
  });
  eq(i.record_original, i.target.original);
  eq(i.record_key, i.target.key);
  keys(i.binding, 'node entry origin');
  id(i.binding.node);
  id(i.binding.entry);
  checkOrigin(i.binding.origin);
  checkOrigin(i.origin);
  eq(i.origin, i.source.origin);
  spans([i.span]);
  eq(i.span, i.source.span);
  eq(i.token_spans, i.source.token_spans);
  eq(i.guards, i.source.constraints);
  guards(i.guards, true);
  checkCaptures(i.captures);
  assert(typeof i.parent === 'string' && i.parent.length);
  for (const f of i.inhibitions) {
    keys(f, 'domain code');
    assert(
      ['Input', 'Target'].includes(f.domain) && typeof f.code === 'string',
    );
  }
  i.dependencies.forEach(id);
  if (i.group !== null) id(i.group);
  assert(['Action', 'IntermediateFact'].includes(i.kind));
  assert(i.alternatives.length > 0 && i.alternatives.length < 10);
  for (const a of i.alternatives) {
    keys(a, 'action inhibited weight_bits');
    tree(a.action);
    assert(typeof a.inhibited === 'boolean');
    weight(a.weight_bits);
  }
  return i;
}
function checkEvidence(e) {
  keys(
    e,
    'version components similarity dissimilarity difference confidence left_constraints right_constraints',
  );
  eq(e.version, 'text-scanner-v3');
  eq(
    e.components.map((c) => c.name),
    ['lemma-multiset', 'ordered-bigram-multiset', 'spo', 'operands-operators'],
    'ordered scanner component names/count',
  );
  let s = [0n, 1n],
    d = [0n, 1n];
  for (const [c, w] of e.components.map((c, i) => [
    c,
    [40n, 20n, 20n, 20n][i],
  ])) {
    keys(c, 'name weight similarity dissimilarity reason left right');
    eq(integer(c.weight), w);
    const a = read(c.similarity),
      b = read(c.dissimilarity);
    assert(a[0] >= 0 && b[0] >= 0);
    eq(add(a, b), [w, 1n], 'component complementarity');
    assert(
      [
        'Match',
        'Partial',
        'Missing',
        'Mismatch',
        'NegationConflict',
        'UncertainNegation',
      ].includes(c.reason),
    );
    if (c.reason === 'Match') eq(a, [w, 1n]);
    else if (c.reason === 'Partial') assert(a[0] > 0n && b[0] > 0n);
    else eq(a, [0n, 1n]);
    spans(c.left);
    spans(c.right);
    s = add(s, a);
    d = add(d, b);
  }
  eq(read(e.similarity), s);
  eq(read(e.dissimilarity), d);
  eq(read(e.difference), sub(s, d));
  eq(read(e.confidence), clamp(sub(s, d)), 'scanner S-D clamp');
  guards(e.left_constraints, true);
  guards(e.right_constraints, false);
  return read(e.confidence);
}
function entryScore(s, i, e, stream) {
  const rs = [...guards(i.guards, true), ...guards(e.right_constraints, false)];
  if (i.route.includes('Blocked')) rs.push('BlockedAutomaton');
  return checkScore(
    s,
    checkEvidence(e),
    i.policy,
    {
      seed: '18446744073709551557',
      stream,
      draws: [],
      bypass: i.bypass === 'Declared',
    },
    rs,
  );
}
function checkContribution(c, i, score, election, evidence) {
  keys(
    c,
    'id node entry population root source_original target_origin parent inhibitions captures dependencies group kind slot strength action score election evidence',
  );
  for (const [k, v] of Object.entries({
    id: i.id,
    node: i.node,
    entry: i.entry,
    population: i.population,
    root: i.origin,
    source_original: i.source.original,
    target_origin: i.target.origin,
    parent: i.parent,
    inhibitions: i.inhibitions,
    captures: i.captures,
    dependencies: i.dependencies,
    group: i.group,
    kind: i.kind,
    strength: i.policy.strength,
  }))
    eq(c[k], v, `retained contribution ${k}`);
  eq(c.score, score);
  if (election !== undefined) eq(c.election, election);
  if (evidence !== undefined) eq(c.evidence, evidence);
  assert(c.score.qualified);
  eq(c.slot, c.election.winner);
  eq(c.action, i.alternatives[c.slot].action);
  tree(c.action);
  const ctx = {
    seed: c.election.seed,
    stream: c.election.stream,
    draws: c.election.draw ? [c.election.draw.value] : [],
    weight_bits: i.alternatives.map((a) => a.weight_bits),
    eligible: i.alternatives.map((a) => !a.inhibited),
  };
  checkElection(c.election, ctx);
}
const canonical = (c) =>
  JSON.stringify([
    c.root.input,
    c.root.clause,
    c.root.action,
    c.parent,
    c.action,
    [...c.inhibitions].sort(
      (a, b) =>
        a.domain.localeCompare(b.domain) || a.code.localeCompare(b.code),
    ),
    c.captures.map((x) => [x.name, x.value]),
    [...c.dependencies].sort((a, b) => (integer(a) < integer(b) ? -1 : 1)),
  ]);
function independentGroups(cs) {
  const refs = new Set(cs.flatMap((c) => c.dependencies));
  const groups = new Map();
  for (const c of cs) {
    if (c.group !== null || c.kind === 'IntermediateFact' || refs.has(c.id))
      continue;
    const key = canonical(c);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(c);
  }
  return [...groups.values()].filter((g) => g.length > 1);
}
function checkArbitration(cs, groups, survivors, ctx) {
  const expected = independentGroups(cs);
  eq(
    groups.length,
    expected.length,
    'independent canonical redundant group count',
  );
  let offset = 0;
  const removed = new Set();
  groups.forEach((g, n) => {
    keys(g, 'candidates winner election');
    const vs = expected[n];
    eq(
      g.candidates,
      vs.map((c) => c.id),
      'canonical tree/root/capture/inhibition/dependency grouping',
    );
    const ws = vs.map(
      (c) =>
        Math.max(
          0,
          Number(integer(c.score.effective.n)) /
            Number(integer(c.score.effective.d)),
        ) * c.strength,
    );
    const bits = ws.map((w) => {
      const b = Buffer.alloc(8);
      b.writeDoubleLE(w);
      return b.readBigUInt64LE().toString(16).padStart(16, '0');
    });
    offset += checkElection(
      g.election,
      { ...ctx, weight_bits: bits, eligible: vs.map(() => true) },
      'Redundancy',
      offset,
    );
    eq(g.winner, g.election.winner === null ? null : vs[g.election.winner].id);
    for (const c of vs) if (c.id !== g.winner) removed.add(c.id);
  });
  eq(
    survivors,
    cs.filter((c) => !removed.has(c.id)),
    'stable complete surviving contribution metadata',
  );
  return offset;
}
const POLICY_IDS = [
  'default-threshold',
  'negative-sign',
  'positive-sign',
  'brainstorm',
  'positive-atp',
  'negative-atp',
  'below-neighbor',
  'above-neighbor',
  'upper-endpoint',
  'lower-endpoint',
  'displaced-endpoint',
  'crystallized',
  'subthreshold-strength100',
  'zero-strength',
  'zero-cap',
  'disabled',
  'fractional-cap',
  'clamped-jitter',
];
function checkEntries(es) {
  keys(
    es,
    'seed batch_stream batch_draws batch_consumed evaluated arbitration refusals',
  );
  u64(es.seed);
  eq(es.batch_stream, 'entry-batch');
  eq(es.batch_draws, []);
  eq(es.batch_consumed, 0);
  eq(es.evaluated.length, 12);
  const cs = [];
  for (const o of es.evaluated) {
    keys(o, 'input evidence score election contribution');
    const i = checkInput(o.input);
    entryScore(o.score, i, o.evidence, 'entry-batch');
    assert(o.score.qualified && o.contribution, 'actual entry contribution');
    checkContribution(o.contribution, i, o.score, o.election, o.evidence);
    cs.push(o.contribution);
  }
  eq(new Set(cs.map((c) => c.id)).size, 12);
  eq(cs[0].root.input, cs[1].root.input);
  eq(cs[0].root.clause, cs[1].root.clause);
  eq(cs[0].root.action, cs[1].root.action);
  assert(
    cs[0].root.fanout !== cs[1].root.fanout,
    'actual fanout provenance differs',
  );
  assert(cs[1].root.rules.length > 0);
  assert(cs[0].captures[0].span[0] !== cs[1].captures[0].span[0]);
  eq(canonical(cs[0]), canonical(cs[1]), 'capture spans excluded');
  assert(
    cs[0].node === cs[2].node && cs[0].entry !== cs[2].entry,
    'multiple same-node entries survive complementary actions',
  );
  keys(es.arbitration, 'stream draws consumed groups survivors');
  eq(
    es.arbitration.consumed,
    checkArbitration(cs, es.arbitration.groups, es.arbitration.survivors, {
      seed: es.seed,
      stream: es.arbitration.stream,
      draws: es.arbitration.draws,
    }),
  );
  eq(es.arbitration.groups.length, 1);
  eq(es.arbitration.survivors.length, 11);
  eq(
    es.refusals.map((r) => r.id),
    [
      'low-no-bypass',
      'low-bypass',
      'guard-bypass',
      'blocked-automaton',
      'stale-binding',
    ],
  );
  for (const r of es.refusals) {
    keys(r, 'id input evidence output after context_owner');
    const i = checkInput(r.input);
    keys(r.output, 'score contribution error');
    eq(r.after, 0);
    assert(r.context_owner.startsWith('probe-authored'));
    if (r.id === 'stale-binding') {
      eq(r.output.error, 'BindingMismatch');
      eq(r.output.score, null);
      eq(r.output.contribution, null);
      assert(i.binding.entry !== i.entry);
      continue;
    }
    eq(r.output.error, null);
    entryScore(r.output.score, i, r.evidence, 'refusal');
    if (r.id === 'low-bypass') {
      assert(cmp(read(r.output.score.effective), [50n, 1n]) < 0n);
      checkContribution(
        r.output.contribution,
        i,
        r.output.score,
        undefined,
        r.evidence,
      );
      eq(
        r.output.score.effective,
        es.refusals[0].output.score.effective,
        'bypass retains real score',
      );
    } else eq(r.output.contribution, null);
  }
}
function replay(s) {
  const expectedOps =
    s.id === 'shared-cap-fairness-drain'
      ? [
          'begin',
          'admit',
          'seal',
          'compose',
          ...Array.from({ length: 6 }, () => [
            'dispatch',
            'dispatch',
            'complete',
            'complete-duplicate',
            'complete',
            'complete-duplicate',
          ]).flat(),
          'close',
          'seal',
          'compose',
        ]
      : [
          'begin',
          'admit',
          'dispatch',
          'cancel',
          'late-result',
          'restart',
          'ack',
          'ack',
          'begin',
          'admit',
          'dispatch',
          'ack',
          'ack',
          'late-result',
          'retire',
          'begin',
          'stale-handle-seal',
          'dispatch',
          'complete',
          'wrong-scope-dispatch',
        ];
  eq(
    s.events.map((e) => e.command.op),
    expectedOps,
    'required ordered broker command count/shape',
  );
  keys(s, 'id limits input initial events');
  checkInput(s.input);
  keys(s.limits, 'active pending owned_bytes operation_work trace');
  assert(
    s.limits.active > 0 &&
      s.limits.active <= 1000 &&
      s.limits.pending <= 100000,
  );
  keys(s.initial, 'generation counts');
  eq(s.initial.counts, {
    pending: 0,
    active: 0,
    quarantined: 0,
    owned_within_limit: true,
  });
  let generation = s.initial.generation,
    quantum = 0;
  const cycles = new Map(),
    active = new Map();
  let fg = [],
    mt = [],
    previous = s.initial.counts;
  const completed = [];
  const dispatchOrder = [];
  const token = (l) => `${l.cycle}:${l.job}:${l.generation}`;
  for (const e of s.events) {
    keys(e, 'command result counts');
    const q = e.command;
    assert(q && typeof q.op === 'string');
    let result = 'Ok',
      mutates = true;
    const cycle = () => {
      assert(cycles.has(q.cycle), 'referenced cycle exists');
      return cycles.get(q.cycle);
    };
    switch (q.op) {
      case 'begin':
        keys(q, 'op cycle generation');
        id(q.cycle);
        u64(q.generation);
        eq(q.generation, generation);
        assert(!cycles.has(q.cycle));
        cycles.set(q.cycle, {
          jobs: new Map(),
          status: 'Open',
          closed: false,
          extra: 0,
          creationGeneration: generation,
        });
        break;
      case 'admit':
        keys(q, 'op cycle jobs');
        {
          const c = cycle();
          assert(c.status === 'Open' && !c.closed);
          for (const j of q.jobs) {
            keys(j, 'id priority');
            id(j.id);
            assert(!c.jobs.has(j.id));
            assert(['Foreground', 'Maintenance'].includes(j.priority));
            c.jobs.set(j.id, 'Pending');
            (j.priority === 'Foreground' ? fg : mt).push({
              cycle: q.cycle,
              job: j.id,
            });
          }
        }
        break;
      case 'dispatch':
        keys(q, 'op maximum');
        {
          const leases = [];
          let n = Math.min(
            q.maximum,
            s.limits.active - active.size,
            fg.length + mt.length,
          );
          while (n-- > 0) {
            const use = fg.length > 0 && (!mt.length || quantum < 8);
            let item;
            if (use) {
              quantum = mt.length ? quantum + 1 : 0;
              item = fg.shift();
            } else {
              quantum = 0;
              item = mt.shift();
            }
            const l = { ...item, generation };
            cycles.get(l.cycle).jobs.set(l.job, 'Active');
            active.set(token(l), { ...l, revoked: false });
            leases.push(l);
            dispatchOrder.push(l.job);
          }
          result = leases;
        }
        break;
      case 'complete':
      case 'complete-duplicate':
      case 'late-result':
        {
          keys(
            q,
            q.op === 'complete'
              ? 'op lease evaluation'
              : q.op === 'late-result'
                ? 'op lease score'
                : 'op lease',
          );
          const l = q.lease;
          keys(l, 'cycle job generation');
          id(l.cycle);
          id(l.job);
          u64(l.generation);
          const t = active.get(token(l));
          if (!t) {
            result = 'InvalidLease';
            mutates = false;
          } else if (t.revoked) {
            result = 'Revoked';
            mutates = false;
          } else if (l.generation !== generation) {
            result = 'StaleGeneration';
            mutates = false;
          } else {
            assert(q.op === 'complete');
            const o = q.evaluation;
            assert(o && o.score);
            const i = { ...s.input, id: l.job };
            if (o.contribution) {
              entryScore(o.score, i, o.contribution.evidence, 'broker-score');
              checkContribution(o.contribution, i, o.score);
              completed.push(o.contribution);
            } else {
              checkScore(o.score, [60n, 1n], i.policy, {
                seed: '18446744073709551557',
                stream: 'broker-score',
                draws: [],
              });
            }
            active.delete(token(l));
            cycles.get(l.cycle).jobs.set(l.job, 'Done');
            // Actual target byte charges are checked in the separate diagnostic view.
          }
        }
        break;
      case 'close':
        keys(q, 'op cycle');
        {
          const c = cycle();
          if (c.status !== 'Open' || c.closed) {
            result = 'Closed';
            mutates = false;
          } else if ([...c.jobs.values()].some((v) => v !== 'Done')) {
            result = 'Incomplete';
            mutates = false;
          } else c.closed = true;
        }
        break;
      case 'seal':
        keys(q, 'op cycle');
        {
          const c = cycle();
          if (c.status !== 'Open') {
            result = 'Closed';
            mutates = false;
          } else if (
            !c.closed ||
            [...c.jobs.values()].some((v) => v !== 'Done')
          ) {
            result = 'Incomplete';
            mutates = false;
          } else c.status = 'Ready';
        }
        break;
      case 'compose':
        {
          const c = cycle();
          if (c.status !== 'Ready') {
            keys(q, 'op cycle');
            result = 'Incomplete';
            mutates = false;
          } else {
            keys(q, 'op cycle seed stream draws');
            const p = e.result;
            keys(p, 'cycle generation nodes groups arbitration');
            eq(p.cycle, q.cycle);
            eq(p.generation, generation);
            eq(p.groups, []);
            const survivors = p.nodes.map((n) => {
              keys(n, 'contribution attachment');
              eq(n.attachment, null);
              return n.contribution;
            });
            checkArbitration(
              [...completed].sort(
                (a, b) =>
                  [...c.jobs.keys()].indexOf(a.id) -
                  [...c.jobs.keys()].indexOf(b.id),
              ),
              p.arbitration,
              survivors,
              { seed: q.seed, stream: q.stream, draws: q.draws },
            );
            c.status = 'Composed';
            result = p;
          }
        }
        break;
      case 'cancel':
        keys(q, 'op cycle');
        {
          const c = cycle();
          assert(c.status === 'Open');
          for (const [j, state] of c.jobs) {
            if (state === 'Pending') c.jobs.set(j, 'Cancelled');
            else if (state === 'Active') {
              c.jobs.set(j, 'Revoked');
              for (const a of active.values())
                if (a.cycle === q.cycle && a.job === j) a.revoked = true;
            }
          }
          fg = fg.filter((j) => j.cycle !== q.cycle);
          mt = mt.filter((j) => j.cycle !== q.cycle);
          c.status = 'Partial';
          c.extra = 0;
        }
        break;
      case 'restart':
        keys(q, 'op from to');
        eq(q.from, generation);
        assert(integer(q.to) > integer(generation));
        for (const c of cycles.values()) {
          if (c.status !== 'Composed' && c.status !== 'Partial') {
            c.status = 'Partial';
            c.extra = 0;
            for (const [j, st] of c.jobs)
              if (st === 'Pending') c.jobs.set(j, 'Cancelled');
              else if (st === 'Active') c.jobs.set(j, 'Revoked');
          }
        }
        for (const a of active.values()) a.revoked = true;
        fg = [];
        mt = [];
        quantum = 0;
        generation = q.to;
        break;
      case 'ack':
        keys(q, 'op owner worker lease');
        {
          const t = active.get(token(q.lease));
          if (q.owner !== generation || q.worker !== q.lease.generation) {
            result = 'StaleGeneration';
            mutates = false;
          } else if (!t || !t.revoked) {
            result = 'InvalidLease';
            mutates = false;
          } else {
            active.delete(token(q.lease));
            cycles.get(t.cycle).jobs.set(t.job, 'Cancelled');
          }
        }
        break;
      case 'retire':
        keys(q, 'op cycle');
        {
          const c = cycle();
          assert(['Partial', 'Composed'].includes(c.status));
          assert(
            [...c.jobs.values()].every(
              (v) => !['Pending', 'Active', 'Revoked'].includes(v),
            ),
          );
          cycles.delete(q.cycle);
        }
        break;
      case 'stale-handle-seal':
        keys(q, 'op cycle handle_generation current_generation');
        assert(cycles.has(q.cycle));
        eq(q.current_generation, generation);
        assert(
          q.handle_generation !== cycles.get(q.cycle).creationGeneration,
          'explicit stale creation handle provenance',
        );
        result = 'Identity';
        mutates = false;
        break;
      case 'wrong-scope-dispatch':
        keys(q, 'op maximum');
        result = 'Scope';
        mutates = false;
        break;
      default:
        assert.fail(`unknown broker command ${q.op}`);
    }
    eq(e.result, result, `broker transition result ${q.op}`);
    keys(e.counts, 'pending active quarantined owned_within_limit');
    const expected = {
      pending: fg.length + mt.length,
      active: active.size,
      quarantined: [...active.values()].filter((a) => a.revoked).length,
    };
    for (const [k, v] of Object.entries(expected))
      eq(e.counts[k], v, `broker independent transition ${q.op}/${k}`);
    assert(
      e.counts.active <= s.limits.active &&
        e.counts.quarantined <= e.counts.active,
    );
    assert(e.counts.pending <= s.limits.pending);
    eq(
      e.counts.owned_within_limit,
      true,
      'actual target accounting remains within configured owned limit',
    );
    const jobs = [...cycles.values()].reduce((n, c) => n + c.jobs.size, 0);
    assert(
      4 * cycles.size + 4 * jobs <= s.limits.operation_work,
      'structural cleanup lower bound; E independently tested only in zero-extra fixture',
    );
    if (!mutates) eq(e.counts, previous, 'refusal has no effects');
    previous = e.counts;
  }
  if (s.id === 'shared-cap-fairness-drain') {
    eq(dispatchOrder.length, 12);
    const ids = s.events
      .find((e) => e.command.op === 'admit')
      .command.jobs.map((j) => j.id);
    eq(
      dispatchOrder,
      [...ids.slice(0, 8), ids[10], ids[8], ids[9], ids[11]],
      'shared FIFO 8:1 service',
    );
    eq(previous.pending, 0);
    eq(previous.active, 0);
    eq(completed.length, 12, 'all admitted work actually scored');
  } else {
    eq(s.id, 'cancel-restart-quarantine');
    eq(previous.quarantined, 0);
    assert(
      s.events.some(
        (e) =>
          e.command.op === 'dispatch' &&
          e.counts.quarantined === 1 &&
          e.result.length === 0,
      ),
      'quarantine blocks reuse',
    );
    assert(
      s.events.some(
        (e) => e.command.op === 'stale-handle-seal' && e.result === 'Identity',
      ),
    );
  }
}
function attachment(a) {
  keys(
    a,
    'id limits issuer_input target_inputs issuer link receipt after_attach early_close evaluations final_counts plan',
  );
  eq(a.id, 'hard-attachment-no-forced-vote');
  const i = checkInput(a.issuer_input);
  entryScore(a.issuer.score, i, a.issuer.evidence, 'broker-score');
  checkContribution(a.issuer, i, a.issuer.score);
  eq(a.target_inputs.length, 2);
  a.target_inputs.forEach(checkInput);
  keys(a.link, 'from to group mode remainder text');
  eq(a.link.mode, 'Hard');
  eq(a.link.from, a.issuer.node);
  assert(a.link.to !== a.link.from);
  eq(
    a.link.text,
    Buffer.from(i.source.original)
      .subarray(...a.link.remainder)
      .toString(),
  );
  eq(a.receipt, { disposition: 'Scheduled(2)', selected: true, consumed: 0 });
  eq(a.after_attach.pending, 2);
  eq(a.after_attach.active, 0);
  eq(a.early_close, 'Incomplete', 'admitted attachment work blocks completion');
  eq(a.evaluations.length, 2);
  const cs = [a.issuer];
  a.evaluations.forEach((o, n) => {
    keys(o, 'lease score evidence contribution');
    const ti = a.target_inputs[n];
    eq(o.lease.job, ti.id);
    eq(ti.source.original, a.link.text);
    eq(ti.guards, i.guards);
    eq(ti.parent, i.parent);
    eq(ti.group, a.link.group);
    eq(ti.dependencies, [i.id]);
    eq(ti.node, a.link.to);
    if (o.contribution) {
      entryScore(o.score, ti, o.contribution.evidence, 'broker-score');
      checkContribution(o.contribution, ti, o.score);
      cs.push(o.contribution);
    } else {
      entryScore(o.score, ti, o.evidence, 'broker-score');
      assert(cmp(read(o.score.scan), [50n, 1n]) < 0n);
      assert(o.score.refusals.includes('BelowThreshold'));
    }
  });
  eq(cs.length, 2);
  eq(a.final_counts.pending, 0);
  eq(a.final_counts.active, 0);
  keys(a.plan, 'cycle generation nodes groups arbitration');
  eq(a.plan.arbitration, []);
  eq(
    a.plan.nodes.map((n) => n.contribution),
    cs,
    'dependency stable topological order',
  );
  eq(a.plan.nodes[0].attachment, null);
  const context = a.plan.nodes[1].attachment;
  keys(context, 'issuer group depth remainder span parent guards captures');
  eq(context, {
    issuer: i.id,
    group: a.link.group,
    depth: 1,
    remainder: a.link.text,
    span: a.link.remainder,
    parent: i.parent,
    guards: i.guards,
    captures: i.captures,
  });
  eq(a.plan.groups, [
    { id: a.link.group, issuers: [i.id], members: cs.map((c) => c.id) },
  ]);
}
// Independent reviewed logical-charge-v1 table. No implementation or layout import.
const LOGICAL_V1 = Object.freeze({
  u8: 1,
  f64: 8,
  usize: 8,
  u128: 16,
  Span: 16,
  String: 24,
  'Vec<Expr>': 24,
  Alternative: 40,
  Expr: 48,
  InhibitionFact: 32,
  DrawRecord: 48,
  Definition: 64,
  Charge: 56,
  Capture: 80,
  RouteCapture: 96,
  Token: 160,
  EntryRequest: 384,
  Contribution: 2048,
  '&Contribution': 8,
  AttachmentContext: 320,
  Visit: 80,
  ContextReference: 8,
});
function attachmentBoundaries(a, rows, base, ledger) {
  keys(
    a,
    'issuer target work_dependencies retry_dependencies link remainder random',
  );
  const cap = { name: 'cap', span: [0, 5], value: { tag: 'Text', value: 'é' } };
  eq(a.issuer, { ...base, captures: [cap] });
  eq(a.target, {
    ...base,
    node: '2',
    entry: '2',
    binding: { ...base.binding, node: '2', entry: '2' },
  });
  eq(
    a.work_dependencies,
    Array.from({ length: 60 }, (_, i) => String(i + 1)),
  );
  eq(a.retry_dependencies, ['1']);
  eq(a.link, {
    from: '1',
    to: '2',
    group: '10',
    direction: 'Directed',
    affinity: { n: '1', d: '2' },
  });
  eq(a.remainder, [0, 5]);
  eq(a.random, { seed: '1', stream: 'attach', limit: 1 });
  const bytes = (s) => Buffer.byteLength(s, 'utf8');
  const capPayload = bytes(cap.name) + bytes(cap.value.value);
  const inherited = LOGICAL_V1.RouteCapture + capPayload;
  const header = LOGICAL_V1.AttachmentContext + LOGICAL_V1.Visit;
  const extra =
    header +
    LOGICAL_V1.ContextReference +
    inherited +
    bytes(base.parent) +
    bytes(a.random.stream);
  const rootContribution = ledger.completion + inherited;
  const rootOwned = ledger.beforeCompletion + rootContribution;
  const retainedE = rootContribution + extra;
  const selectedCleanup = 4 * 2 + 4 * 2 + retainedE,
    declinedCleanup = 4 * 2 + 4 * 1 + retainedE;
  const dependencyBytes = a.work_dependencies.length * LOGICAL_V1.u128;
  const binding = 16 + 2 * bytes(base.original) + 2 * bytes(base.parent);
  const work =
    ledger.descriptor +
    dependencyBytes +
    header +
    inherited +
    bytes(base.parent) +
    bytes(a.random.stream) +
    binding +
    dependencyBytes;
  eq(
    [
      capPayload,
      inherited,
      extra,
      rootContribution,
      rootOwned,
      retainedE,
      binding,
      work,
      selectedCleanup,
      declinedCleanup,
    ],
    [5, 101, 521, 2397, 3087, 2918, 38, 2937, 2934, 2930],
    'independent attachment fixed-table/UTF8/CJER arithmetic',
  );
  const operations = [
    ['attachment-work', 'operation_work', work],
    ['attachment-selected-owned', 'owned_bytes', rootOwned + 256 + extra + 160],
    ['attachment-declined-owned', 'owned_bytes', rootOwned + 256 + extra],
    ['attachment-selected-cleanup', 'operation_work', selectedCleanup],
    ['attachment-declined-cleanup', 'operation_work', declinedCleanup],
  ];
  eq(rows.length, 10);
  eq(
    rows.map((r) => [r.operation, r.limit_kind, r.limit]),
    operations.flatMap(([o, k, c]) => [
      [o, k, c - 1],
      [o, k, c],
    ]),
    'five fixed attachment minus-one/exact pairs',
  );
  const counts = (actual, pending, active, owned) => {
    keys(actual, 'pending active quarantined owned_bytes');
    eq(
      actual,
      { pending, active, quarantined: 0, owned_bytes: owned },
      'fixed attachment independent logical counts',
    );
  };
  const receipt = (r, accepted, selected, draw) => {
    keys(r, 'error disposition trace');
    eq(
      r.error,
      accepted ? null : 'Backpressure',
      'fixed boundary actual refusal/outcome',
    );
    if (!accepted) {
      eq(r.disposition, null);
      eq(r.trace, null);
      return;
    }
    eq(r.disposition, selected ? 'Scheduled(1)' : 'AffinityDeclined');
    keys(r.trace, 'seed stream draw affinity selected');
    eq(r.trace.seed, a.random.seed);
    eq(r.trace.stream, a.random.stream);
    eq(r.trace.affinity, a.link.affinity);
    eq(r.trace.draw, [0, draw]);
    const affinity = read(a.link.affinity);
    const expectedSelected = BigInt(draw) * affinity[1] < affinity[0] * MAX;
    eq(r.trace.selected, expectedSelected, 'independent BigInt affinity grid');
    eq(r.trace.selected, selected);
  };
  for (let n = 0; n < rows.length; n++) {
    const r = rows[n],
      accepted = n % 2 === 1,
      isWork = r.operation === 'attachment-work',
      selected = !r.operation.includes('declined');
    keys(
      r,
      'operation limit_kind limit broker_limits draws before after status_before status_after draws_before draws_after output retry extra_cycle repeated dispatched completion_kind unrelated_completion drained after_restart_retire',
    );
    const draw = selected ? 0 : Number(MAX);
    eq(r.draws, [draw]);
    keys(r.broker_limits, 'active pending owned_bytes operation_work trace');
    eq(r.broker_limits, {
      active: 1000,
      pending: 100000,
      owned_bytes: r.limit_kind === 'owned_bytes' ? r.limit : 16777216,
      operation_work: r.limit_kind === 'operation_work' ? r.limit : 1000000,
      trace: 100000,
    });
    const active = isWork ? 1 : 0,
      beforeOwned = rootOwned + (isWork ? 160 : 256),
      pending = selected ? 1 : 0;
    counts(r.before, 0, active, beforeOwned);
    eq(r.status_before, 'Open');
    eq(r.status_after, 'Open');
    eq(r.draws_before, 0);
    eq(r.draws_after, accepted ? 1 : 0);
    receipt(r.output, accepted, selected, draw);
    let finalOwned;
    if (accepted) {
      finalOwned = beforeOwned + extra + pending * 160;
      counts(r.after, pending, active, finalOwned);
      eq(r.retry, null);
    } else {
      eq(
        r.after,
        r.before,
        'fixed attachment refusal atomic counts/lifecycle/visit/identity',
      );
      keys(
        r.retry,
        'retirement comparison before draws_before output after draws_after',
      );
      eq(r.retry.retirement, isWork ? null : 'retire-second-cycle');
      eq(r.retry.comparison, 'one-dependency');
      const beforeRetry = beforeOwned - (isWork ? 0 : 256);
      counts(r.retry.before, 0, active, beforeRetry);
      eq(r.retry.draws_before, 0);
      eq(r.retry.draws_after, 1);
      receipt(r.retry.output, true, selected, draw);
      finalOwned = beforeRetry + extra + pending * 160;
      counts(r.retry.after, pending, active, finalOwned);
    }
    keys(r.repeated, 'before after draws_before draws_after output');
    counts(r.repeated.before, pending, active, finalOwned);
    eq(r.repeated.after, r.repeated.before);
    eq(r.repeated.draws_before, 1);
    eq(r.repeated.draws_after, 1);
    eq(
      r.repeated.output,
      { error: 'Repeated', disposition: null, trace: null },
      'visit retained once; refused first attempt remained retryable',
    );
    if (r.operation.endsWith('cleanup') && accepted) {
      keys(r.extra_cycle, 'error before after');
      eq(r.extra_cycle.error, 'Backpressure');
      eq(r.extra_cycle.before, r.after);
      eq(r.extra_cycle.after, r.after);
      eq(
        4 * 2 + 4 * (selected ? 2 : 1) + retainedE,
        r.limit,
        'exact cleanup bound C/J/E; extra cycle cannot enter',
      );
    } else eq(r.extra_cycle, null);
    eq(
      r.dispatched,
      selected ? [{ cycle: '1', job: '2', generation: '1' }] : [],
    );
    eq(r.completion_kind, 'explicit-host-no-vote');
    if (isWork) {
      keys(
        r.unrelated_completion,
        'lease before after duplicate_error after_duplicate',
      );
      eq(r.unrelated_completion.lease, {
        cycle: '1',
        job: '99',
        generation: '1',
      });
      counts(r.unrelated_completion.before, 0, 1, finalOwned);
      counts(r.unrelated_completion.after, 0, 0, finalOwned);
      eq(r.unrelated_completion.duplicate_error, 'InvalidLease');
      eq(
        r.unrelated_completion.after_duplicate,
        r.unrelated_completion.after,
        'unrelated lease completes once after refusal/admission',
      );
    } else eq(r.unrelated_completion, null);
    counts(r.drained, 0, 0, finalOwned);
    counts(r.after_restart_retire, 0, 0, ledger.brokerBase);
    assert(finalOwned <= r.broker_limits.owned_bytes);
  }
}

function fixedBudgets(b) {
  keys(b, 'profile fixture attachment_fixture rows');
  eq(b.profile, 'logical-charge-v1');
  const f = b.fixture;
  keys(
    f,
    'source target population node entry id key original alternatives bind origin token_spans route parent captures inhibitions dependencies group kind resource binding policy random',
  );
  eq(f.population, {
    tenant: 'tenant',
    specimen: 'specimen',
    node_type: 'text',
  });
  eq(f.node, '1');
  eq(f.entry, '1');
  eq(f.id, '1');
  eq(f.original, 'hello');
  eq(f.key, {
    encoder_version: 'text-v3',
    modality: 'text',
    resolution: 'low',
  });
  eq(f.bind, []);
  eq(f.alternatives, [
    {
      action: [{ kind: 'text', value: 'é' }],
      inhibited: false,
      weight_bits: '3ff0000000000000',
    },
  ]);
  eq(f.route, { kind: 'Wildcard', name: 'test' });
  eq(f.parent, 'parent');
  eq(f.random, { seed: '0', stream: 'test', draws: [], limit: 0 });
  eq(f.policy, {
    strength: 1,
    threshold: -100,
    jitter: false,
    cap: { n: '1', d: '1' },
    valence: { n: '0', d: '1' },
    intensity: { n: '0', d: '1' },
  });
  eq(f.captures, []);
  eq(f.inhibitions, []);
  eq(f.dependencies, []);
  eq(f.group, null);
  eq(f.resource, null);
  eq(f.kind, 'Action');
  checkOrigin(f.origin);
  eq(f.origin, { input: 'input', clause: 0, action: 0, fanout: 0, rules: [] });
  eq(f.binding, { node: '1', entry: '1', origin: f.origin });
  eq(f.token_spans, [[0, 5]]);
  const bytes = (s) => Buffer.byteLength(s, 'utf8');
  const population = Object.values(f.population).reduce(
    (n, s) => n + bytes(s),
    0,
  );
  const keyBytes = Object.values(f.key).reduce((n, s) => n + bytes(s), 0);
  const originBytes = bytes(f.origin.input);
  const literal = bytes(f.alternatives[0].action[0].value);
  eq(literal, 2, 'UTF8 é charge differs from scalar count');
  let preparedPayload = 0,
    preparedWork = 0;
  for (const a of [f.source, f.target]) {
    keys(
      a,
      'prepared tokens semantic_tokens captures operators definitions charges',
    );
    checkPrepared(a.prepared);
    eq(a.prepared.original, 'hello');
    eq(a.prepared.origin, f.origin);
    eq(a.prepared.span, [0, 5]);
    eq(a.prepared.token_spans, [[0, 5]]);
    eq(a.tokens, [{ source: 'hello', lemma: 'hello', rule_refs: [] }]);
    eq(a.semantic_tokens, 1);
    eq(a.captures, 0);
    eq(a.operators, []);
    eq(a.definitions, 0);
    eq(a.charges, 0);
    const utf8 =
      bytes(a.prepared.original) +
      bytes(a.prepared.origin.input) +
      a.tokens.reduce((n, t) => n + bytes(t.source) + bytes(t.lemma), 0);
    preparedPayload +=
      utf8 + (a.tokens.length + a.semantic_tokens) * LOGICAL_V1.Token;
    preparedWork += utf8 + a.tokens.length + 1 + a.semantic_tokens;
  }
  const ast = LOGICAL_V1.Alternative + LOGICAL_V1.Expr + literal;
  const astWork = 1 + 1 + literal;
  const entryPayload =
    2048 +
    preparedPayload +
    population +
    keyBytes +
    bytes(f.original) +
    ast +
    2 * LOGICAL_V1.Alternative +
    2 * originBytes +
    f.token_spans.length * LOGICAL_V1.Span +
    bytes(f.route.name) +
    bytes(f.parent);
  const entryWork =
    2048 +
    preparedWork +
    population +
    keyBytes +
    bytes(f.original) +
    astWork +
    2 +
    2 * originBytes +
    f.token_spans.length +
    bytes(f.route.name) +
    bytes(f.parent);
  const nested =
    2 * bytes(f.original) +
    ast +
    2 * originBytes +
    bytes(f.route.name) +
    bytes(f.parent) +
    population +
    keyBytes +
    LOGICAL_V1.Span +
    LOGICAL_V1.f64 +
    2 * bytes(f.random.stream) +
    4 * LOGICAL_V1.Span;
  const record = population + bytes(f.original) + keyBytes + ast;
  const comparison =
    record +
    2 * bytes(f.original) +
    3 * originBytes +
    LOGICAL_V1.Span +
    bytes(f.parent) +
    bytes(f.route.name);
  const binding = 2 * originBytes + 2 * bytes(f.original) + 8;
  const descriptor = 160 + comparison + 64 + binding + 2 * population;
  const completion =
    LOGICAL_V1.Contribution +
    record +
    2 * originBytes +
    bytes(f.original) +
    LOGICAL_V1.Span +
    bytes(f.parent) +
    LOGICAL_V1.f64 +
    2 * bytes(f.random.stream) +
    4 * LOGICAL_V1.Span +
    bytes(f.route.name);
  const brokerBase = population + 256,
    beforeCompletion = brokerBase + 256 + 160;
  eq(
    [
      entryPayload,
      entryWork,
      nested,
      record,
      comparison,
      binding,
      descriptor,
      completion,
      brokerBase,
      beforeCompletion,
    ],
    [2971, 2158, 248, 127, 178, 28, 466, 2296, 274, 690],
    'independent fixed metadata/UTF8 ledger',
  );
  const operations = [
    ['single-payload', 'payload_bytes', entryPayload],
    ['single-work', 'work', entryWork],
    ['batch-payload', 'payload_bytes', LOGICAL_V1.EntryRequest + entryPayload],
    [
      'arbitration-payload',
      'payload_bytes',
      LOGICAL_V1.Contribution + 3 * nested,
    ],
    ['descriptor-work', 'operation_work', descriptor],
    ['completion-owned', 'owned_bytes', beforeCompletion + completion],
    ['completion-work', 'operation_work', 4 + 4 + completion],
  ];
  eq(b.rows.length, 24, 'twelve strict minus-one/exact pairs');
  eq(
    b.rows.slice(0, 14).map((r) => [r.operation, r.limit_kind, r.limit]),
    operations.flatMap(([o, k, c]) => [
      [o, k, c - 1],
      [o, k, c],
    ]),
    'fixed boundary names/order/identical limits',
  );
  const checkCounts = (c, expected) => {
    keys(c, 'pending active quarantined owned_bytes');
    eq(c, expected, 'fixed boundary logical counts and atomicity');
  };
  attachmentBoundaries(b.attachment_fixture, b.rows.slice(14), f, {
    completion,
    descriptor,
    beforeCompletion,
    brokerBase,
  });
  for (let n = 0; n < 14; n++) {
    const r = b.rows[n],
      accepted = n % 2 === 1;
    keys(
      r,
      n < 8
        ? 'operation limit_kind limit scoring_budget draws_before draws_after output'
        : 'operation limit_kind limit broker_limits generation cycle job before after status_before status_after output fallback duplicate extra_cycle after_restart_retire',
    );
    if (n < 8) {
      keys(r.scoring_budget, 'contributions work payload_bytes draws');
      eq(r.scoring_budget, {
        contributions: 100000,
        work: r.limit_kind === 'work' ? r.limit : 1000000,
        payload_bytes: r.limit_kind === 'payload_bytes' ? r.limit : 16777216,
        draws: 100000,
      });
      eq(r.draws_before, 0);
      eq(r.draws_after, 0, 'fixed boundary atomic RNG');
      keys(r.output, 'error contribution');
      eq(
        r.output.error,
        accepted ? null : 'BudgetExceeded',
        'fixed boundary actual refusal/outcome',
      );
      if (accepted) {
        const c = r.output.contribution;
        assert(c);
        eq(c.id, '1');
        keys(
          c,
          'id node entry population root source_original target_origin parent inhibitions captures dependencies group kind slot strength action score election evidence',
        );
        eq(c.node, '1');
        eq(c.entry, '1');
        eq(c.population, f.population);
        eq(c.root, f.origin);
        eq(c.source_original, 'hello');
        eq(c.target_origin, f.origin);
        eq(c.parent, f.parent);
        eq(c.action, f.alternatives[0].action);
        eq(c.slot, 0);
        eq(c.strength, 1);
        eq(c.dependencies, []);
        eq(c.captures, []);
        eq(c.inhibitions, []);
        eq(c.group, null);
        eq(c.kind, 'Action');
        checkScore(c.score, checkEvidence(c.evidence), f.policy, f.random);
        checkElection(c.election, {
          ...f.random,
          weight_bits: ['3ff0000000000000'],
          eligible: [true],
        });
      } else eq(r.output.contribution, null);
      continue;
    }
    keys(r.broker_limits, 'active pending owned_bytes operation_work trace');
    eq(r.broker_limits, {
      active: 1000,
      pending: 100000,
      owned_bytes: r.limit_kind === 'owned_bytes' ? r.limit : 16777216,
      operation_work: r.limit_kind === 'operation_work' ? r.limit : 1000000,
      trace: 100000,
    });
    eq(r.generation, '1');
    eq(r.cycle, '1');
    eq(r.job, '1');
    eq(r.status_before, 'Open');
    eq(r.status_after, 'Open');
    keys(r.output, 'error proof_returned');
    eq(
      r.output.error,
      accepted ? null : 'Backpressure',
      'fixed boundary actual refusal/outcome',
    );
    const descriptorRow = r.operation === 'descriptor-work';
    const before = {
      pending: 0,
      active: descriptorRow ? 0 : 1,
      quarantined: 0,
      owned_bytes: descriptorRow ? brokerBase + 256 : beforeCompletion,
    };
    checkCounts(r.before, before);
    const after = accepted
      ? {
          pending: descriptorRow ? 1 : 0,
          active: 0,
          quarantined: 0,
          owned_bytes: descriptorRow
            ? beforeCompletion
            : beforeCompletion + completion,
        }
      : before;
    checkCounts(r.after, after);
    eq(r.output.proof_returned, accepted && !descriptorRow);
    if (!accepted)
      eq(
        r.after,
        r.before,
        'fixed boundary refusal leaves private lease/lifecycle/counts unchanged',
      );
    if (descriptorRow) {
      keys(r.fallback, 'dispatched counts');
      eq(
        r.fallback.dispatched,
        accepted ? [{ cycle: '1', job: '1', generation: '1' }] : [],
      );
      checkCounts(r.fallback.counts, { ...after, pending: 0 });
      eq(r.duplicate, null);
      eq(r.extra_cycle, null);
    } else {
      if (accepted) eq(r.fallback, null);
      else {
        keys(r.fallback, 'result proof_returned counts');
        eq(
          r.fallback.result,
          'Ok',
          'same private lease completes explicit no-vote after refusal',
        );
        eq(r.fallback.proof_returned, false);
        checkCounts(r.fallback.counts, { ...before, active: 0 });
      }
      keys(r.duplicate, 'error counts');
      eq(r.duplicate.error, 'InvalidLease');
      checkCounts(r.duplicate.counts, { ...after, active: 0 });
      if (r.operation === 'completion-work' && accepted) {
        keys(r.extra_cycle, 'error before after');
        eq(r.extra_cycle.error, 'Backpressure');
        eq(r.extra_cycle.before, after);
        eq(
          r.extra_cycle.after,
          after,
          'filled cleanup reservation refuses extra cycle atomically',
        );
      } else eq(r.extra_cycle, null);
    }
    checkCounts(r.after_restart_retire, {
      pending: 0,
      active: 0,
      quarantined: 0,
      owned_bytes: brokerBase,
    });
  }
}

function validate(t) {
  keys(
    t,
    'schema profile scanner scope logical_budget numeric elections seeded entries orchestration',
  );
  eq(t.schema, 'studio-scoring-parity-v2');
  fixedBudgets(t.logical_budget);
  eq(t.profile, 'scoring-orchestration-v3-2026-10-23');
  eq(t.scanner, 'text-scanner-v3');
  eq(t.scope, 'authored-synthetic-pure-mechanisms');
  eq(
    t.numeric.map((x) => x.id),
    POLICY_IDS,
    'required policy scenario names/count/order',
  );
  for (const c of t.numeric) {
    keys(c, 'id input before after output');
    keys(c.input, 'scan policy seed stream draws limit');
    u64(c.input.seed);
    eq(c.input.seed, '18446744073709551557');
    eq(c.input.stream, c.id);
    eq(c.before, 0);
    eq(c.input.limit, 2);
    eq(c.input.draws.length, 2);
    eq(
      c.after,
      checkScore(c.output, read(c.input.scan), c.input.policy, c.input),
    );
  }
  eq(
    t.elections.map((x) => x.id),
    [
      'zero',
      'singleton',
      'first-endpoint',
      'last-endpoint',
      'interior',
      'ineligible',
    ],
  );
  for (const c of t.elections) {
    keys(c, 'id input before after output');
    keys(c.input, 'weight_bits eligible seed stream draws limit');
    eq(c.before, 0);
    eq(c.input.limit, 1);
    eq(c.input.draws.length, 1);
    eq(c.input.stream, c.id);
    eq(c.after, checkElection(c.output, c.input));
  }
  keys(t.seeded, 'seed count values');
  eq(t.seeded.count, 6);
  u64(t.seeded.seed);
  let state = integer(t.seeded.seed);
  const mask = (1n << 64n) - 1n;
  const ds = [];
  for (let i = 0; i < t.seeded.count; i++) {
    state = (state + 0x9e3779b97f4a7c15n) & mask;
    let z = state;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & mask;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & mask;
    ds.push(Number((z ^ (z >> 31n)) >> 32n));
  }
  eq(t.seeded.values, ds, 'independent masked SplitMix64 BigInt');
  checkEntries(t.entries);
  keys(t.orchestration, 'fairness quarantine attachment headroom');
  replay(t.orchestration.fairness);
  replay(t.orchestration.quarantine);
  attachment(t.orchestration.attachment);
  const h = t.orchestration.headroom;
  keys(
    h,
    'id limits retained_cycles retained_jobs extra before before_owned refusal after after_owned post_restart_refusal post_retire_retry final_counts',
  );
  eq(h.id, 'cleanup-reservation');
  eq(h.retained_cycles, 2);
  eq(h.retained_jobs, 0);
  eq(h.extra, 0);
  eq(
    4 * h.retained_cycles + 4 * h.retained_jobs + h.extra,
    h.limits.operation_work,
  );
  eq(h.refusal, 'Backpressure');
  eq(h.after, h.before);
  eq(h.before_owned, h.after_owned);
  eq(
    h.before_owned,
    Object.values(t.orchestration.fairness.input.population).reduce(
      (n, s) => n + Buffer.byteLength(s),
      0,
    ) +
      256 +
      2 * 256,
    'observed zero-extra retained inventory bytes',
  );
  eq(h.post_restart_refusal, 'Backpressure');
  eq(h.post_retire_retry, 'Ok');
  return true;
}
function parse(bytes) {
  assert(
    bytes.length > 1000 && bytes.length < 512 * 1024,
    'strict bounded fixture bytes',
  );
  const raw = JSON.parse(bytes.toString());
  keys(raw, 'schema semantic diagnostics');
  eq(raw.schema, 'studio-scoring-parity-envelope-v2');
  keys(raw.diagnostics, 'schema charge_profile target_arch observations');
  eq(raw.diagnostics.schema, 'logical-charge-v1-observations');
  eq(raw.diagnostics.charge_profile, 'logical-charge-v1');
  assert(['aarch64', 'wasm32'].includes(raw.diagnostics.target_arch));
  const expected = [];
  function visit(v, path) {
    if (v && typeof v === 'object') {
      if (
        Object.hasOwn(v, 'owned_within_limit') &&
        !path.endsWith('/initial/counts')
      )
        expected.push(path);
      for (const [k, c] of Object.entries(v)) visit(c, `${path}/${k}`);
    }
  }
  visit(raw.semantic, '');
  eq(
    raw.diagnostics.observations.map((o) => o.path),
    expected,
    'strict actual accounting diagnostic paths',
  );
  eq(expected.length, 68);
  for (const o of raw.diagnostics.observations) {
    keys(o, 'path owned_bytes configured_limit');
    eq(o.configured_limit, 16 * 1024 * 1024);
    assert(
      Number.isSafeInteger(o.owned_bytes) &&
        o.owned_bytes > 0 &&
        o.owned_bytes <= o.configured_limit,
      'each actual target byte charge <=configured limit',
    );
  }
  return raw.semantic;
}
const hash = (b) => createHash('sha256').update(b).digest('hex');
const args = process.argv.slice(2);
if (args[0] === '--check-json') {
  validate(parse(readFileSync(args[1])));
  console.log('PASS: normal independent scoring comparator');
} else if (args[0] === '--self-test') {
  const good = parse(readFileSync(args[1]));
  validate(good);
  let count = 0;
  for (const mutate of [
    (t) => {
      delete t.numeric[0].output.effective;
    },
    (t) => {
      t.schema = 'unknown';
    },
    (t) => {
      t.numeric.pop();
    },
    (t) => {
      t.numeric.push(t.numeric[0]);
    },
    (t) => {
      t.numeric[0].output.effective.d = '0';
    },
    (t) => {
      t.numeric[0].after = 1;
    },
    (t) => {
      t.entries.evaluated = [];
    },
    (t) => {
      t.orchestration.fairness.events = [];
    },
  ]) {
    const t = structuredClone(good);
    mutate(t);
    assert.throws(() => validate(t));
    count++;
  }
  assert.throws(() => parse(Buffer.from('{')));
  console.log(
    `PASS: ${count} invalid/incomplete shapes plus malformed JSON refused by normal comparator`,
  );
} else {
  const [
    nativePath,
    featurePath,
    defaultPath,
    wasmOut,
    negativeArithmetic,
    negativeTransition,
    negativeBoundary,
  ] = args;
  assert(negativeBoundary, 'seven paths required');
  const native = readFileSync(nativePath);
  const t = parse(native);
  validate(t);
  const defaultBytes = readFileSync(defaultPath),
    defaultModule = await WebAssembly.compile(defaultBytes);
  const de = WebAssembly.Module.exports(defaultModule).map((e) => e.name);
  assert(!de.includes('scoring_probe_raw'), 'default export absent');
  console.log(
    'PASS: default WASM compile/export inspection only; not instantiated',
  );
  const bytes = readFileSync(featurePath),
    featureModule = await WebAssembly.compile(bytes);
  assert(
    WebAssembly.Module.exports(featureModule).some(
      (e) => e.name === 'scoring_probe_raw',
    ),
  );
  let hostCalls = 0;
  const imports = {};
  for (const i of WebAssembly.Module.imports(featureModule)) {
    assert(
      i.kind === 'function',
      'all observed imports must be throwing functions',
    );
    imports[i.module] ??= {};
    imports[i.module][i.name] = () => {
      hostCalls++;
      throw Error(`forbidden host call ${i.module}.${i.name}`);
    };
  }
  const instance = await WebAssembly.instantiate(featureModule, imports);
  const packed = instance.exports.scoring_probe_raw();
  const ptr = Number(packed >> 32n),
    len = Number(packed & 0xffffffffn);
  assert(len > 1000 && len < 512 * 1024);
  const wasm = Buffer.from(
    new Uint8Array(instance.exports.memory.buffer, ptr, len),
  );
  instance.exports.free(ptr, len);
  writeFileSync(wasmOut, wasm);
  validate(parse(wasm));
  eq(JSON.parse(native).diagnostics.target_arch, 'aarch64');
  eq(JSON.parse(wasm).diagnostics.target_arch, 'wasm32');
  eq(
    JSON.parse(wasm).diagnostics.observations,
    JSON.parse(native).diagnostics.observations,
    'matching observed logical-charge-v1 diagnostics across targets',
  );
  const nativeSemantic = Buffer.from(JSON.stringify(t));
  const wasmSemantic = Buffer.from(JSON.stringify(parse(wasm)));
  eq(
    wasmSemantic.equals(nativeSemantic),
    true,
    'actual native/WASM projected semantic bytes',
  );
  writeFileSync(nativePath + '.semantic.json', nativeSemantic);
  writeFileSync(wasmOut + '.semantic.json', wasmSemantic);
  eq(hostCalls, 0);
  const corrupt = JSON.parse(native);
  corrupt.semantic.numeric[0].output.effective.n = '51';
  writeFileSync(negativeArithmetic, JSON.stringify(corrupt));
  const transition = JSON.parse(native);
  const revoked = transition.semantic.orchestration.quarantine.events.find(
    (e) => e.command.op === 'cancel',
  );
  assert(revoked.counts.quarantined === 1);
  revoked.counts.quarantined = 0;
  writeFileSync(negativeTransition, JSON.stringify(transition));
  const boundary = JSON.parse(native);
  boundary.semantic.logical_budget.rows[14].output.error = null;
  writeFileSync(negativeBoundary, JSON.stringify(boundary));
  console.log(
    JSON.stringify({
      result: 'PASS',
      logical_charge_profile: 'logical-charge-v1',
      fixed_boundary_rows: 24,
      matching_logical_diagnostics: 68,
      semantic_bytes: nativeSemantic.length,
      semantic_native_sha256: hash(nativeSemantic),
      semantic_wasm_sha256: hash(wasmSemantic),
      raw_native_bytes: native.length,
      raw_wasm_bytes: wasm.length,
      native_sha256: hash(native),
      wasm_sha256: hash(wasm),
      host_calls: hostCalls,
      feature_imports: WebAssembly.Module.imports(featureModule),
      default_exports: de,
      feature_sha256: hash(bytes),
      default_sha256: hash(defaultBytes),
    }),
  );
}
