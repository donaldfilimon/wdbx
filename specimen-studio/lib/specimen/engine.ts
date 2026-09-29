import { isDesktop, runNative, nativeReview } from './native';
import { validateCollections } from './contracts';
import {
  clamp,
  COLORS,
  SUBSYSTEMS,
  uid,
  type Cycle,
  type Entry,
  type Resolution,
  type Resource,
  type Settings,
  type Specimen,
  type SpecimenNode,
  type TraceStep,
  type Visual,
  type Vote,
} from './types';

export const DEFAULT_SETTINGS: Settings = {
  voteThreshold: 62,
  jitter: 1.5,
  maxStrength: 10,
  initialStrength: 5,
  entryLimit: 20,
  scanLimit: 1000,
  fanoutLimit: 12,
  historyLimit: 100000,
  pinLimit: 100,
  maxNodes: 1000,
  maxRafts: 8,
  maxThreads: 1,
  chunkSize: 256,
  raftThreshold: 128,
  brainstorm: false,
  maintenance: false,
  adaptiveThreading: false,
  idleMin: 60,
  idleMax: 120,
  transientWidth: 32,
  seed: 104729,
};
const words: Record<string, string> = {
  zero: '0',
  one: '1',
  two: '2',
  three: '3',
  four: '4',
  five: '5',
  six: '6',
  seven: '7',
  eight: '8',
  nine: '9',
  ten: '10',
  plus: '+',
  minus: '-',
  times: '*',
  multiplied: '*',
  divided: '/',
  equals: '=',
  hi: 'hello',
  hey: 'hello',
  greetings: 'hello',
};
const now = () => new Date().toISOString();
export function normalize(text: string): string {
  return text
    .toLowerCase()
    .replace(/[“”]/g, '"')
    .replace(/’/g, "'")
    .replace(/\b(multiplied by|divided by)\b/g, (x) =>
      x.startsWith('multiplied') ? '*' : '/',
    )
    .replace(/\b\w+\b/g, (w) => words[w] ?? w)
    .replace(/\s+/g, ' ')
    .trim();
}
function tokens(text: string): string[] {
  return normalize(text).match(/&\w+|[\p{L}\p{N}_]+|[+*/^%=()-]/gu) ?? [];
}
function hash(text: string): string {
  let n = 2166136261;
  for (const c of text) {
    n = Math.imul(n ^ c.charCodeAt(0), 16777619);
  }
  return (n >>> 0).toString(36);
}
function arithmeticText(input: string): string | null {
  const normalized = normalize(input)
    .replace(
      /\bwhat(?: is|'s)|\bcalculate|\bcompute|\bsolve|\bplease|\bthe result of|\bequal to/g,
      '',
    )
    .replace(/[?=]/g, '')
    .trim();
  return /^[-+\d.\s()*/%^]+$/.test(normalized) &&
    /[+*/%^]|\d\s*-/.test(normalized)
    ? normalized
    : null;
}
export function bindShape(input: string): string {
  const s = normalize(input);
  if (s === '&n &op &n' || s.startsWith('&calc(') || arithmeticText(s))
    return 'arithmetic';
  if (
    /^say\s+.+\s+(?:\d+|&n)\s+(?:times|\*)$/.test(s) ||
    s === 'say &text &n *'
  )
    return 'repeat';
  if (/^(?:&imagine|imagine)(?:\b|\()/i.test(s)) return 'imagine';
  if (/\b(what time|what now|current time)\b/.test(s)) return 'time';
  if (/\b(recall|remember|memory)\b/.test(s)) return 'recall';
  if (/\b(don't|do not|never)\b/.test(s)) return 'negation';
  if (/\b(feel|feeling|happy|sad|stressed)\b/.test(s)) return 'tone';
  return tokens(s).join(' ');
}
export function identify(input: string): {
  id: string;
  resolution: Resolution;
  shape: string;
} {
  const shape = bindShape(input);
  const length = tokens(shape).length;
  const resolution: Resolution =
    length <= 6 ? 'low' : length <= 16 ? 'medium' : 'high';
  const named = [
    'arithmetic',
    'repeat',
    'imagine',
    'time',
    'recall',
    'negation',
    'tone',
    'hello',
  ].includes(shape);
  return {
    id: `${resolution}:${named ? shape : hash(shape)}`,
    resolution,
    shape,
  };
}
export function random(state: Specimen): number {
  let n = state.settings.seed | 0;
  n ^= n << 13;
  n ^= n >>> 17;
  n ^= n << 5;
  state.settings.seed = n >>> 0 || 1;
  return (state.settings.seed >>> 0) / 4294967296;
}
export function weighted<T>(
  items: T[],
  weight: (item: T) => number,
  rng: () => number,
): T | undefined {
  if (!items.length) return;
  const ws = items.map((x) => Math.max(0, weight(x)));
  const total = ws.reduce((a, b) => a + b, 0);
  if (total === 0)
    return items[Math.min(items.length - 1, Math.floor(rng() * items.length))];
  let p = rng() * total;
  for (let i = 0; i < items.length; i++) {
    p -= ws[i];
    if (p < 0) return items[i];
  }
  return items.at(-1);
}
export function confidence(
  base: number,
  node: SpecimenNode,
  settings: Settings,
  rng: () => number,
): number {
  const n = clamp(base, -100, 100);
  if (
    !node.jitter ||
    Math.abs(n) === 100 ||
    (node.strength === settings.maxStrength && n >= settings.voteThreshold)
  )
    return n;
  const magnitude = settings.jitter * (settings.brainstorm ? 3 : 1);
  return clamp(n + (rng() < 0.5 ? -1 : 1) * rng() * magnitude, -100, 100);
}
export function calculate(input: string): number {
  const text = arithmeticText(input) ?? normalize(input).trim();
  if (text.length > 2000)
    throw new Error('Calculation exceeds 2,000 characters.');
  const matches = text.match(/(?:\d+(?:\.\d*)?|\.\d+)|[()+\-*/%^]/g) ?? [];
  if (matches.join('') !== text.replace(/\s/g, ''))
    throw new Error('Use numbers, parentheses, and + − × ÷ % ^.');
  let index = 0,
    steps = 0;
  function expr(min = 0): number {
    if (++steps > 500) throw new Error('Calculation is too complex.');
    const t = matches[index++];
    let left: number;
    if (t === '(') {
      left = expr();
      if (matches[index++] !== ')')
        throw new Error('Close every opening parenthesis.');
    } else if (t === '-' || t === '+') left = (t === '-' ? -1 : 1) * expr(4);
    else if (t !== undefined && /^\d|^\./.test(t)) left = Number(t);
    else throw new Error('A number is missing from this expression.');
    while (index < matches.length) {
      const op = matches[index];
      const prec =
        op === '+' || op === '-'
          ? 1
          : op === '*' || op === '/' || op === '%'
            ? 2
            : op === '^'
              ? 3
              : 0;
      if (prec === 0 || prec < min) break;
      index++;
      const right = expr(prec + (op === '^' ? 0 : 1));
      if ((op === '/' || op === '%') && right === 0)
        throw new Error('Division by zero is undefined.');
      left =
        op === '+'
          ? left + right
          : op === '-'
            ? left - right
            : op === '*'
              ? left * right
              : op === '/'
                ? left / right
                : op === '%'
                  ? left % right
                  : left ** right;
      if (!Number.isFinite(left))
        throw new Error('The result is outside the supported numeric range.');
    }
    return left;
  }
  const value = expr();
  if (index !== matches.length)
    throw new Error('Check the calculation syntax.');
  return value;
}
export function runTape(code: string, input = '', budget = 10000): string {
  const ops = code.replace(/[^><+\-.,[\]]/g, '');
  const jumps = new Map<number, number>();
  const stack: number[] = [];
  for (let i = 0; i < ops.length; i++) {
    if (ops[i] === '[') stack.push(i);
    if (ops[i] === ']') {
      const j = stack.pop();
      if (j === undefined)
        throw new Error('Automaton has an unmatched bracket.');
      jumps.set(i, j);
      jumps.set(j, i);
    }
  }
  if (stack.length) throw new Error('Automaton has an unmatched bracket.');
  const tape = new Uint8Array(1024);
  let ptr = 0,
    pc = 0,
    steps = 0,
    read = 0,
    out = '';
  while (pc < ops.length) {
    if (++steps > budget)
      throw new Error('Automaton stopped at its instruction budget.');
    switch (ops[pc]) {
      case '>':
        ptr = (ptr + 1) % 1024;
        break;
      case '<':
        ptr = (ptr + 1023) % 1024;
        break;
      case '+':
        tape[ptr]++;
        break;
      case '-':
        tape[ptr]--;
        break;
      case '.':
        out += String.fromCharCode(tape[ptr]);
        break;
      case ',':
        tape[ptr] = input.charCodeAt(read++) || 0;
        break;
      case '[':
        if (!tape[ptr]) pc = jumps.get(pc)!;
        break;
      case ']':
        if (tape[ptr]) pc = jumps.get(pc)!;
        break;
    }
    pc++;
  }
  return out;
}
export function addNode(
  state: Specimen,
  input: {
    name: string;
    pattern: string;
    action: string;
    tone?: SpecimenNode['tone'];
    type?: SpecimenNode['type'];
    contextId?: string;
    entries?: Entry[];
  },
  ref?: string,
): Specimen {
  const next = structuredClone(state);
  const name = input.name.trim(),
    pattern = input.pattern.trim(),
    action = input.action.trim();
  if (!name || !pattern || !action)
    throw new Error('Enter a name, pattern, and action.');
  if (name.length > 100 || pattern.length > 2000 || action.length > 8000)
    throw new Error(
      'Keep names under 100, patterns under 2,000, and actions under 8,000 characters.',
    );
  const id = identify(pattern);
  if (
    next.nodes.some(
      (n) => n.ref !== ref && n.entries.some((e) => e.pattern === pattern),
    )
  )
    throw new Error(
      'That exact pattern is already stored. Use a distinct pattern or edit its node.',
    );
  if (
    next.mutations.some(
      (m) => m.originalPattern === pattern && m.originalAction === action,
    )
  )
    throw new Error(
      'This original pairing is inhibited by a previous mutation.',
    );
  if (!ref && next.nodes.length >= next.settings.maxNodes)
    throw new Error('The node limit is reached. Adjust the limit in Settings.');
  const old = next.nodes.find((n) => n.ref === ref);
  const node: SpecimenNode = {
    ref: old?.ref ?? uid(),
    name,
    patternId: id.id,
    resolution: id.resolution,
    type: input.type ?? 'pattern',
    contextId: input.contextId ?? '',
    strength: old?.strength ?? next.settings.initialStrength,
    jitter: old?.jitter ?? true,
    tone: input.tone ?? 'neutral',
    entries: [
      {
        id: old?.entries[0]?.id ?? uid(),
        pattern,
        alternatives: [
          {
            id: old?.entries[0]?.alternatives[0]?.id ?? uid(),
            action,
            inhibition: '',
            weight: 1,
            remixed: old?.entries[0]?.alternatives[0]?.remixed ?? false,
          },
        ],
      },
    ],
    createdAt: old?.createdAt ?? now(),
  };
  if (input.entries) node.entries = structuredClone(input.entries);
  else if (old) {
    node.entries[0].alternatives[0] = {
      ...old.entries[0].alternatives[0],
      action,
    };
    node.entries[0].alternatives.push(...old.entries[0].alternatives.slice(1));
    node.entries.push(...old.entries.slice(1));
  }
  node.patternId = identify(node.entries[0].pattern).id;
  node.resolution = identify(node.entries[0].pattern).resolution;
  if (node.entries.length > next.settings.entryLimit)
    throw new Error('This node exceeds its entry limit.');
  const originals = new Set(
    next.nodes
      .filter((n) => n.ref !== ref)
      .flatMap((n) => n.entries.map((e) => e.pattern)),
  );
  const slotIds = new Set<string>();
  for (const entry of node.entries) {
    if (
      !entry.pattern.trim() ||
      entry.pattern.length > 2000 ||
      originals.has(entry.pattern)
    )
      throw new Error('Original patterns must be nonempty and unique.');
    originals.add(entry.pattern);
    if (identify(entry.pattern).id !== node.patternId)
      throw new Error(
        'Every entry must share this node’s Pattern ID. Move a different pattern to a new node.',
      );
    if (!entry.alternatives.length || entry.alternatives.length > 100)
      throw new Error('Store between 1 and 100 alternatives per entry.');
    for (const a of entry.alternatives) {
      if (
        !a.action.trim() ||
        a.action.length > 8000 ||
        !Number.isFinite(a.weight) ||
        a.weight < 0 ||
        slotIds.has(a.id)
      )
        throw new Error(
          'Each vote needs a unique slot, action, and nonnegative finite weight.',
        );
      slotIds.add(a.id);
      if (
        next.mutations.some(
          (m) =>
            m.originalPattern === entry.pattern &&
            m.originalAction === a.action,
        )
      )
        throw new Error('A previous mutation inhibits this original pairing.');
    }
  }
  if (old) next.nodes[next.nodes.indexOf(old)] = node;
  else next.nodes.push(node);
  log(next, 'node', old ? 'Node updated' : 'Pattern learned', name);
  return next;
}
export function addEntry(
  state: Specimen,
  nodeRef: string,
  pattern: string,
  action: string,
): Specimen {
  const next = structuredClone(state),
    node = next.nodes.find((n) => n.ref === nodeRef);
  if (!node) throw new Error('Node not found.');
  if (node.entries.length >= next.settings.entryLimit)
    throw new Error('This node is full. Create a related node.');
  if (identify(pattern).id !== node.patternId)
    throw new Error('The entry must produce the same Pattern ID as its node.');
  if (next.nodes.some((n) => n.entries.some((e) => e.pattern === pattern)))
    throw new Error('This exact original pattern already exists.');
  node.entries.push({
    id: uid(),
    pattern,
    alternatives: [
      { id: uid(), action, inhibition: '', weight: 1, remixed: false },
    ],
  });
  log(next, 'node', 'Entry added', node.name);
  return next;
}
export function log(
  state: Specimen,
  type: string,
  title: string,
  detail: string,
): void {
  state.events.unshift({ id: uid(), type, title, detail, createdAt: now() });
  state.events = state.events.slice(0, 500);
  state.updatedAt = now();
}
export function removeNode(state: Specimen, ref: string): Specimen {
  const next = structuredClone(state),
    node = next.nodes.find((n) => n.ref === ref);
  if (!node) return next;
  next.nodes = next.nodes.filter((n) => n.ref !== ref);
  next.attachments = next.attachments.filter(
    (a) => a.from !== ref && a.to !== ref,
  );
  if (!next.nodes.some((n) => n.patternId === node.patternId))
    next.resources.forEach((r) => {
      if (r.resourceId === node.patternId) r.resourceId = '';
    });
  log(next, 'node', 'Node removed', node.name);
  return next;
}
export function saveResource(
  state: Specimen,
  input: Omit<Resource, 'ref' | 'patternId'>,
  ref?: string,
): Specimen {
  const next = structuredClone(state);
  if (
    !Number.isFinite(input.valence) ||
    Math.abs(input.valence) > 1 ||
    !Number.isFinite(input.intensity) ||
    input.intensity < 0 ||
    input.intensity > 1
  )
    throw new Error('Valence must be −1 to 1 and intensity 0 to 1.');
  if (!ref && next.resources.length >= 10000)
    throw new Error('The memory limit of 10,000 entries is reached.');
  if (!input.text.trim() || !input.value.trim())
    throw new Error('Enter a term and its supporting information.');
  if (input.text.length > 2000 || input.value.length > 12000)
    throw new Error('This memory entry is too long.');
  if (!SUBSYSTEMS.includes(input.subsystem as (typeof SUBSYSTEMS)[number]))
    throw new Error('Choose a registered subsystem.');
  if (
    input.resourceId &&
    !next.nodes.some((n) => n.patternId === input.resourceId)
  )
    throw new Error('The linked ID must belong to a stored node.');
  const resource = {
    ...input,
    ref: ref ?? uid(),
    patternId: identify(input.text).id,
  };
  const at = next.resources.findIndex((r) => r.ref === ref);
  if (at >= 0) next.resources[at] = resource;
  else next.resources.push(resource);
  log(next, 'memory', 'Memory saved', `${input.subsystem} · ${input.text}`);
  return next;
}
export function seedSpecimen(): Specimen {
  let s: Specimen = {
    schema: 'wdbx.studio.v1',
    name: 'Specimen 001',
    nodes: [],
    resources: [],
    attachments: [],
    history: [],
    events: [],
    mutations: [],
    proposals: [],
    atp: { valence: 0.2, intensity: 0.25, lastUpdate: 0 },
    settings: { ...DEFAULT_SETTINGS },
    updatedAt: now(),
  };
  const defaults = [
    ['Calculate', '&n &op &n', '&calc(&current_input)'],
    ['Greeting', 'hello', 'Hello. What would you like to explore?'],
    ['Repeat', 'say &text &n times', '&repeat(&current_input)'],
    ['Time', 'what time is it', '&time&'],
    ['Recall', 'what can you recall', '&recall(&current_input)'],
    ['Imagine', 'imagine &text', '&imagine(&current_input)'],
    ['Tone', 'how are you feeling', '&tone&'],
    ['Negation', "don't &text", 'I will leave that action alone.'],
  ];
  for (const [name, pattern, action] of defaults)
    s = addNode(s, {
      name,
      pattern,
      action,
      tone: name === 'Greeting' ? 'warm' : 'neutral',
    });
  s.nodes[0].strength = 6;
  const memories = [
    [
      'dictionary',
      'specimen',
      'A persistent collection of pattern nodes and supporting organs.',
    ],
    [
      'dictionary',
      'WDBX',
      'A system for retrieving patterns, collecting gated votes, and composing responses.',
    ],
    ['thesaurus', 'hello', 'hi, hey, greetings'],
    ['chargebook', 'happy', 'warm, pleased, content'],
    ['chargebook', 'stressed', 'tense, concerned'],
    ['negation', 'do not', 'Inhibit the scoped action rather than execute it.'],
    ['verbSemantics', 'give', 'subject, predicate, object, recipient'],
    ['morphology', 'running', 'run'],
    [
      'antiThesaurus',
      'race',
      'Use context to distinguish running from ethnicity.',
    ],
    [
      'conjunctiveRules',
      'and',
      'Combine independent, compatible contributions.',
    ],
    [
      'roleMapper',
      'subject predicate object',
      'Preserve who does what to whom.',
    ],
    [
      'sigils',
      '&current_input',
      'Read the original input; never execute it as arbitrary code.',
    ],
  ];
  for (const [subsystem, text, value] of memories)
    s.resources.push({
      ref: uid(),
      subsystem,
      text,
      value,
      patternId: identify(text).id,
      resourceId: subsystem === 'dictionary' ? s.nodes[4].patternId : '',
      valence: text === 'happy' ? 0.7 : 0,
      intensity: 0.4,
    });
  s.events = [];
  log(
    s,
    'system',
    'Starter specimen created',
    '8 pattern nodes and 12 supporting memories.',
  );
  return s;
}
const isLexicallyNegated = (text: string) =>
  /\b(don't|do not|never)\b/i.test(normalize(text));
type ScopedVariant = {
  input: string;
  negated: boolean[];
  ambiguous: boolean;
};
function scopedFanouts(input: string, state: Specimen): ScopedVariant[] {
  const original = input.split(';').map(isLexicallyNegated);
  const out = new Map<string, ScopedVariant>();
  const add = (text: string, negated: boolean[], ambiguous = false) => {
    // As with the original Set-based budget, the first admitted derivation
    // owns this text. Later cyclic rewrites must not change its source scopes.
    if (!out.has(text)) out.set(text, { input: text, negated, ambiguous });
  };
  add(input, original);
  add(normalize(input), original);
  for (const r of state.resources.filter((x) => x.subsystem === 'thesaurus')) {
    const family = [r.text, ...r.value.split(',').map((x) => x.trim())];
    for (const v of Array.from(out.values()))
      for (const term of family) {
        if (!term || term.length > 100) continue;
        if (v.input.toLowerCase().includes(term.toLowerCase()))
          for (const other of family) {
            const text = v.input.replace(
              new RegExp(escapeRegex(term), 'gi'),
              () => other,
            );
            // Replacements crossing or introducing delimiters have no reliable
            // source-clause correspondence, even when clause counts match.
            const ambiguous =
              v.ambiguous ||
              (text !== v.input && (term.includes(';') || other.includes(';')));
            const clauses = text.split(';');
            const inhibitAll =
              ambiguous &&
              (v.negated.some(Boolean) || clauses.some(isLexicallyNegated));
            add(
              text,
              clauses.map(
                (clause, i) =>
                  inhibitAll ||
                  v.negated[i] === true ||
                  isLexicallyNegated(clause),
              ),
              ambiguous,
            );
            if (out.size >= state.settings.fanoutLimit)
              return [...out.values()].slice(0, state.settings.fanoutLimit);
          }
      }
  }
  return [...out.values()].slice(0, state.settings.fanoutLimit);
}
export function fanouts(input: string, state: Specimen): string[] {
  return scopedFanouts(input, state).map((variant) => variant.input);
}
const escapeRegex = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
function matchesBind(pattern: string, input: string): boolean {
  if (pattern.includes('&automata(')) return false;
  if (/&[^\s]*\*/.test(pattern)) {
    const literal = pattern.match(/&([^\s]*\*)/)?.[1] ?? '';
    const needle = literal.replaceAll('*', '');
    return normalize(input).includes(needle);
  }
  if (normalize(pattern) === normalize(input)) return true;
  if (normalize(pattern) === '&n &op &n' && arithmeticText(input)) return true;
  const parts = normalize(pattern).split(/(&n|&op|&equals|&text)/g);
  if (parts.length === 1) return false;
  const expression = parts
    .map((p) =>
      p === '&n'
        ? '[-+]?\\d+(?:\\.\\d+)?'
        : p === '&op'
          ? '[+*/^%\\-]'
          : p === '&equals'
            ? '(?:=|equals)'
            : p === '&text'
              ? '.+?'
              : escapeRegex(p).replace(/ +/g, '\\s*'),
    )
    .join('');
  try {
    return new RegExp(`^${expression}$`, 'i').test(normalize(input));
  } catch {
    return false;
  }
}
export function deepScore(pattern: string, input: string): number {
  if (matchesBind(pattern, input)) return 100;
  const a = new Set(tokens(pattern)),
    b = new Set(tokens(input));
  const overlap = [...a].filter((t) => b.has(t)).length;
  const union = new Set([...a, ...b]).size;
  if (!union) return -100;
  return clamp((200 * overlap) / union - 100, -100, 100);
}
export class HybridTable<T> {
  private records: T[];
  private index: Map<string, T[]> | null;
  constructor(records: T[], key: (row: T) => string, threshold = 32) {
    this.records = records;
    this.index = records.length >= threshold ? new Map() : null;
    if (this.index)
      for (const row of records) {
        const id = key(row);
        this.index.set(id, [...(this.index.get(id) ?? []), row]);
      }
    this.key = key;
  }
  private key: (row: T) => string;
  lookup(id: string): T[] {
    return this.index
      ? (this.index.get(id) ?? [])
      : this.records.filter((x) => this.key(x) === id);
  }
  get mode() {
    return this.index ? 'hash' : 'array';
  }
}
export async function raftFindAll<T>(
  items: T[],
  predicate: (item: T) => boolean,
  settings: Settings,
  signal?: AbortSignal,
): Promise<T[]> {
  const out: T[] = [];
  for (let start = 0; start < items.length; start += settings.chunkSize) {
    const chunk = items.slice(start, start + settings.chunkSize);
    const count =
      items.length < settings.raftThreshold
        ? 1
        : Math.min(settings.maxRafts, chunk.length);
    const size = Math.ceil(chunk.length / count);
    const parts: T[][] = Array.from({ length: count }, () => []);
    await Promise.all(
      parts.map(async (part, r) => {
        for (
          let i = r * size;
          i < Math.min((r + 1) * size, chunk.length);
          i++
        ) {
          signal?.throwIfAborted();
          if (predicate(chunk[i])) part.push(chunk[i]);
          await new Promise<void>((resolve) => setTimeout(resolve, 0));
        }
      }),
    );
    out.push(...parts.flat());
  }
  return out;
}
function supporting(state: Specimen, vote: Vote): Resource[] {
  const node = state.nodes.find((n) => n.ref === vote.nodeRef);
  const keys = new Set([
    identify(vote.input).id,
    identify(vote.action).id,
    ...tokens(vote.input).map((w) => identify(w).id),
  ]);
  return state.resources
    .filter(
      (r) =>
        keys.has(r.patternId) ||
        (!!r.resourceId && r.resourceId === node?.patternId),
    )
    .filter(
      (r) =>
        r.resourceId === node?.patternId ||
        normalize(vote.input + ' ' + vote.action).includes(normalize(r.text)),
    );
}
function composeVisual(
  input: string,
  resources: Resource[],
  state: Specimen,
): Visual {
  const width = 320,
    height = 180,
    n = clamp(state.settings.transientWidth, 8, 128);
  const novel = resources.length === 0;
  const points: Visual = {
    xArray: [],
    yArray: [],
    colorArray: [],
    brightnessArray: [],
    size: { width, height },
    position: { x: 0, y: 0 },
  };
  const colors = ['#14786b', '#91c7a7', '#e5ad50', '#8174b7', '#b7d6db'];
  const base = parseInt(hash(input), 36);
  for (let i = 0; i < n; i++) {
    const feature = Math.sin(((base % 71) + i * 1.73) * 0.3);
    const relu = random(state) < (novel ? 0.75 : 0.25);
    const activation = relu
      ? Math.max(0, feature)
      : 1 / (1 + Math.exp(-feature));
    points.xArray.push(Math.cos(i * 2.399) * activation * 125);
    points.yArray.push(Math.sin(i * 2.399) * activation * 70);
    points.colorArray.push(colors[(i + base) % colors.length]);
    points.brightnessArray.push(0.45 + activation * 0.5);
  }
  return points;
}
function executeAction(
  vote: Vote,
  state: Specimen,
): { text: string; visual?: Visual; transforms: string[] } {
  const action = vote.action;
  const transforms = ['qualified vote'];
  if (action.includes('&calc(')) {
    const result = calculate(vote.input);
    return {
      text: new Intl.NumberFormat('en-US', {
        maximumFractionDigits: 10,
      }).format(result),
      transforms: [...transforms, 'bounded arithmetic parser'],
    };
  }
  if (action.includes('&repeat(')) {
    const m = normalize(vote.input).match(
      /^say\s+(.+)\s+(\d+)\s+(?:times|\*)[.!?]?$/,
    );
    if (!m) throw new Error('Try “say hello 3 times”.');
    const count = Number(m[2]);
    if (count < 1 || count > 100)
      throw new Error('Choose a repetition count from 1 to 100.');
    return {
      text: Array.from({ length: count }, () => m[1]).join(' '),
      transforms: [...transforms, 'repeat action'],
    };
  }
  if (action.includes('&time&'))
    return {
      text: new Intl.DateTimeFormat(undefined, {
        dateStyle: 'full',
        timeStyle: 'short',
      }).format(new Date()),
      transforms: [...transforms, 'current local time'],
    };
  if (action.includes('&recall(')) {
    const memories = supporting(state, vote).filter(
      (r) => r.subsystem === 'dictionary',
    );
    const pinned = state.history.filter((h) => h.pinned).slice(-3);
    const text = [
      ...memories.map((r) => `${r.text}: ${r.value}`),
      ...pinned.map(
        (h) =>
          `Pinned: ${h.input} → ${h.segments.map((x) => x.text).join(' ')}`,
      ),
    ].join('\n\n');
    return {
      text:
        text ||
        'No relevant memory is stored yet. Add a memory entry or pin a conversation.',
      transforms: [...transforms, 'Pattern-ID and linked-resource retrieval'],
    };
  }
  if (action.includes('&tone&'))
    return {
      text:
        state.atp.valence > 0.3
          ? 'The current tone is warm.'
          : state.atp.valence < -0.3
            ? 'The current tone is cautious.'
            : 'The current tone is calm and curious.',
      transforms: [...transforms, 'ATP state read'],
    };
  if (action.includes('&imagine(')) {
    const resources = supporting(state, vote);
    return {
      text: `A visual study of ${vote.input.replace(/^&?imagine\s*/i, '').replace(/[()]/g, '') || 'the current idea'}. ${resources.length ? `Composed with ${resources.length} retrieved resources.` : 'Composed from the prompt’s structural features.'}`,
      visual: composeVisual(vote.input, resources, state),
      transforms: [
        ...transforms,
        'fixed sparse feature synthesis',
        'ephemeral visual composition',
      ],
    };
  }
  if (action.startsWith('&automata(')) {
    const code = action.slice(10, -1);
    return {
      text: runTape(code, vote.input),
      transforms: [...transforms, 'bounded eight-primitive automaton'],
    };
  }
  let text = action.replaceAll('&current_input', vote.input);
  for (const r of supporting(state, vote)) {
    text = text.replaceAll(`&memory(${r.text})`, r.value);
  }
  return { text, transforms };
}
export async function runCycle(
  source: Specimen,
  input: string,
  onTrace?: (trace: TraceStep[]) => void,
  signal?: AbortSignal,
): Promise<{ state: Specimen; cycle: Cycle }> {
  if (isDesktop()) return runNative(source, input, onTrace, signal);
  if (!input.trim()) throw new Error('Enter a prompt first.');
  if (input.length > 8000)
    throw new Error('Keep prompts under 8,000 characters.');
  signal?.throwIfAborted();
  const state = structuredClone(source),
    start = performance.now();
  const cycle: Cycle = {
    id: uid(),
    input: input.trim(),
    createdAt: now(),
    segments: [],
    votes: [],
    trace: [],
    duration: 0,
    feedback: [],
    status: 'complete',
    pinned: false,
    supervisedUntil: Date.now() + 90000,
  };
  const step = (phase: string, detail: string, count?: number) => {
    cycle.trace.push({ id: uid(), phase, detail, count });
    onTrace?.([...cycle.trace]);
  };
  const clauses = input
    .split(';')
    .map((clause) => clause.trim())
    .filter(Boolean)
    .map((clause) => ({
      input: clause,
      negated: isLexicallyNegated(clause),
    }));
  const variants = scopedFanouts(input, state);
  const chunks = variants.flatMap((variant) =>
    variant.input.split(';').flatMap((clause, index) =>
      clause
        .split(/\s+(?:and then|then)\s+/i)
        .map((input) => input.trim())
        .filter(Boolean)
        .map((input) => ({
          input,
          negated: variant.negated[index],
        })),
    ),
  );
  step(
    'Prepare',
    `${variants.length} bounded variants; ${chunks.length} input chunks`,
    chunks.length,
  );
  const negatedClauseCount = clauses.filter((clause) => clause.negated).length;
  const negated = negatedClauseCount === clauses.length;
  if (negatedClauseCount > 0)
    step(
      'Scope',
      clauses.length > 1
        ? `Lexical negation scopes ${negatedClauseCount} of ${clauses.length} semicolon-delimited clauses; ${clauses.length - negatedClauseCount} independent ${clauses.length - negatedClauseCount === 1 ? 'clause remains' : 'clauses remain'} active`
        : 'Lexical negation conservatively scoped to the full prompt; no explicit semicolon boundary',
      negatedClauseCount,
    );
  const ambiguousVariants = variants.filter(
    (variant) => variant.ambiguous && variant.negated.some(Boolean),
  ).length;
  if (ambiguousVariants)
    step(
      'Scope',
      `${ambiguousVariants} fan-out variants have ambiguous clause correspondence; conservatively inhibit their action scopes`,
      ambiguousVariants,
    );
  const map = new HybridTable(
    state.nodes.filter((n) => n.type === 'pattern'),
    (n) => n.patternId,
  );
  const jobs: {
    node: SpecimenNode;
    entry: Entry;
    input: string;
    negated: boolean;
    group?: string;
    depth: number;
  }[] = [];
  const visited = new Set<string>();
  const enqueue = (
    node: SpecimenNode,
    chunk: string,
    negated: boolean,
    group?: string,
    depth = 0,
  ) => {
    if (depth > 4) return;
    for (const entry of node.entries) {
      const key = `${entry.id}:${chunk}:${negated}:${group ?? ''}`;
      if (!visited.has(key)) {
        visited.add(key);
        jobs.push({ node, entry, input: chunk, negated, group, depth });
      }
    }
  };
  for (const chunk of chunks) {
    const id = identify(chunk.input);
    const candidates = [
      ...map.lookup(id.id),
      ...state.nodes.filter(
        (n) =>
          n.type === 'pattern' &&
          n.entries.some(
            (e) =>
              /&[^\s]*\*/.test(e.pattern) ||
              matchesBind(e.pattern, chunk.input),
          ),
      ),
    ];
    for (const node of new Map(candidates.map((n) => [n.ref, n])).values())
      enqueue(node, chunk.input, chunk.negated);
  }
  step(
    'Retrieve',
    `${jobs.length} entry candidates retrieved using ${map.mode} indexes`,
    jobs.length,
  );
  const seenVotes = new Set<string>();
  let scans = 0;
  for (let i = 0; i < jobs.length; i++) {
    signal?.throwIfAborted();
    if (i % Math.min(state.settings.scanLimit, 128) === 0)
      await new Promise<void>((r) => setTimeout(r, 0));
    const job = jobs[i];
    scans++;
    if (job.negated && bindShape(job.entry.pattern) !== 'negation') continue;
    const base = deepScore(job.entry.pattern, job.input);
    const score = confidence(base, job.node, state.settings, () =>
      random(state),
    );
    if (score < state.settings.voteThreshold) continue;
    const alternative = weighted(
      job.entry.alternatives.filter(
        (a) =>
          !a.inhibition ||
          !normalize(job.input).includes(normalize(a.inhibition)),
      ),
      (a) => a.weight,
      () => random(state),
    );
    if (!alternative) continue;
    const signature = `${job.entry.id}:${alternative.id}:${normalize(job.input)}:${job.group ?? ''}`;
    if (seenVotes.has(signature)) continue;
    seenVotes.add(signature);
    const vote: Vote = {
      id: uid(),
      nodeRef: job.node.ref,
      entryRef: job.entry.id,
      action: alternative.action,
      base,
      confidence: score,
      strength: job.node.strength,
      input: job.input,
      group: job.group,
      resources: [],
    };
    vote.resources = supporting(state, vote).map((r) => r.ref);
    cycle.votes.push(vote);
    const patternTokens = new Set(tokens(job.entry.pattern));
    const remainder = tokens(job.input)
      .filter((t) => !patternTokens.has(t))
      .join(' ');
    if (remainder && remainder !== normalize(job.input))
      for (const a of state.attachments) {
        const target =
          a.from === job.node.ref
            ? a.to
            : a.bidirectional && a.to === job.node.ref
              ? a.from
              : null;
        if (target && (a.hard || random(state) < a.affinity)) {
          const node = state.nodes.find((n) => n.ref === target);
          if (node)
            enqueue(
              node,
              remainder,
              job.negated,
              job.group ?? a.id,
              job.depth + 1,
            );
        }
      }
  }
  step(
    'Deep scan',
    `${scans} high-resolution comparisons; global ceiling ${state.settings.scanLimit}`,
    scans,
  );
  // Equivalent fan-out actions share a conflict group; complementary actions remain independent.
  const groups = new Map<string, Vote[]>();
  for (const vote of cycle.votes) {
    const key = vote.group
      ? `attached:${vote.id}`
      : `${identify(vote.input).id}:${normalize(vote.action)}`;
    groups.set(key, [...(groups.get(key) ?? []), vote]);
  }
  cycle.votes = [...groups.values()].map((v) =>
    weighted(
      v,
      (x) => Math.max(0, x.confidence) * x.strength,
      () => random(state),
    )!,
  );
  step(
    'Vote',
    `${cycle.votes.length} qualified contributions; voter phase complete`,
    cycle.votes.length,
  );
  const allResources = new Set(cycle.votes.flatMap((v) => v.resources));
  step(
    'Enrich',
    `${allResources.size} relevant supporting resources; no global similarity scan`,
    allResources.size,
  );
  for (const vote of cycle.votes) {
    try {
      const result = executeAction(vote, state);
      cycle.segments.push({
        id: uid(),
        text: result.text,
        contributors: [vote.nodeRef],
        sourceVotes: [vote.id],
        transformations: result.transforms,
        color: COLORS[cycle.segments.length % COLORS.length],
      });
      if (result.visual) cycle.visual = result.visual;
    } catch (e) {
      cycle.segments.push({
        id: uid(),
        text: e instanceof Error ? e.message : 'The action could not complete.',
        contributors: [vote.nodeRef],
        sourceVotes: [vote.id],
        transformations: ['action rejected safely'],
        color: COLORS[cycle.segments.length % COLORS.length],
      });
    }
  }
  if (!cycle.segments.length) {
    cycle.status = negated ? 'inhibited' : 'unmatched';
    cycle.segments = [
      {
        id: uid(),
        text: negated
          ? 'That action is inhibited.'
          : 'No stored pattern qualified. Teach this prompt a response in the Node Library.',
        contributors: [],
        sourceVotes: [],
        transformations: ['empty-match fallback'],
        color: 'green',
      },
    ];
  } else if (negated) cycle.status = 'inhibited';
  const elapsed = Math.max(0, (Date.now() - state.atp.lastUpdate) / 1000);
  state.atp.valence *= Math.exp(-elapsed / 180);
  state.atp.intensity *= Math.exp(-elapsed / 180);
  for (const r of state.resources.filter(
    (r) =>
      r.subsystem === 'chargebook' &&
      normalize(input).includes(normalize(r.text)),
  )) {
    state.atp.valence = clamp(state.atp.valence + r.valence * 0.2, -1, 1);
    state.atp.intensity = clamp(state.atp.intensity + r.intensity * 0.2, 0, 1);
  }
  state.atp.lastUpdate = Date.now();
  signal?.throwIfAborted();
  step(
    'Compose',
    `${cycle.segments.length} output segments with contributor provenance`,
    cycle.segments.length,
  );
  cycle.duration = performance.now() - start;
  state.history.push(cycle);
  trimHistory(state);
  log(
    state,
    'cycle',
    cycle.status === 'unmatched' ? 'No qualifying pattern' : 'Cycle completed',
    `${input.slice(0, 120)} · ${cycle.votes.length} votes`,
  );
  return { state, cycle };
}
function trimHistory(state: Specimen) {
  const pinned = state.history.filter((x) => x.pinned);
  const unpinned = state.history
    .filter((x) => !x.pinned)
    .slice(-state.settings.historyLimit);
  state.history = [...pinned, ...unpinned].sort((a, b) =>
    a.createdAt.localeCompare(b.createdAt),
  );
}
export function feedback(
  source: Specimen,
  cycleId: string,
  right: boolean,
  color?: string,
): { state: Specimen; message: string } {
  let state = structuredClone(source);
  const cycle = state.history.find((c) => c.id === cycleId);
  if (!cycle) throw new Error('This answer is no longer available.');
  const event = color ?? 'all';
  if (cycle.feedback.includes(event) || cycle.feedback.includes('all'))
    throw new Error('Feedback was already applied to this answer or segment.');
  const previouslyRated = new Set(
    cycle.feedback.flatMap((c) =>
      cycle.segments
        .filter((s) => s.color === c)
        .flatMap((s) => s.contributors),
    ),
  );
  const refs = [
    ...new Set(
      cycle.segments
        .filter((s) => !color || s.color === color)
        .flatMap((s) => s.contributors),
    ),
  ];
  if (!refs.length) throw new Error('This segment has no contributing nodes.');
  if (refs.some((ref) => previouslyRated.has(ref)))
    throw new Error(
      'A contributor in this selection already received feedback for this cycle.',
    );
  let changed = 0;
  for (const ref of refs) {
    const node = state.nodes.find((n) => n.ref === ref);
    if (node && random(state) < 0.5) {
      node.strength = clamp(
        node.strength + (right ? 1 : -1),
        0,
        state.settings.maxStrength,
      );
      changed++;
    }
  }
  cycle.feedback.push(event);
  for (const node of state.nodes)
    if (node.strength === 0) state = removeNode(state, node.ref);
  const message = `${changed} of ${refs.length} contributors changed strength after independent coin flips.`;
  log(
    state,
    'feedback',
    right ? 'Positive feedback' : 'Negative feedback',
    message,
  );
  return { state, message };
}
export function togglePin(source: Specimen, id: string): Specimen {
  const s = structuredClone(source),
    c = s.history.find((x) => x.id === id);
  if (!c) return s;
  if (
    !c.pinned &&
    s.history.filter((x) => x.pinned).length >= s.settings.pinLimit
  )
    throw new Error('The pin limit is reached. Unpin a record first.');
  c.pinned = !c.pinned;
  trimHistory(s);
  log(
    s,
    'memory',
    c.pinned ? 'Conversation pinned' : 'Conversation unpinned',
    c.input,
  );
  return s;
}
export function maintenance(
  source: Specimen,
  mode?: 'phagy' | 'mutation',
): Specimen {
  const s = structuredClone(source);
  const action = mode ?? (random(s) < 0.5 ? 'mutation' : 'phagy');
  if (action === 'phagy') {
    const before = s.events.length;
    s.events = s.events.slice(0, 200);
    trimHistory(s);
    const refs = new Set(s.nodes.map((n) => n.ref));
    s.attachments = s.attachments.filter(
      (a) => refs.has(a.from) && refs.has(a.to),
    );
    log(
      s,
      'maintenance',
      'PHAGY completed',
      `One bounded cleanup job. ${Math.max(0, before - 200)} older activity entries retired; pins and mutation history preserved.`,
    );
  } else {
    const candidates = s.nodes.filter((n) => n.type === 'pattern');
    for (let i = candidates.length - 1; i > 0; i--) {
      const j = Math.floor(random(s) * (i + 1));
      [candidates[i], candidates[j]] = [candidates[j], candidates[i]];
    }
    candidates.length = Math.min(candidates.length, 8);
    let did = false;
    outer: for (const weak of candidates)
      for (const strong of candidates) {
        if (
          weak.strength >= strong.strength ||
          weak.ref === strong.ref ||
          s.mutations.some(
            (m) => m.recipient === strong.ref && m.donor === weak.ref,
          )
        )
          continue;
        for (const entry of weak.entries) {
          const donor = strong.entries.find(
            (e) => deepScore(e.pattern, entry.pattern) >= 80,
          );
          if (!donor) continue;
          const count = Math.min(
            entry.alternatives.length,
            donor.alternatives.length,
          );
          if (entry.alternatives.slice(count).some((a) => a.remixed)) continue;

          for (let i = 0; i < count; i++) {
            const a = entry.alternatives[i],
              b = donor.alternatives[i];
            if (
              a.remixed ||
              a.action.includes('&') ||
              b.action.includes('&') ||
              s.mutations.some(
                (m) => m.recipient === weak.ref && m.donor === strong.ref,
              )
            )
              continue;
            const original = a.action;
            const left = a.action.split(/(?<=[.!?])\s+/),
              right = b.action.split(/(?<=[.!?])\s+/);
            a.action = [left[0], right.at(-1)]
              .filter((v, j, arr) => arr.indexOf(v) === j)
              .join(' ');
            a.weight = random(s) < 0.5 ? a.weight : b.weight;
            a.remixed = true;
            entry.alternatives = entry.alternatives.slice(0, count);
            s.mutations.push({
              id: uid(),
              recipient: weak.ref,
              donor: strong.ref,
              slot: a.id,
              originalPattern: entry.pattern,
              originalAction: original,
              createdAt: now(),
            });
            log(
              s,
              'mutation',
              'Vote remixed',
              `${weak.name} borrowed a sentence transition from ${strong.name}; the original pairing is inhibited.`,
            );
            did = true;
            break outer;
          }
        }
      }
    if (!did)
      log(
        s,
        'maintenance',
        'Mutation scan completed',
        'No eligible weak/strong pairing. No learned content changed.',
      );
  }
  return s;
}
export function validateSpecimen(value: unknown): Specimen {
  if (!value || typeof value !== 'object')
    throw new Error('Choose a WDBX Studio JSON file.');
  const s = value as Specimen;
  if (s.schema !== 'wdbx.studio.v1')
    throw new Error('This file is not a supported WDBX Studio specimen.');
  if (
    typeof s.name !== 'string' ||
    s.name.length > 100 ||
    !s.settings ||
    !s.atp
  )
    throw new Error('Specimen metadata is missing.');
  for (const key of [
    'nodes',
    'resources',
    'attachments',
    'history',
    'events',
    'mutations',
    'proposals',
  ] as const)
    if (!Array.isArray(s[key]))
      throw new Error(`The ${key} collection is missing.`);
  validateSettings(s.settings);
  validateCollections(s);
  if (
    s.nodes.length > s.settings.maxNodes ||
    s.resources.length > 10000 ||
    s.history.length > s.settings.historyLimit + s.settings.pinLimit
  )
    throw new Error('A stored collection exceeds its limit.');
  const refs = new Set<string>(),
    patterns = new Set<string>();
  for (const n of s.nodes) {
    if (
      !n ||
      typeof n.ref !== 'string' ||
      refs.has(n.ref) ||
      typeof n.name !== 'string' ||
      !['pattern', 'A', 'B'].includes(n.type) ||
      !Array.isArray(n.entries) ||
      !n.entries.length ||
      n.entries.length > s.settings.entryLimit ||
      !Number.isFinite(n.strength) ||
      n.strength < 1 ||
      n.strength > s.settings.maxStrength
    )
      throw new Error('A node is invalid or repeated.');
    refs.add(n.ref);
    for (const e of n.entries) {
      if (
        typeof e.pattern !== 'string' ||
        e.pattern.length > 2000 ||
        patterns.has(e.pattern) ||
        identify(e.pattern).id !== n.patternId ||
        !Array.isArray(e.alternatives) ||
        !e.alternatives.length ||
        e.alternatives.length > 100
      )
        throw new Error(
          'A pattern entry is invalid, repeated, or indexed incorrectly.',
        );
      patterns.add(e.pattern);
      for (const a of e.alternatives)
        if (
          typeof a.action !== 'string' ||
          a.action.length > 8000 ||
          typeof a.inhibition !== 'string' ||
          typeof a.id !== 'string' ||
          typeof a.remixed !== 'boolean' ||
          !Number.isFinite(a.weight) ||
          a.weight < 0
        )
          throw new Error('A vote alternative is invalid.');
    }
  }
  for (const r of s.resources) {
    if (
      !r ||
      typeof r.ref !== 'string' ||
      typeof r.text !== 'string' ||
      typeof r.value !== 'string' ||
      r.value.length > 12000 ||
      identify(r.text).id !== r.patternId ||
      !SUBSYSTEMS.includes(r.subsystem as (typeof SUBSYSTEMS)[number]) ||
      (r.resourceId && !s.nodes.some((n) => n.patternId === r.resourceId)) ||
      !Number.isFinite(r.valence) ||
      Math.abs(r.valence) > 1 ||
      !Number.isFinite(r.intensity) ||
      r.intensity < 0 ||
      r.intensity > 1
    )
      throw new Error('A supporting memory is invalid.');
  }
  for (const a of s.attachments)
    if (
      !refs.has(a.from) ||
      !refs.has(a.to) ||
      a.from === a.to ||
      !Number.isFinite(a.affinity) ||
      a.affinity < 0 ||
      a.affinity > 1
    )
      throw new Error('An attachment has invalid endpoints or affinity.');
  for (const h of s.history) {
    if (
      typeof h.input !== 'string' ||
      !Array.isArray(h.segments) ||
      !Array.isArray(h.votes) ||
      !Array.isArray(h.trace) ||
      !Array.isArray(h.feedback) ||
      !['complete', 'unmatched', 'inhibited', 'cancelled'].includes(h.status) ||
      h.segments.some(
        (x) =>
          typeof x.text !== 'string' ||
          !Array.isArray(x.contributors) ||
          !Array.isArray(x.sourceVotes) ||
          !Array.isArray(x.transformations),
      )
    )
      throw new Error('Conversation history is malformed.');
    if (h.visual) {
      const v = h.visual;
      if (
        !Array.isArray(v.xArray) ||
        !Array.isArray(v.yArray) ||
        !Array.isArray(v.colorArray) ||
        !Array.isArray(v.brightnessArray) ||
        new Set([
          v.xArray.length,
          v.yArray.length,
          v.colorArray.length,
          v.brightnessArray.length,
        ]).size !== 1 ||
        v.xArray.length > 128 ||
        v.xArray.some((x) => !Number.isFinite(x)) ||
        v.yArray.some((x) => !Number.isFinite(x))
      )
        throw new Error('Visual arrays are invalid.');
    }
  }
  if (
    !Number.isFinite(s.atp.valence) ||
    Math.abs(s.atp.valence) > 1 ||
    !Number.isFinite(s.atp.intensity) ||
    s.atp.intensity < 0 ||
    s.atp.intensity > 1
  )
    throw new Error('ATP state is invalid.');
  return structuredClone(s);
}
export function validateSettings(s: Settings): void {
  const bounds: Partial<Record<keyof Settings, [number, number]>> = {
    voteThreshold: [-100, 100],
    jitter: [0, 10],
    maxStrength: [1, 100],
    initialStrength: [1, 100],
    entryLimit: [1, 100],
    scanLimit: [1, 10000],
    fanoutLimit: [1, 100],
    historyLimit: [1, 100000],
    pinLimit: [1, 1000],
    maxNodes: [1, 10000],
    maxRafts: [1, 32],
    maxThreads: [1, 1],
    chunkSize: [1, 10000],
    raftThreshold: [1, 10000],
    idleMin: [10, 3600],
    idleMax: [10, 7200],
    transientWidth: [8, 128],
    seed: [1, 4294967295],
  };
  for (const [key, bound] of Object.entries(bounds)) {
    const value = s[key as keyof Settings];
    if (
      typeof value !== 'number' ||
      !Number.isFinite(value) ||
      value < bound[0] ||
      value > bound[1]
    )
      throw new Error(`Set ${key} between ${bound[0]} and ${bound[1]}.`);
  }
  for (const key of [
    'maxStrength',
    'initialStrength',
    'entryLimit',
    'scanLimit',
    'fanoutLimit',
    'historyLimit',
    'pinLimit',
    'maxNodes',
    'maxRafts',
    'maxThreads',
    'chunkSize',
    'raftThreshold',
    'transientWidth',
    'seed',
  ] as const)
    if (!Number.isInteger(s[key]))
      throw new Error(`${key} must be a whole number.`);
  if (s.adaptiveThreading)
    throw new Error(
      'Adaptive OS threading is not supported by the browser profile.',
    );
  if (s.initialStrength > s.maxStrength)
    throw new Error('Initial strength cannot exceed maximum strength.');
  if (s.idleMin > s.idleMax)
    throw new Error('The minimum idle interval cannot exceed the maximum.');
  for (const key of ['brainstorm', 'maintenance', 'adaptiveThreading'] as const)
    if (typeof s[key] !== 'boolean')
      throw new Error(`${key} must be on or off.`);
}

export async function observeContext(
  source: Specimen,
  signal?: AbortSignal,
): Promise<Specimen> {
  if (isDesktop()) return nativeReview(source, 'review', undefined, signal);
  const state = structuredClone(source);
  const unmatched = await raftFindAll(
    state.history,
    (h) => h.status === 'unmatched',
    state.settings,
    signal,
  );
  const activeA = state.nodes.filter((n) => n.type === 'A');
  const activeB = state.nodes.filter((n) => n.type === 'B');
  const counts = new Map<string, number>();
  for (const h of unmatched)
    counts.set(h.input, (counts.get(h.input) ?? 0) + 1);
  for (const h of unmatched) {
    const supervisor = activeA.find(
      (n) =>
        (!n.contextId || normalize(h.input).includes(normalize(n.contextId))) &&
        deepScore(n.entries[0].pattern, h.input) >=
          state.settings.voteThreshold,
    );
    const idleObserver = activeB.find(
      (n) =>
        !n.contextId || normalize(h.input).includes(normalize(n.contextId)),
    );
    const eligibleA =
      supervisor && !h.reviewed && h.supervisedUntil >= Date.now();
    const eligibleB = idleObserver && (counts.get(h.input) ?? 0) >= 2;
    if (
      (eligibleA || eligibleB) &&
      !state.proposals.some((p) => p.pattern === h.input)
    ) {
      state.proposals.push({
        id: uid(),
        pattern: h.input,
        evidence: counts.get(h.input) ?? 1,
        status: 'pending',
      });
      log(
        state,
        'learning',
        eligibleA ? 'Residual review prepared' : 'Idle correlation prepared',
        h.input,
      );
    }
    if (eligibleA) h.reviewed = true;
  }
  log(
    state,
    'maintenance',
    'Context review completed',
    `${state.history.length} records traversed in chunks of ${state.settings.chunkSize}; up to ${state.settings.maxRafts} cooperative Index Rafts. Learning requires review.`,
  );
  return state;
}
