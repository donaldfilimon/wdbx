import sectionsJson from '@/lib/specification.json';
import type { SpecChapter } from '@/lib/specimen/spec-figures';
import type { Specimen, TraceStep } from '@/lib/specimen/types';
import { cn } from '@/lib/utils';
import { SpecFigureView } from '../specification/figures';
import { ChapterLink } from '../specification/figure';
import { SearchField } from './common';

const sections = sectionsJson as SpecChapter[];

/** Live studio state the specification's diagrams can reflect. */
export interface SpecLive {
  specimen: Specimen;
  trace: TraceStep[];
  busy: boolean;
  desktop: boolean;
}

const navLink =
  'flex min-h-11 items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm text-ink-soft no-underline hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-focus aria-[current=page]:bg-teal-soft aria-[current=page]:font-semibold aria-[current=page]:text-teal-strong';
const pageLink =
  'inline-flex min-h-11 max-w-[48%] items-center rounded-lg border border-line px-3 text-sm font-semibold text-teal no-underline hover:border-teal focus-visible:outline-2 focus-visible:outline-focus';

export function Specification({
  chapter,
  query,
  setQuery,
  onChapter,
  live,
}: {
  chapter: number;
  query: string;
  setQuery: (v: string) => void;
  onChapter: (n: number) => void;
  live: SpecLive;
}) {
  const current = sections.find((s) => s.number === chapter) ?? sections[0];
  const filtered = sections.filter((s) =>
    `${s.title} ${s.text}`.toLowerCase().includes(query.toLowerCase()),
  );
  return (
    <div className="grid gap-6 md:grid-cols-[minmax(13rem,17rem)_minmax(0,1fr)]">
      <aside className="grid content-start gap-2 md:sticky md:top-4 md:max-h-[calc(100dvh-8rem)] md:overflow-auto">
        <SearchField
          value={query}
          onChange={setQuery}
          placeholder="Search specification…"
        />
        <span className="chapter-count px-1 text-xs font-semibold text-muted-foreground">
          {filtered.length} chapters
        </span>
        <nav aria-label="Specification chapters" className="grid gap-0.5">
          {filtered.map((s) => (
            <ChapterLink
              key={s.number}
              chapter={s.number}
              onChapter={onChapter}
              className={navLink}
              current={s.number === chapter}
            >
              <span className="font-mono text-xs text-muted-foreground">
                {String(s.number).padStart(2, '0')}
              </span>
              {s.title}
            </ChapterLink>
          ))}
        </nav>
        {!filtered.length && (
          <p className="px-1 text-sm text-muted-foreground">
            No chapters match that search.
          </p>
        )}
      </aside>
      <article className="min-w-0 max-w-3xl">
        <div className="flex gap-3 font-mono text-xs text-muted-foreground">
          <span>{String(current.number).padStart(2, '0')} / 26</span>
          <span>{Math.ceil(current.words / 230)} min read</span>
        </div>
        <h2 className="mt-1 mb-4 text-2xl font-semibold text-ink">
          {current.title}
        </h2>
        <div className="reference-content">
          {current.segments.map((segment, i) =>
            segment.kind === 'html' ? (
              <div
                key={i}
                className="prose-segment"
                dangerouslySetInnerHTML={{
                  __html: segment.html.replaceAll(
                    '<table>',
                    '<table tabindex="0" aria-label="Scrollable specification data table">',
                  ),
                }}
              />
            ) : (
              <SpecFigureView
                key={segment.id}
                figure={segment}
                chapters={sections}
                onChapter={onChapter}
                live={live}
              />
            ),
          )}
        </div>
        <nav
          aria-label="Chapter pages"
          className={cn(
            'mt-8 flex gap-3 border-t border-line pt-4',
            chapter > 1 ? 'justify-between' : 'justify-end',
          )}
        >
          {chapter > 1 && (
            <ChapterLink
              chapter={chapter - 1}
              onChapter={onChapter}
              className={pageLink}
            >
              ← {sections[chapter - 2].title}
            </ChapterLink>
          )}
          {chapter < 26 && (
            <ChapterLink
              chapter={chapter + 1}
              onChapter={onChapter}
              className={cn(pageLink, 'text-right')}
            >
              {sections[chapter].title} →
            </ChapterLink>
          )}
        </nav>
      </article>
    </div>
  );
}
