import { Check, Circle, Loader2 } from 'lucide-react';
import { LIFECYCLE, reachedPhases } from '@/lib/specimen/spec-figures';
import type { TraceStep } from '@/lib/specimen/types';

/** Chapter 11: the kernel's cycle phases over `run_cycle`, lit by the trace. */
export function LifecycleDiagram({
  trace,
  busy,
}: {
  trace: TraceStep[];
  busy: boolean;
}) {
  const { done, active } = reachedPhases(trace, busy);
  const status = active
    ? `Running: ${active}`
    : done.length
      ? `Last cycle reached ${done.at(-1)}`
      : 'No cycle yet. Run a prompt in Studio to light the phases.';
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
              : done.includes(phase)
                ? 'done'
                : 'pending';
          return (
            <li
              key={phase}
              aria-current={state === 'active' ? 'step' : undefined}
              data-state={state}
              className="rounded-lg border border-line bg-surface-2 p-3 data-[state=active]:border-teal data-[state=done]:border-teal-soft"
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
