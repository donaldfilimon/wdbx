/**
 * Pure shaping of engine data for the live views: no React, no kernel calls,
 * so each view shows exactly what the kernel recorded.
 */
import type { Cycle, Specimen, TraceStep } from './types';

export interface PhaseView {
  phase: string;
  detail: string;
  count?: number;
  /** Index Rafts: covered / total of the last checkpoint (0-1). */
  coverage?: number;
  checkpoints: number;
  state: 'done' | 'active';
}

type RaftStep = TraceStep & { covered?: number; total?: number };

/** One row per phase; repeated Index Rafts checkpoints fold into coverage. */
export function phaseTimeline(
  steps: TraceStep[],
  running = false,
): PhaseView[] {
  const out: PhaseView[] = [];
  for (const raw of steps) {
    const step = raw as RaftStep;
    const last = out.at(-1);
    if (last && last.phase === step.phase) {
      if (step.total) {
        last.coverage = (step.covered ?? 0) / step.total;
        last.checkpoints += 1;
      }
      if (step.count !== undefined) last.count = step.count;
      last.detail = step.detail || last.detail;
      continue;
    }
    out.push({
      phase: step.phase,
      detail: step.detail,
      count: step.count,
      coverage: step.total ? (step.covered ?? 0) / step.total : undefined,
      checkpoints: step.total ? 1 : 0,
      state: 'done',
    });
  }
  if (running && out.length) out[out.length - 1].state = 'active';
  return out;
}

export interface VoteBar {
  node: string;
  confidence: number;
  strength: number;
}

export function voteBreakdown(
  cycle: Pick<Cycle, 'votes'> | undefined,
  nodes: Pick<Specimen['nodes'][number], 'ref' | 'name'>[],
): VoteBar[] {
  if (!cycle) return [];
  return cycle.votes
    .map((v) => ({
      node: nodes.find((n) => n.ref === v.nodeRef)?.name ?? 'Removed node',
      confidence: v.confidence,
      strength: v.strength,
    }))
    .sort((a, b) => b.confidence - a.confidence);
}

export interface CyclePoint {
  id: string;
  input: string;
  createdAt: string;
  confidence: number | null;
  duration: number;
  matched: number;
  status: Cycle['status'];
}

/** The last `limit` cycles, oldest first. */
export function cycleSeries(
  history: Pick<
    Cycle,
    'id' | 'input' | 'createdAt' | 'duration' | 'status' | 'votes'
  >[],
  limit = 50,
): CyclePoint[] {
  return history.slice(-limit).map((c) => ({
    id: c.id,
    input: c.input,
    createdAt: c.createdAt,
    confidence: c.votes.length
      ? Math.max(...c.votes.map((v) => v.confidence))
      : null,
    duration: c.duration,
    matched: c.status === 'complete' ? 1 : 0,
    status: c.status,
  }));
}

/** ATP as the kernel will see it at `now` (exp(-elapsed s / 180)). */
export function atpNow(atp: Specimen['atp'], now: number) {
  const elapsed = Math.max(0, (now - atp.lastUpdate) / 1000);
  const factor = Math.exp(-elapsed / 180);
  return { valence: atp.valence * factor, intensity: atp.intensity * factor };
}
