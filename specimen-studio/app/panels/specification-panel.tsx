/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */

import sections from '@/lib/specification.json';
import { SearchField } from './common';

export function Specification({
  chapter,
  query,
  setQuery,
  onChapter,
}: {
  chapter: number;
  query: string;
  setQuery: (v: string) => void;
  onChapter: (n: number) => void;
}) {
  const current = sections.find((s) => s.number === chapter) ?? sections[0];
  const filtered = sections.filter((s) =>
    `${s.title} ${s.text}`.toLowerCase().includes(query.toLowerCase()),
  );
  const accessibleHtml = current.html.replaceAll(
    '<table>',
    '<table tabindex="0" aria-label="Scrollable specification data table">',
  );
  return (
    <div className="reference-layout">
      <aside className="chapter-nav">
        <SearchField
          value={query}
          onChange={setQuery}
          placeholder="Search specification…"
        />
        <span className="chapter-count">{filtered.length} chapters</span>
        <nav aria-label="Specification chapters">
          {filtered.map((s) => (
            <a
              key={s.number}
              href={`?view=specification&chapter=${s.number}`}
              className={s.number === chapter ? 'active' : ''}
              aria-current={s.number === chapter ? 'page' : undefined}
              onClick={(e) => {
                if (!e.metaKey && !e.ctrlKey) {
                  e.preventDefault();
                  onChapter(s.number);
                }
              }}
            >
              <span>{String(s.number).padStart(2, '0')}</span>
              {s.title}
            </a>
          ))}
        </nav>
        {!filtered.length && (
          <p className="muted">No chapters match that search.</p>
        )}
      </aside>
      <article className="reference-article">
        <div className="reference-meta">
          <span>{String(current.number).padStart(2, '0')} / 26</span>
          <span>{Math.ceil(current.words / 230)} min read</span>
        </div>
        <h2>{current.title}</h2>
        <div
          className="reference-content"
          dangerouslySetInnerHTML={{ __html: accessibleHtml }}
        />
        <div className="chapter-pagination">
          {chapter > 1 ? (
            <a
              href={`?view=specification&chapter=${chapter - 1}`}
              onClick={(e) => {
                e.preventDefault();
                onChapter(chapter - 1);
              }}
            >
              ← {sections[chapter - 2].title}
            </a>
          ) : (
            <span />
          )}
          {chapter < 26 && (
            <a
              href={`?view=specification&chapter=${chapter + 1}`}
              onClick={(e) => {
                e.preventDefault();
                onChapter(chapter + 1);
              }}
            >
              {sections[chapter].title} →
            </a>
          )}
        </div>
      </article>
    </div>
  );
}
