import { Check, Circle, CircleX, Loader2 } from 'lucide-react';
import {
  LIFECYCLE,
  lifecycleState,
  type RunOutcome,
} from '@/lib/specimen/spec-figures';
import type { TraceStep } from '@/lib/specimen/types';

/** Chapter 11: the kernel's cycle phases over `run_cycle`, lit by the trace. */
export function LifecycleDiagram({
  trace,
  outcome,
}: {
  trace: TraceStep[];
  outcome: RunOutcome;
}) {
  const { done, active, stopped, status } = lifecycleState(trace, outcome);
  return (
    <div className="grid gap-3">
      <output className="text-xs font-semibold text-ink-soft">{status}</output>
      <ol
        aria-label="Cycle lifecycle phases"
        className="m-0 grid list-none gap-2 p-0 sm:grid-cols-3"
      >
        {LIFECYCLE.map(({ phase, calls }) => {
          const state =
            phase === active
              ? 'active'
              : phase === stopped
                ? 'stopped'
                : done.includes(phase)
                  ? 'done'
                  : 'pending';
          return (
            <li
              key={phase}
              aria-current={state === 'active' ? 'step' : undefined}
              data-state={state}
              className="rounded-lg border border-line bg-surface-2 p-3 data-[state=active]:border-teal data-[state=done]:border-line-strong data-[state=stopped]:border-danger"
            >
              <div className="mb-1.5 flex items-center gap-2 text-sm font-semibold text-ink">
                {state === 'done' ? (
                  <Check aria-hidden="true" size={15} className="text-teal" />
                ) : state === 'active' ? (
                  <Loader2
                    aria-hidden="true"
                    size={15}
                    className="text-teal motion-safe:animate-spin"
                  />
                ) : state === 'stopped' ? (
                  <CircleX
                    aria-hidden="true"
                    size={15}
                    className="text-danger"
                  />
                ) : (
                  <Circle
                    aria-hidden="true"
                    size={15}
                    className="text-muted-foreground"
                  />
                )}
                {phase}
                <span className="sr-only">
                  {state === 'done'
                    ? ' (reached)'
                    : state === 'active'
                      ? ' (running)'
                      : state === 'stopped'
                        ? ' (stopped here)'
                        : ' (not reached)'}
                </span>
              </div>
              <ul className="m-0 grid list-none gap-0.5 p-0 font-mono text-[11px] break-all text-ink-soft">
                {calls.map((call) => (
                  <li key={call}>{call}()</li>
                ))}
              </ul>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
