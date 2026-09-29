export type ShortcutAction = 'palette' | 'sidebar' | 'dock' | 'split' | 'close';

function editing(target: EventTarget | null): boolean {
  const el = target as { tagName?: string; isContentEditable?: boolean } | null;
  return (
    !!el &&
    (el.isContentEditable === true ||
      ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName ?? ''))
  );
}

/** Maps a keydown to a shell action; layout keys never fire while typing. */
export function matchShortcut(e: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  target: EventTarget | null;
}): ShortcutAction | null {
  const mod = e.metaKey || e.ctrlKey;
  if (mod && !e.altKey && e.key.toLowerCase() === 'k') return 'palette';
  if (e.key === 'Escape') return 'close';
  if (editing(e.target)) return null;
  if (mod && !e.altKey && e.key.toLowerCase() === 'b') return 'sidebar';
  if (mod && !e.altKey && e.key === '\\') return 'split';
  if (!mod && !e.altKey && e.key === '`') return 'dock';
  return null;
}
