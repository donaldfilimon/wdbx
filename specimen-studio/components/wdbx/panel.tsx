import { useId, type ComponentProps, type ReactNode } from 'react';
import { cn } from '@/lib/utils';

/**
 * A bordered surface. With a `title` it is a labelled section (its heading
 * names the landmark); `actions` sit at the header's end.
 */
export function Panel({
  title,
  description,
  actions,
  className,
  children,
  ...props
}: Omit<ComponentProps<'section'>, 'title'> & {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
}) {
  const id = useId();
  return (
    <section
      aria-labelledby={title ? id : undefined}
      className={cn(
        'rounded-xl border border-line bg-surface text-ink shadow-sm',
        className,
      )}
      {...props}
    >
      {title && (
        <header className="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
          <div className="min-w-0 flex-1">
            <h2 id={id} className="m-0 text-sm font-semibold text-ink">
              {title}
            </h2>
            {description && (
              <p className="m-0 mt-0.5 text-xs text-muted-foreground">
                {description}
              </p>
            )}
          </div>
          {actions}
        </header>
      )}
      {children}
    </section>
  );
}
