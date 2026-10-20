import { graphView } from '@/lib/specimen/kernel';
import { nextCamera, type NetworkViewPrefs } from './view-prefs';

function kernelCamera(
  prefs: NetworkViewPrefs,
  advance: boolean,
): NetworkViewPrefs['camera'] {
  try {
    const camera = graphView({
      camera: prefs.camera,
      edges: prefs.edges,
      edgeBudget: prefs.edgeBudget,
      refs: [],
      links: [],
      advance,
    }).camera;
    if (camera === 'top' || camera === 'side' || camera === 'orbit')
      return camera;
  } catch {
    // The kernel is still loading; the local cycle matches it.
  }
  return advance ? nextCamera(prefs.camera) : prefs.camera;
}

/** Obsidian-style graph controls plus a Cursor-like assistant. View automation only. */
export function GraphChrome({
  prefs,
  onChange,
  note,
  onReview,
  onTeach,
}: {
  prefs: NetworkViewPrefs;
  onChange: (next: NetworkViewPrefs) => void;
  note: string;
  onReview?: () => void;
  onTeach?: () => void;
}) {
  const set = (patch: Partial<NetworkViewPrefs>) =>
    onChange({ ...prefs, ...patch });
  return (
    <div className="graph-chrome">
      <div
        className="graph-chrome-tools"
        role="toolbar"
        aria-label="Graph view"
        tabIndex={0}
        onKeyDown={(event) => {
          const target = event.target;
          if (
            target instanceof HTMLInputElement ||
            target instanceof HTMLSelectElement ||
            target instanceof HTMLTextAreaElement
          )
            return;
          if (event.metaKey || event.ctrlKey || event.altKey) return;
          const key = event.key.toLowerCase();
          if (key === '1') set({ camera: 'orbit' });
          else if (key === '2') set({ camera: 'top' });
          else if (key === '3') set({ camera: 'side' });
          else if (key === 'f') set({ follow: !prefs.follow });
          else if (key === 'n')
            set({
              camera: kernelCamera(prefs, true),
            });
          else return;
          event.preventDefault();
        }}
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
        <button
          type="button"
          className="button utility"
          onClick={() =>
            set({
              camera: kernelCamera(prefs, true),
            })
          }
        >
          Next view
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
          Keys 1, 2, and 3 set the camera while this toolbar is focused. N
          cycles the view. F follows the selection. None of these teach or
          train.
        </p>
        <div className="graph-chrome-tools">
          {onReview && (
            <button type="button" className="button outline" onClick={onReview}>
              Review in Activity
            </button>
          )}
          {onTeach && (
            <button type="button" className="button outline" onClick={onTeach}>
              Teach in Studio
            </button>
          )}
        </div>
      </aside>
    </div>
  );
}
