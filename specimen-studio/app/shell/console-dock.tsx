import type { Specimen } from '@/lib/specimen/types';
import { clock } from '../state/format';

/** Live activity journal; the WDBX console joins it in P3. */
export function ConsoleDock({ events }: { events: Specimen['events'] }) {
  const latest = events.slice(0, 50);
  return (
    <section className="console-dock" aria-label="Activity dock">
      <header className="console-dock-head">
        <h2>Activity</h2>
        <small>
          Latest {latest.length} of {events.length} events
        </small>
      </header>
      <ol className="console-dock-log" role="log" aria-live="off">
        {latest.map((event) => (
          <li key={event.id}>
            <time dateTime={event.createdAt}>{clock(event.createdAt)}</time>
            <strong>{event.title}</strong>
            <span>{event.detail}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}
