import { ArrowDown } from 'lucide-react';

const STEPS = [
  {
    title: 'Specimen',
    detail: 'The studio saves its own wdbx.studio.v1 projection of the model.',
  },
  {
    title: 'Structural checks',
    detail: 'Schema, metadata and collection shapes (contracts.ts).',
  },
  {
    title: 'Migration',
    detail:
      'Kernel migrateIds: clamps settings, splits oversized nodes, repairs retired IDs.',
  },
  {
    title: 'Native validation',
    detail: 'Kernel validate: the same rules the desktop applies.',
  },
] as const;

/** Chapter 24: how this studio saves and restores a specimen. */
export function PersistenceDiagram({ desktop }: { desktop: boolean }) {
  const stores = [
    {
      id: 'browser',
      title: 'IndexedDB revisions',
      detail: 'Browser storage in this profile only.',
    },
    {
      id: 'desktop',
      title: 'WDBX v2 journal',
      detail: 'The studio/snapshot record in the desktop app’s store.',
    },
  ];
  const current = desktop ? 'desktop' : 'browser';
  return (
    <div className="grid gap-0.5">
      <ol
        aria-label="Restore checks"
        className="m-0 grid list-none gap-0.5 p-0"
      >
        {STEPS.map((step, i) => (
          <li key={step.title}>
            {i > 0 && (
              <ArrowDown
                aria-hidden="true"
                size={16}
                className="mx-auto my-0.5 text-muted-foreground"
              />
            )}
            <div className="rounded-lg border border-line bg-surface-2 px-3 py-2">
              <strong className="block text-sm text-ink">{step.title}</strong>
              <span className="text-xs text-ink-soft">{step.detail}</span>
            </div>
          </li>
        ))}
      </ol>
      <ArrowDown
        aria-hidden="true"
        size={16}
        className="mx-auto my-0.5 text-muted-foreground"
      />
      <ul
        aria-label="Storage"
        className="m-0 grid list-none gap-2 p-0 sm:grid-cols-2"
      >
        {stores.map((store) => (
          <li
            key={store.id}
            data-current={store.id === current}
            className="rounded-lg border border-line bg-surface-2 px-3 py-2 data-[current=true]:border-teal"
          >
            <strong className="flex items-center justify-between gap-2 text-sm text-ink">
              {store.title}
              {store.id === current && (
                <span className="rounded bg-teal-soft px-1.5 text-xs text-teal-strong">
                  This edition
                </span>
              )}
            </strong>
            <span className="text-xs text-ink-soft">{store.detail}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
