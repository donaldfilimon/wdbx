import type { NetworkViewPrefs } from './view-prefs';

/** Obsidian-style graph controls plus a Cursor-like assistant. View automation only. */
export function GraphChrome({
  prefs,
  onChange,
  note,
}: {
  prefs: NetworkViewPrefs;
  onChange: (next: NetworkViewPrefs) => void;
  note: string;
}) {
  const set = (patch: Partial<NetworkViewPrefs>) =>
    onChange({ ...prefs, ...patch });
  return (
    <div className="graph-chrome">
      <div
        className="graph-chrome-tools"
        role="toolbar"
        aria-label="Graph view"
      >
        {(['orbit', 'top', 'side'] as const).map((camera) => (
          <button
            key={camera}
            type="button"
            className="button utility"
            aria-pressed={prefs.camera === camera}
            onClick={() => set({ camera })}
          >
            {camera === 'orbit' ? 'Orbit' : camera === 'top' ? 'Top' : 'Side'}
          </button>
        ))}
        <button
          type="button"
          className="button utility"
          aria-pressed={prefs.follow}
          onClick={() => set({ follow: !prefs.follow })}
        >
          Follow selection
        </button>
        <label className="graph-chrome-field">
          Edges
          <select
            aria-label="Edge filter"
            value={prefs.edges}
            onChange={(event) =>
              set({ edges: event.target.value as NetworkViewPrefs['edges'] })
            }
          >
            <option value="all">All</option>
            <option value="hard">Hard only</option>
            <option value="local">Neighborhood</option>
          </select>
        </label>
        <label className="graph-chrome-field">
          Density
          <input
            aria-label="Connection density"
            type="range"
            min={80}
            max={600}
            step={20}
            value={prefs.edgeBudget}
            onChange={(event) =>
              set({ edgeBudget: Number(event.target.value) })
            }
          />
        </label>
      </div>
      <aside className="agent-rail" aria-label="Graph assistant">
        <strong>Graph assistant</strong>
        <p>{note}</p>
        <p>
          Follow selection and the camera presets only move the view. Teaching,
          review, and any weight change stay on the visible controls.
        </p>
      </aside>
    </div>
  );
}
