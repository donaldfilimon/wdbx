import { applyCommand, seedSpecimen } from '@/lib/specimen/kernel';
import { WButton } from '@/components/wdbx';
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';

import { AttachmentForm } from './attachment-form';
import { NodeForm } from './node-form';
import { ResourceForm } from './resource-form';
import type { StudioModel } from '../state/use-studio';

export function DialogHost({ m }: { m: StudioModel }) {
  const {
    state,
    commit,
    announce,
    dialog,
    setDialog,
    editing,
    pendingLoad,
    setStorageEnabled,
    setSelected,
    prompt,
    pushUndo,
    save,
    setRunOutcome,
    setTrace,
  } = m;
  return (
    <Dialog
      open={dialog !== null}
      onOpenChange={(open) => {
        if (!open) setDialog(null);
      }}
    >
      <DialogContent className="max-h-[90dvh] overflow-y-auto p-6 max-sm:p-5 sm:max-w-lg">
        <DialogTitle className="mr-6 text-2xl leading-tight font-semibold tracking-tight">
          {dialog === 'node'
            ? editing
              ? 'Edit node'
              : 'Teach a pattern'
            : dialog === 'resource'
              ? editing
                ? 'Edit memory'
                : 'Add a memory'
              : dialog === 'load'
                ? 'Load this specimen?'
                : dialog === 'attach'
                  ? 'Attach two nodes'
                  : 'Start a fresh specimen?'}
        </DialogTitle>
        <DialogDescription className="text-sm leading-relaxed text-muted-foreground">
          {dialog === 'node'
            ? 'A pattern activates a node. Its vote defines the action.'
            : dialog === 'resource'
              ? 'Store supporting information in a scoped, indexed resource.'
              : dialog === 'load'
                ? 'This replaces the active workspace. Download your current specimen first if you want to keep it.'
                : dialog === 'attach'
                  ? 'Delegate unmatched input to a related node.'
                  : 'This replaces your workspace with the starter specimen. Save your current work first.'}
        </DialogDescription>
        {dialog === 'node' && (
          <NodeForm
            node={state.nodes.find((n) => n.ref === editing)}
            defaultPattern={!editing ? prompt : ''}
            onSubmit={(value) => {
              const applied = applyCommand(state, {
                command: 'addNode',
                input: value,
                nodeRef: editing,
              });
              const proposal = applied.state.proposals.find(
                (p) => p.pattern === value.pattern,
              );
              if (proposal) proposal.status = 'learned';
              pushUndo(applied.undo);
              commit(applied.state);
              setDialog(null);
              announce(applied.message);
            }}
          />
        )}
        {dialog === 'resource' && (
          <ResourceForm
            resource={state.resources.find((r) => r.ref === editing)}
            state={state}
            onSubmit={(value) => {
              const applied = applyCommand(state, {
                command: 'saveResource',
                input: value,
                resourceRef: editing,
              });
              pushUndo(applied.undo);
              commit(applied.state);
              setDialog(null);
              announce(applied.message);
            }}
          />
        )}
        {dialog === 'load' && pendingLoad && (
          <>
            <div className="rounded-lg bg-surface-2 p-4">
              <strong className="text-ink">{pendingLoad.name}</strong>
              <p className="m-0 mt-1 text-sm text-muted-foreground">
                {pendingLoad.nodes.length} nodes ·{' '}
                {pendingLoad.resources.length} memories ·{' '}
                {pendingLoad.history.length} conversations
              </p>
            </div>
            <div className="mt-4 flex flex-wrap justify-end gap-3">
              <WButton variant="outline" onClick={save}>
                Save current specimen
              </WButton>
              <WButton
                variant="primary"
                onClick={() => {
                  setRunOutcome('idle');
                  setTrace([]);
                  commit(pendingLoad);
                  setStorageEnabled(true);
                  setDialog(null);
                  setSelected('');
                  announce('Specimen loaded.');
                }}
              >
                Load specimen
              </WButton>
            </div>
          </>
        )}
        {dialog === 'reset' && (
          <div className="mt-4 flex flex-wrap justify-end gap-3">
            <WButton variant="outline" onClick={save}>
              Save current specimen
            </WButton>
            <WButton
              variant="primary"
              onClick={() => {
                setRunOutcome('idle');
                setTrace([]);
                commit(seedSpecimen());
                setStorageEnabled(true);
                setDialog(null);
                setSelected('');
                announce('Starter specimen ready.');
              }}
            >
              Create starter specimen
            </WButton>
          </div>
        )}
        {dialog === 'attach' && (
          <AttachmentForm
            state={state}
            onSave={(a) => {
              const applied = applyCommand(state, {
                command: 'connect',
                from: a.from,
                to: a.to,
                bidirectional: a.bidirectional,
                hard: a.hard,
                affinity: a.affinity,
              });
              pushUndo(applied.undo);
              commit(applied.state);
              setDialog(null);
              announce(applied.message);
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
