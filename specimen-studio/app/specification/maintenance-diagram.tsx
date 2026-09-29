import type { Specimen } from '@/lib/specimen/types';

/** Chapter 19: the idle coin flip between vote mutation and PHAGY cleanup. */
export function MaintenanceDiagram({ specimen }: { specimen: Specimen }) {
  const phagy = specimen.events.filter(
    (e) => e.type === 'maintenance' && e.title === 'PHAGY completed',
  ).length;
  const branches = [
    {
      side: 'Heads',
      title: 'Bounded vote mutation-tag event',
      detail: 'A Markov remix of one vote slot, recorded with its donor.',
      count: specimen.mutations.length,
      unit: 'mutations recorded',
    },
    {
      side: 'Tails',
      title: 'PHAGY cleanup event',
      detail: 'Expired derived references and retention limits are trimmed.',
      count: phagy,
      unit: 'PHAGY passes in the event log',
    },
  ];
  return (
    <div className="grid gap-3">
      <p className="m-0 mx-auto rounded-full border border-line bg-surface-2 px-4 py-1.5 text-center text-sm font-semibold text-ink">
        Idle maintenance: coin flip
      </p>
      <ul className="m-0 grid list-none gap-3 p-0 sm:grid-cols-2">
        {branches.map((b) => (
          <li
            key={b.side}
            className="grid gap-1 rounded-lg border border-line bg-surface-2 p-3"
          >
            <span className="font-mono text-xs text-amber uppercase">
              {b.side}
            </span>
            <strong className="text-sm text-ink">{b.title}</strong>
            <span className="text-xs text-ink-soft">{b.detail}</span>
            <span className="text-xs text-ink">
              <strong className="tabular-nums">{b.count}</strong> {b.unit}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
