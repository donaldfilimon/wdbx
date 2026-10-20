import {
  applyCommand,
  maintenance,
  validateSpecimen,
} from '@/lib/specimen/kernel';
import { isDesktop, nativeReview } from '@/lib/specimen/native';
import { ActivityView } from '../panels/activity-panel';
import { EnginePanel } from '../panels/engine-panel';
import { MemoryView } from '../panels/memory-panel';
import { NodesView } from '../panels/nodes-panel';
import { SettingsView } from '../panels/settings-panel';
import { StorePanel } from '../panels/store-panel';
import { BrowserLabIntro, LabPanel } from '../lab/lab-panel';
import { NetworkPanel } from '../network/network-panel';
import { Specification } from '../panels/specification-panel';
import { NotesView } from '../panels/notes-panel';
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
    pushUndo,
    agentAsk,
    agentLog,
    decideAgentRun,
  } = model;
  return (
    <>
      {view === 'studio' && <StudioPanel m={model} nav={nav} />}
      {view === 'lab' &&
        (isDesktop() ? (
          <LabPanel specimen={state} onSnapshot={commit} />
        ) : (
          <BrowserLabIntro />
        ))}
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
              const applied = applyCommand(state, {
                command: 'removeNode',
                nodeRef: ref,
              });
              pushUndo(applied.undo);
              commit(applied.state);
              announce('Node removed. Use Undo to restore it.');
            })
          }
          onAttach={() => openDialog('attach')}
          onSelect={(ref) => {
            setSelected(ref);
            nav('studio');
          }}
        />
      )}
      {view === 'notes' && (
        <NotesView
          state={state}
          commit={commit}
          pushUndo={pushUndo}
          announce={announce}
          act={act}
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
          onPin={(id) =>
            act(() => {
              const applied = applyCommand(state, {
                command: 'togglePin',
                cycleId: id,
              });
              pushUndo(applied.undo);
              commit(applied.state);
            })
          }
          onRemove={(ref) =>
            act(() => {
              const applied = applyCommand(state, {
                command: 'removeResource',
                resourceRef: ref,
              });
              pushUndo(applied.undo);
              commit(applied.state);
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
          onDismissAll={() =>
            act(() => {
              const applied = applyCommand(state, {
                command: 'dismissAllProposals',
              });
              pushUndo(applied.undo);
              commit(applied.state);
              announce(applied.message);
            })
          }
          onReconsiderAll={() =>
            act(() => {
              const applied = applyCommand(state, {
                command: 'reconsiderAllProposals',
              });
              pushUndo(applied.undo);
              commit(applied.state);
              announce(applied.message);
            })
          }
          agentAsk={agentAsk}
          agentLog={agentLog}
          onApprove={() => decideAgentRun(true)}
          onDecline={() => decideAgentRun(false)}
          onDismiss={(id) =>
            act(() => {
              const applied = applyCommand(state, {
                command: 'dismissProposal',
                id,
              });
              pushUndo(applied.undo);
              commit(applied.state);
              announce(applied.message);
            })
          }
          onReconsider={(id) =>
            act(() => {
              const applied = applyCommand(state, {
                command: 'reconsiderProposal',
                id,
              });
              pushUndo(applied.undo);
              commit(applied.state);
              announce(applied.message);
            })
          }
        />
      )}
      {view === 'engine' && (
        <EnginePanel
          trace={model.activeTrace}
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
      {view === 'network' && (
        <NetworkPanel
          specimen={state}
          desktop={isDesktop()}
          reloadKey={model.networkEpoch}
          onReview={() => nav('activity')}
          onTeach={() => nav('studio')}
          onSpecimenCommand={(command) =>
            act(() => {
              const applied = applyCommand(state, command);
              pushUndo(applied.undo);
              commit(applied.state);
              announce(applied.message);
            })
          }
        />
      )}
      {view === 'specification' && (
        <Specification
          chapter={chapter}
          query={query}
          setQuery={setQuery}
          onChapter={onChapter}
          live={{
            specimen: state,
            trace: model.activeTrace,
            outcome: model.runOutcome,
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
              const applied = applyCommand(state, {
                command: 'setSettings',
                settings,
                name,
              });
              validateSpecimen(applied.state);
              pushUndo(applied.undo);
              commit(applied.state);
              announce(applied.message);
            })
          }
        />
      )}
    </>
  );
}
