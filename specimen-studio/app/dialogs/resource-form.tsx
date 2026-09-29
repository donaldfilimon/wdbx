import { useState } from 'react';
import { Check } from 'lucide-react';
import {
  control,
  Field,
  FieldRow,
  FormError,
  WButton,
} from '@/components/wdbx';
import { SUBSYSTEMS, type Resource, type Specimen } from '@/lib/specimen/types';

import { formText } from '../state/format';

export function ResourceForm({
  resource,
  state,
  onSubmit,
}: {
  resource?: Resource;
  state: Specimen;
  onSubmit: (v: Omit<Resource, 'ref' | 'patternId'>) => void;
}) {
  const [error, setError] = useState('');
  return (
    <form
      className="grid gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        try {
          onSubmit({
            text: formText(f, 'term'),
            value: formText(f, 'value'),
            subsystem: formText(f, 'subsystem'),
            resourceId: formText(f, 'link'),
            valence: Number(f.get('valence')),
            intensity: Number(f.get('intensity')),
          });
        } catch (err) {
          setError(
            err instanceof Error ? err.message : 'Check the memory fields.',
          );
        }
      }}
    >
      <Field htmlFor="memory-term" label="Word, phrase, or concept">
        <input
          className={control}
          id="memory-term"
          name="term"
          autoComplete="off"
          required
          defaultValue={resource?.text ?? ''}
          placeholder="e.g. curiosity…"
        />
      </Field>
      <Field htmlFor="memory-value" label="Supporting information">
        <textarea
          className={`${control} min-h-24`}
          id="memory-value"
          name="value"
          autoComplete="off"
          required
          defaultValue={resource?.value ?? ''}
          placeholder="Add a definition, synonyms, or a contextual rule…"
        />
      </Field>
      <FieldRow>
        <Field htmlFor="memory-subsystem" label="Subsystem">
          <select
            className={control}
            id="memory-subsystem"
            name="subsystem"
            defaultValue={resource?.subsystem ?? 'dictionary'}
          >
            {SUBSYSTEMS.map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </Field>
        <Field htmlFor="memory-link" label="Link to node ID">
          <select
            className={control}
            id="memory-link"
            name="link"
            defaultValue={resource?.resourceId ?? ''}
          >
            <option value="">Unlinked</option>
            {[...new Set(state.nodes.map((n) => n.patternId))].map((id) => (
              <option key={id}>{id}</option>
            ))}
          </select>
        </Field>
      </FieldRow>
      <FieldRow>
        <Field htmlFor="memory-valence" label="Valence (−1 to 1)">
          <input
            className={control}
            id="memory-valence"
            name="valence"
            type="number"
            min="-1"
            max="1"
            step="0.1"
            defaultValue={resource?.valence ?? 0}
          />
        </Field>
        <Field htmlFor="memory-intensity" label="Intensity (0 to 1)">
          <input
            className={control}
            id="memory-intensity"
            name="intensity"
            type="number"
            min="0"
            max="1"
            step="0.1"
            defaultValue={resource?.intensity ?? 0.5}
          />
        </Field>
      </FieldRow>
      {error && <FormError>{error}</FormError>}
      <WButton variant="primary" type="submit" className="w-full">
        Save memory
        <Check aria-hidden="true" size={16} />
      </WButton>
    </form>
  );
}
