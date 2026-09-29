import { useState } from 'react';
import { GitBranch } from 'lucide-react';
import {
  CheckboxField,
  control,
  Field,
  FormError,
  WButton,
} from '@/components/wdbx';
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
      className="grid gap-3"
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
        <Field
          key={field}
          htmlFor={`attach-${field}`}
          label={field === 'from' ? 'Issuing node' : 'Attached node'}
        >
          <select
            className={control}
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
        </Field>
      ))}
      <Field htmlFor="attach-affinity" label="Attachment affinity (0 to 1)">
        <input
          className={control}
          id="attach-affinity"
          name="affinity"
          type="number"
          min="0"
          max="1"
          step="0.05"
          defaultValue="0.7"
        />
      </Field>
      <CheckboxField name="hard" label="Hard attachment (always schedule)" />
      <CheckboxField name="bidirectional" label="Bidirectional handoff" />
      {error && <FormError>{error}</FormError>}
      <WButton variant="primary" type="submit" className="w-full">
        Create attachment
        <GitBranch aria-hidden="true" size={16} />
      </WButton>
    </form>
  );
}
