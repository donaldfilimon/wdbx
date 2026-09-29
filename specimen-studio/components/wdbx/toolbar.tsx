import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function Toolbar({
  label,
  className,
  ...props
}: ComponentProps<'div'> & { label: string }) {
  return (
    <div
      role="toolbar"
      aria-label={label}
      className={cn('flex flex-wrap items-center gap-2', className)}
      {...props}
    />
  );
}

export function ToolbarSpacer() {
  return <span aria-hidden="true" className="flex-1" />;
}
