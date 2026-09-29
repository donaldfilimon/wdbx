import { useState } from 'react';
import { Check } from 'lucide-react';
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
      className="editor-form"
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
      <label htmlFor="memory-term">Word, phrase, or concept</label>
      <input
        id="memory-term"
        name="term"
        autoComplete="off"
        required
        defaultValue={resource?.text ?? ''}
        placeholder="e.g. curiosity…"
      />
      <label htmlFor="memory-value">Supporting information</label>
      <textarea
        id="memory-value"
        name="value"
        autoComplete="off"
        required
        defaultValue={resource?.value ?? ''}
        placeholder="Add a definition, synonyms, or a contextual rule…"
      />
      <div className="form-columns">
        <div>
          <label htmlFor="memory-subsystem">Subsystem</label>
          <select
            id="memory-subsystem"
            name="subsystem"
            defaultValue={resource?.subsystem ?? 'dictionary'}
          >
            {SUBSYSTEMS.map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </div>
        <div>
          <label htmlFor="memory-link">Link to node ID</label>
          <select
            id="memory-link"
            name="link"
            defaultValue={resource?.resourceId ?? ''}
          >
            <option value="">Unlinked</option>
            {[...new Set(state.nodes.map((n) => n.patternId))].map((id) => (
              <option key={id}>{id}</option>
            ))}
          </select>
        </div>
      </div>
      <div className="form-columns">
        <div>
          <label htmlFor="memory-valence">Valence (−1 to 1)</label>
          <input
            id="memory-valence"
            name="valence"
            type="number"
            min="-1"
            max="1"
            step="0.1"
            defaultValue={resource?.valence ?? 0}
          />
        </div>
        <div>
          <label htmlFor="memory-intensity">Intensity (0 to 1)</label>
          <input
            id="memory-intensity"
            name="intensity"
            type="number"
            min="0"
            max="1"
            step="0.1"
            defaultValue={resource?.intensity ?? 0.5}
          />
        </div>
      </div>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <button className="button primary full-width" type="submit">
        Save memory
        <Check size={16} />
      </button>
    </form>
  );
}
