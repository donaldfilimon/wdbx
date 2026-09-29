import { Boxes, Search, X } from 'lucide-react';

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
    <div className="search-field">
      <Search size={17} />
      <input
        name="search"
        aria-label={placeholder.replace('…', '')}
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
      {value && (
        <button
          className="icon-button"
          aria-label="Clear search"
          onClick={() => onChange('')}
        >
          <X size={15} />
        </button>
      )}
    </div>
  );
}

export function Empty({ title, text }: { title: string; text: string }) {
  return (
    <div className="empty-state">
      <Boxes size={30} />
      <h3>{title}</h3>
      <p>{text}</p>
    </div>
  );
}
