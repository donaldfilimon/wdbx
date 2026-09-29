import { Activity } from 'lucide-react';
import type { EventRecord } from '@/lib/specimen/types';
import { cn } from '@/lib/utils';

const tone: Record<string, string> = {
  feedback: 'bg-amber-soft text-amber',
  learning: 'bg-violet-soft text-violet',
  mutation: 'bg-violet-soft text-violet',
  maintenance: 'bg-info-soft text-info',
};

const time = (iso: string) =>
  new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(iso));

/** Newest-first activity events. */
export function EventTimeline({
  events,
  className,
}: {
  events: EventRecord[];
  className?: string;
}) {
  return (
    <ol className={cn('m-0 list-none divide-y divide-line p-0', className)}>
      {events.map((event) => (
        <li
          key={event.id}
          className="grid grid-cols-[2rem_1fr_auto] items-start gap-3 px-4 py-3"
        >
          <span
            aria-hidden="true"
            className={cn(
              'grid size-8 place-items-center rounded-lg bg-teal-soft text-teal',
              tone[event.type],
            )}
          >
            <Activity size={16} />
          </span>
          <div className="min-w-0">
            <strong className="block text-sm font-semibold text-ink">
              {event.title}
            </strong>
            <p className="m-0 text-sm break-words text-ink-soft">
              {event.detail}
            </p>
          </div>
          <time
            dateTime={event.createdAt}
            className="font-mono text-xs text-muted-foreground tabular-nums"
          >
            {time(event.createdAt)}
          </time>
        </li>
      ))}
    </ol>
  );
}
