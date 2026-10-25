// Standalone opt-in witness. Every mode uses the same semantic comparator.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const eq = (a, b, message) => assert.deepEqual(a, b, message);
const cat = (...parts) => Buffer.concat(parts.map((p) => Buffer.from(p)));
const u64 = (n) => {
  const b = Buffer.alloc(8);
  b.writeBigUInt64BE(BigInt(n));
  return b;
};
const lp = (v) => {
  const b = Buffer.from(v);
  return cat(u64(b.length), b);
};
const list = (xs) => cat(u64(xs.length), ...xs.map(lp));
const decBytes = (d) => cat(lp(d.coefficient), lp(String(d.scale)));
const lexical = (tree) =>
  tree.tag === 'action'
    ? tree.tokens.filter((t) => t.kind !== 'Punctuation')
    : tree.tag === 'negated'
      ? lexical(tree.child)
      : tree.children.flatMap(lexical);
const count = (tree) =>
  tree.tag === 'action'
    ? lexical(tree).length
    : tree.tag === 'negated'
      ? 1 + count(tree.child)
      : 1 + tree.children.map(count).reduce((a, b) => a + b, 0);
function tokenBytes(t, low) {
  const value = t.decimal
    ? low
      ? Buffer.from('#')
      : decBytes(t.decimal)
    : Buffer.from(t.lemma);
  return cat(
    lp(t.kind.toLowerCase()),
    lp(value),
    ...(low ? [] : [lp(t.role.toLowerCase())]),
  );
}
function treeBytes(t, level) {
  if (t.tag === 'action')
    return level === 'control'
      ? lp('action')
      : cat(
          lp('action'),
          list(
            t.tokens
              .filter((x) => level === 'high' || x.kind !== 'Punctuation')
              .map((x) => tokenBytes(x, false)),
          ),
        );
  if (t.tag === 'negated')
    return cat(
      lp('negated'),
      lp(t.uncertain ? 'uncertain' : 'explicit'),
      treeBytes(t.child, level),
    );
  return cat(lp(t.tag), u64(2), ...t.children.map((c) => treeBytes(c, level)));
}
function checkKey(k) {
  const n = count(k.action.ast);
  const level = n <= 6 ? 'low' : n <= 16 ? 'medium' : 'high';
  eq(k.count, n);
  eq(k.class, level);
  eq(k.encoder, 'text-v3');
  eq(k.modality, 'text');
  let content;
  if (level === 'low') {
    const unique = [
      ...new Map(
        lexical(k.action.ast).map((t) => {
          const b = tokenBytes(t, true);
          return [b.toString('hex'), b];
        }),
      ).values(),
    ].sort(Buffer.compare);
    content = cat(list(unique), treeBytes(k.action.ast, 'control'));
  } else content = treeBytes(k.action.ast, level);
  const a = k.action;
  const guards = cat(
    ...[
      ['inhibited', a.constraints.inhibited],
      ['uncertain-negation', a.constraints.uncertain],
      ['unresolved-frame', a.constraints.unresolved_frame],
    ].flatMap(([name, value]) => [lp(name), lp(String(value))]),
    lp('condition'),
    lp(a.constraints.condition.toLowerCase()),
  );
  let extra = Buffer.alloc(0);
  if (level === 'high')
    extra = cat(
      lp('spo'),
      ...a.spo.map((s) =>
        s === null ? lp('unresolved') : cat(lp('resolved'), lp(s)),
      ),
      list(a.captures.map((c) => decBytes(c.decimal))),
    );
  if (level !== 'low')
    extra = cat(extra, list(a.operators.map((s) => Buffer.from(s))));
  const transform = cat(lp('text-v3'), content, u64(1), guards, extra);
  eq(Buffer.from(k.transform), transform, 'independent typed transform');
  const preimage = cat(lp(transform), lp(level), lp('text'));
  eq(Buffer.from(k.preimage), preimage, 'independent LP preimage');
  eq(Buffer.from(k.digest).length, 32);
  eq(Buffer.from(k.digest).toString('hex'), hash(preimage), 'full32 digest');
}
const abs = (n) => (n < 0n ? -n : n);
function gcd(a, b) {
  while (b) [a, b] = [b, a % b];
  return a;
}
function rat(n, d = 1n) {
  assert.ok(d > 0n);
  const g = gcd(abs(n), d);
  return [n / g, d / g];
}
const add = (a, b) => rat(a[0] * b[1] + b[0] * a[1], a[1] * b[1]);
const sub = (a, b) => rat(a[0] * b[1] - b[0] * a[1], a[1] * b[1]);
const wire = (r) => r.map(String);
const readRat = (r) => {
  eq(r.length, 2);
  r.forEach((x) => assert.equal(typeof x, 'string'));
  const v = r.map(BigInt);
  eq(wire(rat(...v)), r);
  return v;
};
function checkTotals(t, expectedS, expectedD) {
  const s = readRat(t.similarity),
    d = readRat(t.dissimilarity);
  if (expectedS) eq(s, expectedS);
  if (expectedD) eq(d, expectedD);
  const difference = sub(s, d);
  eq(t.difference, wire(difference));
  const confidence =
    difference[0] > 100n * difference[1]
      ? [100n, 1n]
      : difference[0] < -100n * difference[1]
        ? [-100n, 1n]
        : difference;
  eq(t.confidence, wire(confidence));
}
function lexeme(t) {
  return JSON.stringify(
    t.decimal
      ? ['number', t.decimal]
      : [
          t.kind === 'Quoted'
            ? 'quoted'
            : t.kind === 'Operator'
              ? 'operator'
              : 'word',
          t.lemma,
        ],
  );
}
function bag(xs) {
  const m = new Map();
  for (const x of xs) m.set(x, (m.get(x) ?? 0n) + 1n);
  return m;
}
function jaccard(a, b, weight) {
  const x = bag(a),
    y = bag(b);
  let intersection = 0n,
    union = 0n;
  for (const k of new Set([...x.keys(), ...y.keys()])) {
    const l = x.get(k) ?? 0n,
      r = y.get(k) ?? 0n;
    intersection += l < r ? l : r;
    union += l > r ? l : r;
  }
  return {
    s: union ? rat(weight * intersection, union) : [0n, 1n],
    reason:
      union === 0n
        ? 'Missing'
        : intersection === union
          ? 'Match'
          : intersection === 0n
            ? 'Mismatch'
            : 'Partial',
  };
}
function polarity(t) {
  if (t.tag === 'action') return false;
  if (t.tag === 'if-then') return polarity(t.children[1]);
  if (t.tag === 'negated') return t.uncertain ? null : true;
  const [a, b] = t.children.map(polarity);
  return a === b ? a : null;
}
function checkScan(e) {
  eq(e.version, 'text-scanner-v3');
  const a = e.left,
    b = e.right,
    lt = lexical(a.ast),
    rt = lexical(b.ast);
  const l = lt.map(lexeme),
    r = rt.map(lexeme);
  const bigrams = (xs) => xs.slice(1).map((x, i) => JSON.stringify([xs[i], x]));
  const expected = [jaccard(l, r, 40n), jaccard(bigrams(l), bigrams(r), 20n)];
  const pa = polarity(a.ast),
    pb = polarity(b.ast);
  const matched = a.spo.reduce(
    (n, s, i) => n + (s !== null && s === b.spo[i] ? 1n : 0n),
    0n,
  );
  const uncertain = pa === null || pb === null,
    conflict = !uncertain && pa !== pb;
  expected.push({
    s: uncertain || conflict ? [0n, 1n] : rat(20n * matched, 3n),
    reason: uncertain
      ? 'UncertainNegation'
      : conflict
        ? 'NegationConflict'
        : a.spo.every((s) => s === null) || b.spo.every((s) => s === null)
          ? 'Missing'
          : matched === 3n
            ? 'Match'
            : matched === 0n
              ? 'Mismatch'
              : 'Partial',
  });
  const present = a.captures.length > 0 && b.captures.length > 0;
  const exact =
    present &&
    JSON.stringify(a.captures.map((c) => c.decimal)) ===
      JSON.stringify(b.captures.map((c) => c.decimal)) &&
    JSON.stringify(a.operators) === JSON.stringify(b.operators);
  expected.push({
    s: [exact ? 20n : 0n, 1n],
    reason: !present ? 'Missing' : exact ? 'Match' : 'Mismatch',
  });
  let totalS = [0n, 1n],
    totalD = [0n, 1n];
  eq(
    e.components.map((c) => c.name),
    ['lemma-multiset', 'ordered-bigram-multiset', 'spo', 'operands-operators'],
    'ordered scanner component names/count',
  );
  e.components.forEach((c, i) => {
    const weight = BigInt(i === 0 ? 40 : 20),
      want = expected[i];
    eq(c.weight, String(weight));
    eq(c.s, wire(want.s));
    eq(c.d, wire(sub([weight, 1n], want.s)));
    eq(c.reason, want.reason);
    const spans = (action, leaves) =>
      i < 2
        ? leaves.map((t) => t.span)
        : i === 2
          ? action.tokens
              .filter((t) =>
                ['Subject', 'Predicate', 'Object'].includes(t.role),
              )
              .map((t) => t.span)
          : action.captures.map((c) => c.span);
    eq(c.left_spans, spans(a, lt));
    eq(c.right_spans, spans(b, rt));
    totalS = add(totalS, want.s);
    totalD = add(totalD, sub([weight, 1n], want.s));
  });
  eq(add(totalS, totalD), [100n, 1n]);
  checkTotals(e.totals, totalS, totalD);
}
function checkPrepared(p) {
  const bytes = Buffer.from(p.original);
  for (const a of p.actions) {
    eq(a.origin.input, 'synthetic-input');
    for (const t of [...a.tokens, ...a.captures]) {
      const [start, end] = t.span;
      assert.ok(
        Number.isInteger(start) &&
          start >= 0 &&
          end > start &&
          end <= bytes.length,
      );
      eq(bytes.subarray(start, end).toString('utf8'), t.source, 'source span');
    }
    if (
      a.constraints.inhibited ||
      a.constraints.unresolved_frame ||
      ['Unresolved', 'Refuted'].includes(a.constraints.condition)
    )
      eq(a.constraints.executable, false);
  }
}
function validate(out) {
  eq(out.schema, 'task8-language-mechanism-probe-v1');
  const f = out.final_fix;
  eq(f.invalid_controls, ['InvalidTables', 'InvalidTables', 'InvalidTables']);
  eq(f.empty_table_records, 257);
  eq(f.empty_query_error, 'BudgetExceeded');
  eq(f.selected_alternatives_error, 'BudgetExceeded');
  eq(f.complete_alternative_count, 12000);
  eq(f.complexity.getter, 6);
  eq(f.complexity.getter, f.complexity.key.count);
  checkKey(f.complexity.key);
  f.conditions.forEach((c, i) => {
    checkPrepared(c.prepared);
    const yes = [false, false, true, true, false, false][i];
    eq(c.prepared.actions.at(-1).constraints.executable, yes);
    eq(
      c.prepared.actions.at(-1).constraints.condition,
      yes ? 'Satisfied' : 'Unresolved',
    );
    eq(c.guard, yes ? 'Ok("executed")' : 'Err(ActionBlocked)');
  });
  assert.equal(Object.hasOwn(out, 'error'), false);
  [
    out.utility,
    ...out.weighted,
    out.quoted,
    out.unframed,
    ...out.numeric,
    out.decimal,
    out.routes.query,
  ].forEach(checkPrepared);
  eq(out.errors, {
    division: 'DivisionByZero',
    decimal: 'Decimal',
    quote: 'UnclosedQuote',
    syntax: 'Syntax',
    nonfinite: 'NonFinite',
    work: 'StepBudget',
  });
  const original = out.utility.actions[0];
  eq(out.utility.ast.tag, 'and');
  eq(original.constraints.inhibited, true);
  eq(original.definitions, [
    { rule: 'dictionary', definition: 'luminous', span: [6, 12] },
  ]);
  eq(original.charges, [
    { rule: 'charge', valence: 0.5, intensity: 0.75, span: [6, 12] },
  ]);
  eq(original.spo, ['bright', 'give', 'bird']);
  eq(original.tokens.find((t) => t.source === 'birds').lemma, 'bird');
  eq(original.tokens.find((t) => t.source === 'give').rules, [
    'grammar',
    'frame',
  ]);
  eq(original.tokens[0].rules, ['negation']);
  const variants = out.utility.actions.filter((a) => a.origin.clause === 0);
  eq(
    variants.map((a) => a.origin.fanout),
    [0, 1, 2, 3],
  );
  eq(
    variants.map((a) => a.origin.rules),
    [[], ['sense'], ['equivalent-a'], ['equivalent-b']],
  );
  assert.ok(
    variants.every((a) => a.constraints.inhibited && !a.constraints.executable),
  );
  assert.ok(
    variants[1].tokens.some(
      (t) => t.source === 'bank' && t.lemma === 'riverbank',
    ),
  );
  eq(out.utility.actions.at(-1).constraints.executable, true);
  eq(
    out.weighted.map((p) => p.actions[1].tokens[0].lemma),
    ['radiant', 'shining'],
  );
  eq(
    out.weighted.map((p) => p.draws),
    [1, 1],
  );
  const quoted = out.quoted.actions[0];
  eq(quoted.tokens.length, 1);
  eq(quoted.tokens[0].lemma, 'never also two 界');
  eq(quoted.tokens[0].quoted, true);
  eq(quoted.captures, []);
  eq(quoted.constraints.inhibited, false);
  eq(quoted.tokens[0].rules, []);
  eq(out.quoted.actions.at(-1).constraints.unresolved_frame, true);
  eq(out.quoted.actions.at(-1).constraints.executable, false);
  eq(
    out.unframed.actions[0].tokens.map((t) => t.lemma),
    ['one', 'two', 'three', 'four', 'five', 'six', 'seven'],
  );
  assert.ok(
    out.unframed.actions[0].tokens.every(
      (t) => t.kind === 'Word' && t.decimal === null,
    ),
  );
  eq(out.unframed.actions[0].captures, []);
  eq(out.unframed.actions[0].operators, []);
  eq(out.unframed.actions[0].value, null);
  out.numeric.forEach((p) => {
    const a = p.actions[0];
    eq(a.value, 4);
    eq(a.operators, ['+']);
    eq(
      a.captures.map((c) => c.decimal),
      [
        { coefficient: '2', scale: 0 },
        { coefficient: '2', scale: 0 },
      ],
    );
  });
  eq(
    out.numeric[0].actions[0].captures.map((c) => c.span),
    [
      [0, 3],
      [9, 12],
    ],
  );
  eq(
    out.decimal.actions[0].captures.map((c) => [c.source, c.decimal]),
    [
      ['1.200', { coefficient: '12', scale: 1 }],
      ['0.00', { coefficient: '0', scale: 0 }],
    ],
  );
  eq(
    out.boundaries.map((k) => [k.count, k.class]),
    [
      [6, 'low'],
      [7, 'medium'],
      [16, 'medium'],
      [17, 'high'],
    ],
  );
  [...out.boundaries, ...out.control_keys, ...out.shared_low.keys].forEach(
    checkKey,
  );
  Object.values(out.scans).forEach(checkScan);
  out.shared_low.scans.forEach(checkScan);
  eq(
    out.scans.antecedent_body_conflict.components[2].reason,
    'NegationConflict',
  );
  eq(out.scans.antecedent_only.components[2].s, ['20', '1']);
  eq(out.scans.uncertain_antecedent.components[2].s, ['20', '1']);
  eq(out.scans.uncertain_body.components[2].reason, 'UncertainNegation');
  eq(out.scans.repeated.components[1].s, ['20', '3']);
  eq(out.scans.partial_roles.components[2].s, ['40', '3']);
  eq(out.scans.uncertain_body.right.constraints.uncertain, true);
  eq(out.scans.uncertain_body.right.constraints.inhibited, true);
  checkTotals(out.huge, rat(10n ** 30n + 1n, 3n), rat(10n ** 30n, 3n));
  eq(out.huge.difference, ['1', '3']);
  eq(out.huge.overflow, 'Overflow');
  eq(out.shared_low.refs, ['1', '2']);
  eq(out.shared_low.originals, ['2+3', '8+9']);
  eq(out.shared_low.keys[0].digest, out.shared_low.keys[1].digest);
  eq(
    out.shared_low.scans.map((e) => e.components[3].s),
    [
      ['20', '1'],
      ['0', '1'],
    ],
  );
  eq(out.scripts.nested_denial, 'UndeclaredContact');
  eq(out.scripts.stale, 'StaleRegistry');
  eq(out.scripts.runaway, 'StepBudget');
  eq(out.scripts.inert_effects, 0);
  eq(out.scripts.tape, 'Bytes([195, 169])');
  eq(out.scripts.bind_value, 'Number(4.0)');
  eq(out.scripts.bind_trace.filter((t) => t === 'BindComputed').length, 1);
  eq(out.scripts.bind_trace.filter((t) => t === 'BindExecuted').length, 1);
  eq(
    out.scripts.allowed_effects,
    '[Read { subsystem: "resource", key: "key" }]',
  );
  eq(out.routes.rules, 3);
  eq(out.routes.hits.length, 1);
  const hit = out.routes.hits[0];
  eq(hit.entry, '10');
  eq(hit.original, 'separate target');
  eq(
    hit.origins.map((o) => o.path.kind),
    ['automaton', 'grammar', 'wildcard'],
  );
  eq(
    hit.origins.map((o) => o.path.rule),
    ['automaton', 'grammar', 'wildcard'],
  );
  assert.ok(
    hit.origins.every(
      (o) =>
        o.constraints.executable &&
        o.origin.clause === 0 &&
        o.origin.fanout === 0,
    ),
  );
  eq(hit.origins[0].path.status, 'Matched');
  eq(hit.origins[0].captures[0].value.bytes, [50]);
  eq(
    hit.origins[1].captures.map((c) => c.span),
    [
      [0, 1],
      [1, 2],
      [2, 3],
    ],
  );
  eq(
    hit.origins[1].captures.map((c) => c.value),
    [
      { number: { coefficient: '2', scale: 0 } },
      { operator: '+' },
      { number: { coefficient: '3', scale: 0 } },
    ],
  );
  eq(hit.origins[2].bypass, 'Declared');
  eq(
    hit.origins[2].captures.map((c) => c.value.text),
    ['2', '', '+3'],
  );
  eq(out.routes.repeated.length, 1);
  eq(out.routes.repeated[0].origins.length, 6);
  eq(
    out.routes.repeated[0].origins.map((o) => o.origin.clause),
    [0, 0, 0, 1, 1, 1],
  );
  eq(
    out.routes.repeated[0].origins.map((o) => o.path.kind),
    ['automaton', 'grammar', 'wildcard', 'automaton', 'grammar', 'wildcard'],
  );
  eq(out.routes.blocked.length, 1);
  eq(out.routes.blocked[0].origins.length, 1);
  const blocked = out.routes.blocked[0].origins[0];
  eq(blocked.path.status, 'Blocked');
  eq(blocked.constraints.inhibited, true);
  eq(blocked.constraints.executable, false);
  eq(blocked.captures, []);
}
const [mode, ...args] = process.argv.slice(2);
if (mode === '--self-test') {
  const out = JSON.parse(readFileSync(args[0], 'utf8'));
  validate(out);
  const corrupt = structuredClone(out);
  corrupt.boundaries[0].digest[0] ^= 1;
  assert.throws(() => validate(corrupt), /full32 digest/);
  const label = structuredClone(out);
  label.scans.numeric.components[0].name = 'corrupted-label';
  assert.throws(() => validate(label), /ordered scanner component names/);
  const missing = structuredClone(out);
  missing.scans.numeric.components.pop();
  assert.throws(() => validate(missing), /ordered scanner component names/);
  console.log(
    'PASS comparator rejects changed digest, component name and count',
  );
} else if (mode === '--check-json') {
  validate(JSON.parse(readFileSync(args[0], 'utf8')));
  console.log('PASS independent JSON semantics');
} else {
  const [wasmPath, defaultPath, outputPath, corruptPath] = args;
  assert.ok(
    mode && wasmPath && defaultPath && outputPath && corruptPath,
    'usage: bun tests/language-parity.mjs NATIVE_JSON FEATURE_WASM DEFAULT_WASM OUTPUT_JSON CORRUPT_JSON',
  );
  const native = readFileSync(mode),
    wasm = readFileSync(wasmPath),
    production = readFileSync(defaultPath);
  const defaultModule = await WebAssembly.compile(production);
  const defaultExports = WebAssembly.Module.exports(defaultModule).map(
    (e) => e.name,
  );
  assert.ok(
    !defaultExports.includes('language_probe_raw'),
    'default language export must be absent',
  );
  assert.ok(
    !defaultExports.includes('identity_probe_raw'),
    'default identity export must be absent',
  );
  for (const name of ['memory', 'alloc', 'free', 'call', 'probe_raw'])
    assert.ok(defaultExports.includes(name));
  const hostCalls = { now_ms: 0, random_u32: 0, progress: 0 };
  const env = Object.fromEntries(
    Object.keys(hostCalls).map((name) => [
      name,
      () => {
        hostCalls[name]++;
        throw new Error(`pure language fixture invoked ${name}`);
      },
    ]),
  );
  const { instance, module } = await WebAssembly.instantiate(wasm, { env });
  eq(
    WebAssembly.Module.imports(module)
      .map((i) => `${i.module}.${i.name}`)
      .sort(),
    ['env.now_ms', 'env.progress', 'env.random_u32'],
  );
  const { language_probe_raw: probe, free, memory } = instance.exports;
  eq(typeof probe, 'function');
  eq(typeof free, 'function');
  assert.ok(memory instanceof WebAssembly.Memory);
  const packed = probe();
  eq(typeof packed, 'bigint');
  const ptr = Number((packed >> 32n) & 0xffffffffn),
    len = Number(packed & 0xffffffffn);
  assert.ok(
    ptr > 0 &&
      len > 0 &&
      len <= 1024 * 1024 &&
      ptr + len <= memory.buffer.byteLength,
  );
  let actual;
  try {
    actual = Buffer.from(new Uint8Array(memory.buffer, ptr, len));
  } finally {
    free(ptr, len);
  }
  eq(hostCalls, { now_ms: 0, random_u32: 0, progress: 0 });
  writeFileSync(outputPath, actual);
  eq(actual, native, 'native and actual feature WASM trace bytes');
  const out = JSON.parse(
    new TextDecoder('utf-8', { fatal: true }).decode(actual),
  );
  validate(out);
  const corrupt = structuredClone(out);
  corrupt.boundaries[0].digest[0] ^= 1;
  writeFileSync(corruptPath, JSON.stringify(corrupt));
  const label = structuredClone(out);
  label.scans.numeric.components[0].name = 'corrupted-label';
  writeFileSync(`${corruptPath}.label.json`, JSON.stringify(label));
  console.log(
    `default-wasm sha256=${hash(production)} compiled/export-inspected language_probe_raw=absent; not instantiated`,
  );
  console.log(
    `feature-wasm sha256=${hash(wasm)} language_probe_raw=executed free=executed host-calls=${JSON.stringify(hostCalls)}`,
  );
  console.log(`native-output sha256=${hash(native)} bytes=${native.length}`);
  console.log(`wasm-output sha256=${hash(actual)} bytes=${actual.length}`);
  console.log(
    'PASS exact runtime bytes; independent source/utility/numeric/typed-transform/LP/SHA256/BigInt-scanner/route semantics',
  );
}
