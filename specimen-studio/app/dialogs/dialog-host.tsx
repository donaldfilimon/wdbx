import {
  addNode,
  log,
  saveResource,
  seedSpecimen,
} from '@/lib/specimen/engine';
import { uid } from '@/lib/specimen/types';
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
      <DialogContent className="studio-dialog">
        <DialogTitle>
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
        <DialogDescription>
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
              {
                const next = addNode(state, value, editing);
                const proposal = next.proposals.find(
                  (p) => p.pattern === value.pattern,
                );
                if (proposal) proposal.status = 'learned';
                commit(next);
                setDialog(null);
                announce(editing ? 'Node updated.' : 'Pattern learned.');
              }
            }}
          />
        )}
        {dialog === 'resource' && (
          <ResourceForm
            resource={state.resources.find((r) => r.ref === editing)}
            state={state}
            onSubmit={(value) => {
              commit(saveResource(state, value, editing));
              setDialog(null);
              announce('Memory saved.');
            }}
          />
        )}
        {dialog === 'load' && pendingLoad && (
          <>
            <div className="load-summary">
              <strong>{pendingLoad.name}</strong>
              <p>
                {pendingLoad.nodes.length} nodes ·{' '}
                {pendingLoad.resources.length} memories ·{' '}
                {pendingLoad.history.length} conversations
              </p>
            </div>
            <div className="modal-actions">
              <button className="button outline" onClick={save}>
                Save current specimen
              </button>
              <button
                className="button primary"
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
              </button>
            </div>
          </>
        )}
        {dialog === 'reset' && (
          <div className="modal-actions">
            <button className="button outline" onClick={save}>
              Save current specimen
            </button>
            <button
              className="button primary"
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
            </button>
          </div>
        )}
        {dialog === 'attach' && (
          <AttachmentForm
            state={state}
            onSave={(a) => {
              const s = structuredClone(state);
              s.attachments.push({ ...a, id: uid() });
              log(
                s,
                'attachment',
                'Nodes attached',
                'A semantic handoff was configured.',
              );
              commit(s);
              setDialog(null);
              announce('Attachment added.');
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
