import { Search, X } from 'lucide-react';
import { EmptyState } from '@/components/wdbx';

/** A labelled search box with a clear button; the label is the placeholder. */
export function SearchField({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
}) {
  return (
    <div className="flex min-h-11 w-full min-w-0 items-center gap-2 rounded-lg border border-line bg-surface-2 px-3 text-ink-soft focus-within:outline-2 focus-within:outline-focus sm:max-w-xs">
      <Search aria-hidden="true" size={16} className="shrink-0" />
      <input
        name="search"
        aria-label={placeholder.replace('…', '')}
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="min-h-10 w-full min-w-0 bg-transparent text-sm text-ink outline-none placeholder:text-muted-foreground"
      />
      {value && (
        <button
          type="button"
          aria-label="Clear search"
          onClick={() => onChange('')}
          className="-mr-2 grid size-11 shrink-0 place-items-center rounded-md hover:text-ink focus-visible:outline-2 focus-visible:outline-focus"
        >
          <X aria-hidden="true" size={15} />
        </button>
      )}
    </div>
  );
}

/** Kept for the panels that still import it; renders the WDBX empty state. */
export function Empty({ title, text }: { title: string; text: string }) {
  return <EmptyState title={title} text={text} />;
}
