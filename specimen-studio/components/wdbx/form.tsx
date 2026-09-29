import type { ComponentProps, ReactNode } from 'react';
import { cn } from '@/lib/utils';

/** Shared look for native inputs, selects and textareas: 44px, token colours. */
export const control =
  'min-h-11 w-full rounded-lg border border-line bg-surface-2 px-2.5 py-2 text-sm text-ink placeholder:text-muted-foreground focus-visible:outline-2 focus-visible:outline-focus disabled:opacity-60';

/** A label above its control; `htmlFor` must match the control's id. */
export function Field({
  label,
  htmlFor,
  hint,
  className,
  children,
}: {
  label: ReactNode;
  htmlFor: string;
  hint?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn('grid gap-1.5', className)}>
      <label htmlFor={htmlFor} className="text-xs font-semibold text-ink">
        {label}
      </label>
      {children}
      {hint && <p className="m-0 text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

/** Two fields side by side from small screens up. */
export function FieldRow({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div className={cn('grid gap-3 sm:grid-cols-2', className)} {...props} />
  );
}

/** A checkbox wrapped by its label, the whole row a 44px target. */
export function CheckboxField({
  label,
  ...props
}: Omit<ComponentProps<'input'>, 'type'> & { label: ReactNode }) {
  return (
    <label className="flex min-h-11 cursor-pointer items-center gap-2.5 text-sm text-ink">
      <input type="checkbox" className="size-4 accent-teal" {...props} />
      {label}
    </label>
  );
}

export function FormError({ children }: { children: ReactNode }) {
  return (
    <p role="alert" className="m-0 text-sm text-danger">
      {children}
    </p>
  );
}

/** A collapsible group of optional fields. */
export function Disclosure({
  summary,
  className,
  children,
}: {
  summary: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <details
      className={cn('grid gap-3 rounded-lg border border-line p-3', className)}
    >
      <summary className="min-h-11 cursor-pointer content-center text-sm font-semibold text-ink-soft">
        {summary}
      </summary>
      <div className="grid gap-3 text-sm text-ink-soft">{children}</div>
    </details>
  );
}
