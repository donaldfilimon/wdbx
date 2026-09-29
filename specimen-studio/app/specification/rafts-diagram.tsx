import { useId, useState } from 'react';
import { raftPlan, type RaftChunk } from '@/lib/specimen/kernel';
import type { Settings } from '@/lib/specimen/types';

const SHOWN = 12;
const MAX_CANDIDATES = 100_000;

/**
 * Chapter 20: how a scan of N candidates is checkpointed and split into
 * rafts, computed by the kernel with this specimen's settings.
 */
export function RaftsDiagram({ settings }: { settings: Settings }) {
  const inputId = useId();
  const [count, setCount] = useState(1000);
  const workers = Math.min(settings.maxRafts, settings.scanLimit);
  let plan: RaftChunk[] = [];
  let failure = '';
  try {
    plan = raftPlan(count, settings.chunkSize, workers);
  } catch (err) {
    failure = err instanceof Error ? err.message : String(err);
  }
  const rafts = plan.reduce((n, c) => Math.max(n, c.rafts.length), 0);
  const summary = failure
    ? failure
    : `${count.toLocaleString()} candidates: ${plan.length} checkpoint${plan.length === 1 ? '' : 's'} of up to ${Math.min(settings.chunkSize, 4096)}, each split across up to ${rafts} raft${rafts === 1 ? '' : 's'}.`;
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
            value={count}
            onChange={(e) => {
              const n = Math.trunc(Number(e.target.value));
              setCount(
                Number.isFinite(n)
                  ? Math.max(0, Math.min(MAX_CANDIDATES, n))
                  : 0,
              );
            }}
            className="min-h-11 w-36 rounded-md border border-line bg-surface-2 px-2 font-mono text-ink"
          />
        </label>
        <span className="text-ink-soft">
          chunkSize {settings.chunkSize} · maxRafts {settings.maxRafts} ·
          scanLimit {settings.scanLimit}
        </span>
      </div>
      <output className="text-sm text-ink">{summary}</output>
      {plan.length > 0 && (
        <ol
          aria-label="Checkpoint chunks"
          className="m-0 grid list-none gap-1.5 p-0"
        >
          {plan.slice(0, SHOWN).map((chunk, i) => (
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
      {plan.length > SHOWN && (
        <p className="m-0 text-xs text-muted-foreground">
          …and {plan.length - SHOWN} more checkpoints.
        </p>
      )}
      {plan.length > 0 && (
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
              <caption className="sr-only">
                Raft plan: checkpoints and raft ranges
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
                {plan.slice(0, 200).map((chunk, i) => (
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
