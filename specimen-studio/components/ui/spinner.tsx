import { cn } from '@/lib/utils';
import { Loader2Icon } from 'lucide-react';

function Spinner({ className, ...props }: React.ComponentProps<'svg'>) {
  return (
    <Loader2Icon
      data-slot="spinner"
      // The SVG itself is the labelled status, preserving its size and SVG ref API.
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
      role="status"
      aria-label="Loading"
      className={cn('size-4 animate-spin', className)}
      {...props}
    />
  );
}

export { Spinner };
