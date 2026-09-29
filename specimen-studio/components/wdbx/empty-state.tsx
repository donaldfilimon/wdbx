import type { ReactNode } from 'react';

export function EmptyState({
  title,
  text,
  action,
}: {
  title: string;
  text: string;
  action?: ReactNode;
}) {
  return (
    <div className="grid justify-items-center gap-2 px-6 py-10 text-center">
      <strong className="text-sm font-semibold text-ink">{title}</strong>
      <p className="m-0 max-w-prose text-sm text-muted-foreground">{text}</p>
      {action}
    </div>
  );
}
