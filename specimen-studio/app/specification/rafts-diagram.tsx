import { useId, useMemo, useState } from 'react';
import {
  kernelLoadError,
  raftPlan,
  type RaftPlan,
} from '@/lib/specimen/kernel';
import type { Settings } from '@/lib/specimen/types';
import { useKernelStatus } from '@/lib/specimen/use-kernel-ready';

const SHOWN = 12;
const TABLE_ROWS = 200;
const MAX_CANDIDATES = 100_000;

/**
 * Chapter 20: how a scan of N candidates is checkpointed and split into
 * rafts, computed by the kernel with this specimen's settings.
 */
export function RaftsDiagram({ settings }: { settings: Settings }) {
  const inputId = useId();
  // The field keeps what was typed; the plan uses the last valid count.
  const [text, setText] = useState('1000');
  const parsed = Number(text);
  const valid =
    text.trim() !== '' &&
    Number.isInteger(parsed) &&
    parsed >= 0 &&
    parsed <= MAX_CANDIDATES;
  const [count, setCount] = useState(1000);
  const workers = Math.min(settings.maxRafts, settings.scanLimit);
  const status = useKernelStatus();
  // Only a bounded prefix is built, and only when an input changes.
  const result = useMemo((): { plan: RaftPlan } | { failure: string } => {
    if (status === 'loading') return { failure: 'Loading the kernel…' };
    if (status === 'failed')
      return { failure: `The kernel failed to load: ${kernelLoadError()}` };
    try {
      return {
        plan: raftPlan(count, settings.chunkSize, workers, TABLE_ROWS),
      };
    } catch (err) {
      return { failure: err instanceof Error ? err.message : String(err) };
    }
  }, [status, count, settings.chunkSize, workers]);
  const plan = 'plan' in result ? result.plan : null;
  const chunks = plan?.chunks ?? [];
  const summary = !plan
    ? (result as { failure: string }).failure
    : `${count.toLocaleString()} candidates: ${plan.checkpoints} checkpoint${plan.checkpoints === 1 ? '' : 's'} of up to ${plan.chunkSize}, each split across up to ${plan.maxRafts} raft${plan.maxRafts === 1 ? '' : 's'}.`;
  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-end gap-3 text-xs">
        <label htmlFor={inputId} className="grid gap-1 font-semibold text-ink">
          Candidates (N)
          <input
            id={inputId}
            type="number"
            min={0}
            max={MAX_CANDIDATES}
            step={1}
            value={text}
            aria-invalid={!valid}
            aria-describedby={`${inputId}-hint`}
            onChange={(e) => {
              setText(e.target.value);
              const n = Number(e.target.value);
              if (
                e.target.value.trim() !== '' &&
                Number.isInteger(n) &&
                n >= 0 &&
                n <= MAX_CANDIDATES
              )
                setCount(n);
            }}
            className="min-h-11 w-36 rounded-md border border-line bg-surface-2 px-2 font-mono text-ink"
          />
        </label>
        <span id={`${inputId}-hint`} className="text-ink-soft">
          {!valid &&
            `Enter a whole number from 0 to ${MAX_CANDIDATES.toLocaleString()}. `}
          chunkSize {settings.chunkSize} · maxRafts {settings.maxRafts} ·
          scanLimit {settings.scanLimit}
        </span>
      </div>
      <output className="text-sm text-ink">{summary}</output>
      {chunks.length > 0 && (
        <ol
          aria-label="Checkpoint chunks"
          className="m-0 grid list-none gap-1.5 p-0"
        >
          {chunks.slice(0, SHOWN).map((chunk, i) => (
            <li
              key={chunk.start}
              className="grid grid-cols-[5.5rem_1fr] items-center gap-2"
            >
              <span className="font-mono text-[11px] text-muted-foreground tabular-nums">
                #{i + 1} {chunk.start}–{chunk.end - 1}
              </span>
              <div
                aria-hidden="true"
                className="flex h-5 gap-px overflow-hidden rounded"
              >
                {chunk.rafts.map(([s, e], r) => (
                  <span
                    key={s}
                    className={r % 2 ? 'bg-teal-soft' : 'bg-teal'}
                    style={{ flexGrow: e - s }}
                  />
                ))}
              </div>
            </li>
          ))}
        </ol>
      )}
      {plan && plan.checkpoints > SHOWN && (
        <p className="m-0 text-xs text-muted-foreground">
          …and {(plan.checkpoints - SHOWN).toLocaleString()} more checkpoints.
        </p>
      )}
      {chunks.length > 0 && (
        <details className="text-xs">
          <summary className="min-h-11 cursor-pointer content-center font-semibold text-ink-soft">
            Show plan
          </summary>
          <section
            // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- axe requires scrollable regions to be keyboard-focusable.
            tabIndex={0}
            aria-label="Raft plan table"
            className="max-h-72 overflow-auto"
          >
            <table className="w-full border-collapse font-mono">
              <caption className="text-left text-muted-foreground">
                {plan && plan.checkpoints > chunks.length
                  ? `First ${chunks.length} of ${plan.checkpoints.toLocaleString()} checkpoints`
                  : 'Raft plan: checkpoints and raft ranges'}
              </caption>
              <thead>
                <tr>
                  <th scope="col" className="p-1 text-left">
                    Checkpoint
                  </th>
                  <th scope="col" className="p-1 text-left">
                    Candidates
                  </th>
                  <th scope="col" className="p-1 text-left">
                    Rafts
                  </th>
                </tr>
              </thead>
              <tbody>
                {chunks.map((chunk, i) => (
                  <tr key={chunk.start} className="border-t border-line">
                    <td className="p-1">{i + 1}</td>
                    <td className="p-1">
                      {chunk.start}–{chunk.end - 1}
                    </td>
                    <td className="p-1">
                      {chunk.rafts.map(([s, e]) => `${s}–${e - 1}`).join(', ')}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        </details>
      )}
    </div>
  );
}
