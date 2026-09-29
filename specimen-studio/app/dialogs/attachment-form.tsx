import { useState } from 'react';
import { GitBranch } from 'lucide-react';
import { type Specimen } from '@/lib/specimen/types';

import { formText } from '../state/format';

export function AttachmentForm({
  state,
  onSave,
}: {
  state: Specimen;
  onSave: (a: Omit<Specimen['attachments'][number], 'id'>) => void;
}) {
  const [error, setError] = useState('');
  return (
    <form
      className="editor-form"
      onSubmit={(e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const from = formText(f, 'from'),
          to = formText(f, 'to');
        if (from === to) {
          setError('Choose two different nodes.');
          return;
        }
        if (state.attachments.some((a) => a.from === from && a.to === to)) {
          setError('Those nodes are already attached.');
          return;
        }
        onSave({
          from,
          to,
          bidirectional: f.get('bidirectional') === 'on',
          hard: f.get('hard') === 'on',
          affinity: Number(f.get('affinity')),
        });
      }}
    >
      {['from', 'to'].map((field, i) => (
        <div key={field}>
          <label htmlFor={`attach-${field}`}>
            {field === 'from' ? 'Issuing node' : 'Attached node'}
          </label>
          <select
            id={`attach-${field}`}
            name={field}
            defaultValue={state.nodes[i]?.ref}
          >
            {state.nodes.map((n) => (
              <option key={n.ref} value={n.ref}>
                {n.name} · {n.patternId}
              </option>
            ))}
          </select>
        </div>
      ))}
      <label htmlFor="attach-affinity">Attachment affinity (0 to 1)</label>
      <input
        id="attach-affinity"
        name="affinity"
        type="number"
        min="0"
        max="1"
        step="0.05"
        defaultValue="0.7"
      />
      <label className="checkbox-label">
        <input type="checkbox" name="hard" />
        Hard attachment (always schedule)
      </label>
      <label className="checkbox-label">
        <input type="checkbox" name="bidirectional" />
        Bidirectional handoff
      </label>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <button className="button primary full-width" type="submit">
        Create attachment
        <GitBranch size={16} />
      </button>
    </form>
  );
}
