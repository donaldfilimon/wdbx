import type { ComponentProps, ReactNode } from 'react';
import { cn } from '@/lib/utils';

export function StatGroup({
  label,
  className,
  ...props
}: ComponentProps<'dl'> & { label: string }) {
  return (
    <dl
      aria-label={label}
      className={cn(
        'm-0 grid grid-cols-[repeat(auto-fit,minmax(9rem,1fr))] gap-px overflow-hidden rounded-xl border border-line bg-line',
        className,
      )}
      {...props}
    />
  );
}

export function Stat({
  label,
  value,
  hint,
}: {
  label: ReactNode;
  value: ReactNode;
  hint?: ReactNode;
}) {
  return (
    <div className="bg-surface px-4 py-3">
      <dt className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
        {label}
      </dt>
      <dd className="m-0 mt-1 font-mono text-lg font-semibold text-ink tabular-nums">
        {value}
      </dd>
      {hint && <dd className="m-0 text-xs text-muted-foreground">{hint}</dd>}
    </div>
  );
}
