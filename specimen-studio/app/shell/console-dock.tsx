import type { Specimen } from '@/lib/specimen/types';
import { clock } from '../state/format';

/** Activity journal plus the command line. Both call the kernel parser. */
export function ConsoleDock({
  events,
  replies,
  onCommand,
}: {
  events: Specimen['events'];
  replies: { id: number; text: string }[];
  onCommand: (line: string) => void;
}) {
  const latest = events.slice(0, 50);
  return (
    <section className="console-dock" aria-label="Activity dock">
      <header className="console-dock-head">
        <h2>Activity</h2>
        <small>
          Latest {latest.length} of {events.length} events
        </small>
      </header>
      {/* The scroll region is focusable for keyboard scrolling; role="log"
          sits on the wrapper so the <ol> keeps its list semantics. */}
      <div
        className="console-dock-log"
        role="log"
        aria-label="Activity events"
        aria-live="off"
        // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- axe requires scrollable regions to be keyboard-focusable.
        tabIndex={0}
      >
        <ol>
          {latest.map((event) => (
            <li key={event.id}>
              <time dateTime={event.createdAt}>{clock(event.createdAt)}</time>
              <strong>{event.title}</strong>
              <span>{event.detail}</span>
            </li>
          ))}
        </ol>
      </div>
      {replies.length > 0 && (
        <div
          className="console-replies"
          role="log"
          aria-label="Console replies"
          aria-live="polite"
        >
          {replies.map((line) => (
            <p key={line.id}>{line.text}</p>
          ))}
        </div>
      )}
      <form
        className="console-dock-form"
        onSubmit={(event) => {
          event.preventDefault();
          const value = new FormData(event.currentTarget).get('command');
          const line = (typeof value === 'string' ? value : '').trim();
          if (!line) return;
          onCommand(line);
          event.currentTarget.reset();
        }}
      >
        <label className="sr-only" htmlFor="console-command">
          Command line
        </label>
        <input
          id="console-command"
          name="command"
          autoComplete="off"
          placeholder="Run a command…"
        />
        <button className="button outline" type="submit">
          Run command
        </button>
      </form>
    </section>
  );
}
