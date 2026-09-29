import { useId, type ReactNode } from 'react';
import { CodeBlock } from '@/components/wdbx';

/** Links to a chapter: a real URL, handled in-app unless modified-clicked. */
export function ChapterLink({
  chapter,
  onChapter,
  className,
  current,
  children,
}: {
  chapter: number;
  onChapter: (n: number) => void;
  className?: string;
  /** Marks the link as the page being read. */
  current?: boolean;
  children: ReactNode;
}) {
  return (
    // oxlint-disable-next-line next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation.
    <a
      href={`?view=specification&chapter=${chapter}`}
      className={className}
      aria-current={current ? 'page' : undefined}
      onClick={(e) => {
        if (!e.metaKey && !e.ctrlKey && !e.shiftKey) {
          e.preventDefault();
          onChapter(chapter);
        }
      }}
    >
      {children}
    </a>
  );
}

/**
 * A specification figure: the diagram, its caption, and the authoritative
 * figure text behind a disclosure.
 */
export function SpecFigure({
  id,
  title,
  caption,
  source,
  children,
}: {
  id: string;
  title: string;
  caption: ReactNode;
  source: string;
  children: ReactNode;
}) {
  const heading = useId();
  return (
    <figure
      aria-labelledby={heading}
      data-figure={id}
      className="spec-figure my-5 grid gap-3 rounded-xl border border-line bg-surface p-4"
    >
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 id={heading} className="m-0 text-sm font-semibold text-ink">
          {title}
        </h3>
        <span className="font-mono text-xs text-muted-foreground">
          Figure {id.replace('ch', '').replace('-', '.')}
        </span>
      </div>
      {children}
      <figcaption className="text-xs text-ink-soft">{caption}</figcaption>
      <details className="text-xs">
        <summary className="min-h-11 cursor-pointer content-center font-semibold text-ink-soft">
          Specification text
        </summary>
        <CodeBlock label={`${title}: specification text`}>{source}</CodeBlock>
      </details>
    </figure>
  );
}

/** Figures without a diagram: labelled, focusable preformatted text. */
export function CodeFigure({
  id,
  lang,
  code,
}: {
  id: string;
  lang: string;
  code: string;
}) {
  const label = `Figure ${id.replace('ch', '').replace('-', '.')}`;
  return (
    <figure data-figure={id} className="spec-figure my-4 grid gap-1.5">
      <figcaption className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-mono">{label}</span>
        <span className="rounded border border-line px-1.5 font-mono uppercase">
          {lang}
        </span>
      </figcaption>
      <CodeBlock label={`${label} (${lang})`} className="max-h-[32rem]">
        {code}
      </CodeBlock>
    </figure>
  );
}
