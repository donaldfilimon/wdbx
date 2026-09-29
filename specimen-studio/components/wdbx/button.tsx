import { cva, type VariantProps } from 'class-variance-authority';
import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export const buttonVariants = cva(
  'inline-flex min-h-11 items-center justify-center gap-2 rounded-lg border px-3.5 text-sm font-medium transition-colors duration-150 motion-reduce:transition-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus disabled:cursor-not-allowed disabled:opacity-50 [&_svg]:shrink-0',
  {
    variants: {
      variant: {
        primary:
          'border-teal bg-teal text-on-teal shadow-sm hover:border-teal-strong hover:bg-teal-strong',
        outline: 'border-line-strong bg-raised text-ink hover:bg-surface-2',
        utility: 'border-line bg-surface text-ink-soft hover:bg-surface-2',
        ghost:
          'border-transparent bg-transparent text-ink-soft hover:bg-surface-2',
      },
      size: {
        default: '',
        icon: 'min-w-11 px-0',
      },
    },
    defaultVariants: { variant: 'outline', size: 'default' },
  },
);

/** The studio's button: 44 px minimum target, token colors, visible focus. */
export function WButton({
  className,
  variant,
  size,
  type = 'button',
  ...props
}: ComponentProps<'button'> & VariantProps<typeof buttonVariants>) {
  return (
    <button
      type={type}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
}
