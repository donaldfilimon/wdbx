/**
 * Data behind the specification reader's interactive figures. Figure ids are
 * `ch<N>-<k>` as emitted by scripts/sync-specification.py; the Markdown stays
 * authoritative and every diagram also shows its source text.
 */
import type { TraceStep } from './types';

/** A specification chapter as emitted by scripts/sync-specification.py. */
export type SpecSegment =
  | { kind: 'html'; html: string }
  | { kind: 'figure'; id: string; lang: string; code: string };

export interface SpecChapter {
  number: number;
  title: string;
  text: string;
  words: number;
  segments: SpecSegment[];
}

/** Figures rendered as interactive diagrams rather than code figures. */
export const DIAGRAMS = [
  'ch2-1',
  'ch5-1',
  'ch5-4',
  'ch5-5',
  'ch11-1',
  'ch19-1',
  'ch20-2',
  'ch24-1',
] as const;

export interface EntityField {
  name: string;
  type?: string;
  note?: string;
  optional: boolean;
  many: boolean;
}

export interface Entity {
  name: string;
  fields: EntityField[];
}

/**
 * Parses the chapter 5 outline notation: an unindented entity name, then
 * indented `field[?][[]][: type]   # note` lines; blank lines separate entities.
 */
export function parseEntityOutline(code: string): Entity[] {
  const entities: Entity[] = [];
  for (const line of code.split('\n')) {
    if (!line.trim()) continue;
    if (!/^\s/.test(line)) {
      entities.push({ name: line.trim(), fields: [] });
      continue;
    }
    const current = entities.at(-1);
    if (!current) continue;
    const [decl, ...rest] = line.split('#');
    const note = rest.join('#').trim() || undefined;
    const colon = decl.indexOf(':');
    let name = (colon < 0 ? decl : decl.slice(0, colon)).trim();
    const type = colon < 0 ? undefined : decl.slice(colon + 1).trim();
    const many = name.endsWith('[]');
    if (many) name = name.slice(0, -2);
    const optional = name.endsWith('?');
    if (optional) name = name.slice(0, -1);
    const field: EntityField = { name, optional, many };
    if (type) field.type = type;
    if (note) field.note = note;
    current.fields.push(field);
  }
  return entities;
}

/** Entity names from `known` that appear as identifiers inside `type`. */
export function entityLinks(
  type: string | undefined,
  known: ReadonlySet<string>,
): string[] {
  if (!type) return [];
  return [...new Set(type.match(/[A-Za-z_]\w*/g) ?? [])].filter((t) =>
    known.has(t),
  );
}

export interface PipelineStage {
  label: string;
  chapter: number;
  /** Stages after the vote/composition phase boundary. */
  after?: boolean;
}

/** Chapter 2's pipeline, top to bottom, each stage linked to its chapter. */
export const PIPELINE: readonly PipelineStage[] = [
  { label: 'User input / permitted internal stimulus', chapter: 7 },
  { label: 'Language preprocessing and bounded fan-out', chapter: 7 },
  { label: 'Relational triples and scoped bindings', chapter: 8 },
  { label: 'Pattern-ID classification and retrieval', chapter: 6 },
  { label: 'High-resolution deep scans and gates', chapter: 20 },
  { label: 'Temporary qualified vote list', chapter: 9 },
  { label: 'Ephemeral JIT orchestrator', chapter: 11, after: true },
  { label: 'Contextual composition', chapter: 11, after: true },
  { label: 'Transient Memory when needed', chapter: 13, after: true },
  {
    label: 'Response / actions + contributor provenance',
    chapter: 18,
    after: true,
  },
  {
    label: 'Feedback and residual Type A supervision',
    chapter: 17,
    after: true,
  },
];

/** The orchestrator's parallel inputs in chapter 2 (fan-out then fan-in). */
export const ORCHESTRATOR_INPUTS: readonly PipelineStage[] = [
  { label: 'Side data', chapter: 12 },
  { label: 'ATP', chapter: 14 },
  { label: 'Scoped scripts', chapter: 8 },
  { label: 'History', chapter: 15 },
];

export interface LifecyclePhase {
  phase: string;
  /** Chapter 11 pseudocode calls this kernel phase covers. */
  calls: string[];
}

/** The kernel's trace phases mapped onto chapter 11's `run_cycle`. */
export const LIFECYCLE: readonly LifecyclePhase[] = [
  {
    phase: 'Prepare',
    calls: ['begin_cycle', 'prepare_variants_triples_and_bindings'],
  },
  { phase: 'Retrieve', calls: ['retrieve_all_compatible_candidates'] },
  { phase: 'Index Rafts', calls: ['enqueue_deep_scans'] },
  {
    phase: 'Deep scan',
    calls: [
      'include_bounded_attachment_handoffs',
      'drain_qualified_votes_for_admitted_work',
    ],
  },
  {
    phase: 'Vote',
    calls: [
      'finish_voter_phase_or_report_partial',
      'arbitrate_redundant_unattached_alternatives',
    ],
  },
  {
    phase: 'Compose',
    calls: [
      'retrieve_support',
      'filter_contextually',
      'compose_steps_and_constraints',
      'resolve_permitted_sigils',
      'apply_bounded_tonally_valid_variation',
      'finalize_output_and_provenance',
    ],
  },
];

/**
 * Which lifecycle phases a trace reached. While `running`, the last reached
 * phase is active rather than done.
 */
export function reachedPhases(
  trace: Pick<TraceStep, 'phase'>[],
  running: boolean,
): { done: string[]; active: string | undefined } {
  const order = LIFECYCLE.map((p) => p.phase);
  const seen = order.filter((p) => trace.some((s) => s.phase === p));
  if (running && seen.length)
    return { done: seen.slice(0, -1), active: seen.at(-1) };
  return { done: seen, active: undefined };
}
