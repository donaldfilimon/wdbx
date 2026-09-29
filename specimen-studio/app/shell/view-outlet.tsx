import {
  maintenance,
  removeNode,
  togglePin,
  validateSettings,
  validateSpecimen,
} from '@/lib/specimen/kernel';
import { NativeLab } from '@/components/native-lab';
import { isDesktop, nativeReview } from '@/lib/specimen/native';
import { ActivityView } from '../panels/activity-panel';
import { EnginePanel } from '../panels/engine-panel';
import { MemoryView } from '../panels/memory-panel';
import { NodesView } from '../panels/nodes-panel';
import { SettingsView } from '../panels/settings-panel';
import { StorePanel } from '../panels/store-panel';
import { Specification } from '../panels/specification-panel';
import { StudioPanel } from '../panels/studio-panel';
import type { View } from '../state/navigation';
import type { StudioModel } from '../state/use-studio';

/** Renders one view; the shell uses it for the main pane and the side pane. */
export function ViewOutlet({
  view,
  model,
  nav,
  query,
  setQuery,
  chapter,
  onChapter,
}: {
  view: View;
  model: StudioModel;
  nav: (v: View, section?: number) => void;
  query: string;
  setQuery: (q: string) => void;
  chapter: number;
  /** Opening a chapter: the primary pane navigates, the side pane stays put. */
  onChapter: (n: number) => void;
}) {
  const {
    state,
    commit,
    act,
    announce,
    openDialog,
    setEditing,
    setSelected,
    setPrompt,
    busy,
    setError,
    reviewContext,
    current,
    setUndo,
  } = model;
  return (
    <>
      {view === 'studio' && <StudioPanel m={model} nav={nav} />}
      {view === 'lab' && <NativeLab specimen={state} onSnapshot={commit} />}
      {view === 'nodes' && (
        <NodesView
          state={state}
          query={query}
          setQuery={setQuery}
          onAdd={() => {
            setEditing(undefined);
            openDialog('node');
          }}
          onEdit={(ref) => {
            setEditing(ref);
            openDialog('node');
          }}
          onRemove={(ref) =>
            act(() => {
              const previous = state;
              commit(removeNode(state, ref));
              announce('Node removed. Use Undo to restore it.');
              setUndo(previous);
            })
          }
          onAttach={() => openDialog('attach')}
          onSelect={(ref) => {
            setSelected(ref);
            nav('studio');
          }}
        />
      )}
      {view === 'memory' && (
        <MemoryView
          state={state}
          query={query}
          setQuery={setQuery}
          onAdd={() => {
            setEditing(undefined);
            openDialog('resource');
          }}
          onEdit={(ref) => {
            setEditing(ref);
            openDialog('resource');
          }}
          onPin={(id) => act(() => commit(togglePin(state, id)))}
          onRemove={(ref) =>
            act(() => {
              setUndo(state);
              const s = structuredClone(state);
              s.resources = s.resources.filter((r) => r.ref !== ref);
              commit(s);
              announce('Memory removed. Use Undo to restore it.');
            })
          }
        />
      )}
      {view === 'activity' && (
        <ActivityView
          state={state}
          busy={busy}
          onReview={() => void reviewContext()}
          onMaintenance={(mode) =>
            act(() => {
              if (isDesktop()) {
                void nativeReview(current.current, 'maintenance', mode)
                  .then(commit)
                  .then(() => announce('Maintenance completed.'))
                  .catch((e) => setError(e.message));
              } else {
                commit(maintenance(state, mode));
                announce('Maintenance completed.');
              }
            })
          }
          onLearn={(pattern) => {
            setPrompt(pattern);
            setEditing(undefined);
            openDialog('node');
          }}
        />
      )}
      {view === 'engine' && (
        <EnginePanel
          trace={model.activeTrace}
          busy={busy}
          cycle={model.cycle}
          history={state.history}
          nodes={state.nodes}
          atp={state.atp}
          threshold={state.settings.voteThreshold}
          outcome={model.runOutcome}
          runInput={model.runInput}
        />
      )}
      {view === 'store' && <StorePanel desktop={isDesktop()} />}
      {view === 'specification' && (
        <Specification
          chapter={chapter}
          query={query}
          setQuery={setQuery}
          onChapter={onChapter}
          live={{
            specimen: state,
            trace: model.activeTrace,
            busy,
            desktop: isDesktop(),
          }}
        />
      )}
      {view === 'settings' && (
        <SettingsView
          key={state.name + JSON.stringify(state.settings)}
          state={state}
          onSave={(settings, name) =>
            act(() => {
              validateSettings(settings);
              if (
                state.nodes.some(
                  (n) =>
                    n.entries.length > settings.entryLimit ||
                    n.strength > settings.maxStrength,
                )
              )
                throw new Error(
                  'Existing nodes exceed those limits. Adjust them before reducing the limits.',
                );
              const next = { ...state, settings, name: name.trim() };
              validateSpecimen(next);
              commit(next);
              announce('Settings saved.');
            })
          }
        />
      )}
    </>
  );
}
