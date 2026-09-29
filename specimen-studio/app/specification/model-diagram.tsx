import { useMemo } from 'react';
import {
  entityLinks,
  parseEntityOutline,
  type Entity,
} from '@/lib/specimen/spec-figures';

const anchor = (figure: string, name: string) => `entity-${figure}-${name}`;

/**
 * Chapter 5 entity outlines as cards. A field type naming another entity in
 * the model links to that entity's card; `counts` annotates live fields.
 */
export function ModelDiagram({
  figure,
  code,
  known,
  counts = {},
}: {
  figure: string;
  code: string;
  /** Entity name -> the figure id that defines it. */
  known: ReadonlyMap<string, string>;
  counts?: Readonly<Record<string, number>>;
}) {
  const entities: Entity[] = useMemo(() => parseEntityOutline(code), [code]);
  const names = useMemo(() => new Set(known.keys()), [known]);
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {entities.map((entity) => (
        <section
          key={entity.name}
          id={anchor(figure, entity.name)}
          tabIndex={-1}
          aria-label={`${entity.name} entity`}
          className="rounded-lg border border-line bg-surface-2 p-3 focus-visible:outline-2 focus-visible:outline-focus target:border-teal"
        >
          <h4 className="m-0 mb-2 font-mono text-sm text-teal">
            {entity.name}
          </h4>
          <dl className="m-0 grid gap-1 text-xs">
            {entity.fields.map((field) => {
              const links = entityLinks(field.type, names);
              return (
                <div key={field.name} className="grid gap-0.5">
                  <dt className="flex flex-wrap items-baseline gap-1.5 font-mono text-ink">
                    {field.name}
                    {field.many && (
                      <span className="text-muted-foreground">[]</span>
                    )}
                    {field.optional && (
                      <span className="text-muted-foreground">optional</span>
                    )}
                    {counts[field.name] !== undefined && (
                      <span className="rounded bg-teal-soft px-1.5 text-teal-strong tabular-nums">
                        {counts[field.name]} live
                      </span>
                    )}
                  </dt>
                  {(field.type || field.note) && (
                    <dd className="m-0 text-ink-soft">
                      {field.type && (
                        <span className="font-mono">
                          {field.type}
                          {links.map((name) => (
                            <a
                              key={name}
                              href={`#${anchor(known.get(name)!, name)}`}
                              className="ml-1.5 inline-flex min-h-6 items-center font-sans font-semibold text-teal"
                              onClick={(e) => {
                                const target = document.getElementById(
                                  anchor(known.get(name)!, name),
                                );
                                if (!target) return;
                                e.preventDefault();
                                target.scrollIntoView({ block: 'nearest' });
                                target.focus({ preventScroll: true });
                              }}
                            >
                              Go to {name}
                            </a>
                          ))}
                        </span>
                      )}
                      {field.type && field.note && ' · '}
                      {field.note}
                    </dd>
                  )}
                </div>
              );
            })}
          </dl>
        </section>
      ))}
    </div>
  );
}
