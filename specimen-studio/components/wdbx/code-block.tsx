import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

/** Preformatted text in a keyboard-focusable, labelled scroll region. */
export function CodeBlock({
  label,
  className,
  ...props
}: ComponentProps<'pre'> & { label: string }) {
  return (
    <pre
      // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- axe requires scrollable regions to be keyboard-focusable.
      tabIndex={0}
      aria-label={label}
      className={cn(
        'm-0 max-h-72 overflow-auto rounded-lg border border-line bg-surface-2 p-3 font-mono text-xs text-ink-soft focus-visible:outline-2 focus-visible:outline-focus',
        className,
      )}
      {...props}
    />
  );
}
