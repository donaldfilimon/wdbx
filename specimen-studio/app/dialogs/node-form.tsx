import { useState } from 'react';
import { ArrowRight } from 'lucide-react';
import { type SpecimenNode } from '@/lib/specimen/types';

import { formText } from '../state/format';

export type NodeInput = {
  name: string;
  pattern: string;
  action: string;
  tone?: SpecimenNode['tone'];
  type?: SpecimenNode['type'];
  contextId?: string;
  entries?: SpecimenNode['entries'];
};

export function NodeForm({
  node,
  defaultPattern,
  onSubmit,
}: {
  node?: SpecimenNode;
  defaultPattern: string;
  onSubmit: (v: NodeInput) => void;
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
            name: formText(f, 'name'),
            pattern: formText(f, 'pattern'),
            action: formText(f, 'action'),
            tone: formText(f, 'tone') as SpecimenNode['tone'],
            type: formText(f, 'type') as SpecimenNode['type'],
            contextId: formText(f, 'context'),
            entries:
              f.has('entries') && formText(f, 'entries').trim()
                ? JSON.parse(formText(f, 'entries'))
                : undefined,
          });
        } catch (err) {
          setError(
            err instanceof Error ? err.message : 'Check the node fields.',
          );
        }
      }}
    >
      <label htmlFor="node-name">Node name</label>
      <input
        id="node-name"
        name="name"
        autoComplete="off"
        required
        maxLength={100}
        defaultValue={node?.name ?? ''}
        placeholder="e.g. Favorite color…"
      />
      <div className="form-columns">
        <div>
          <label htmlFor="node-type">Node type</label>
          <select
            id="node-type"
            name="type"
            defaultValue={node?.type ?? 'pattern'}
          >
            <option value="pattern">Pattern node</option>
            <option value="A">Type A · residual supervisor</option>
            <option value="B">Type B · idle observer</option>
          </select>
        </div>
        <div>
          <label htmlFor="node-tone">Tone</label>
          <select
            id="node-tone"
            name="tone"
            defaultValue={node?.tone ?? 'neutral'}
          >
            {['neutral', 'warm', 'curious', 'cautious'].map((t) => (
              <option key={t}>{t}</option>
            ))}
          </select>
        </div>
      </div>
      <label htmlFor="node-pattern">Original pattern</label>
      <textarea
        id="node-pattern"
        name="pattern"
        spellCheck={false}
        autoComplete="off"
        required
        maxLength={2000}
        defaultValue={node?.entries[0]?.pattern ?? defaultPattern}
        placeholder="e.g. what is your favorite color…"
      />
      <label htmlFor="node-action">Response or action</label>
      <textarea
        id="node-action"
        name="action"
        spellCheck={false}
        autoComplete="off"
        required
        maxLength={8000}
        defaultValue={node?.entries[0]?.alternatives[0]?.action ?? ''}
        placeholder="e.g. Forest green. Or a scoped action sigil…"
      />
      <details>
        <summary>Sigils & context</summary>
        <p>
          Use plain response text or <code>&calc(&current_input)</code>,{' '}
          <code>&repeat(&current_input)</code>, <code>&time&</code>,{' '}
          <code>&recall(&current_input)</code>,{' '}
          <code>&imagine(&current_input)</code>.
        </p>
        <label htmlFor="node-context">Context magnet (Type A / B)</label>
        <input
          id="node-context"
          name="context"
          autoComplete="off"
          defaultValue={node?.contextId ?? ''}
          placeholder="e.g. recent output…"
        />
        <p>
          Custom automata use a bounded eight-operation tape interpreter.
          Programs stop after 10,000 instructions.
        </p>
      </details>
      {node && (
        <details className="advanced-fields">
          <summary>All entries & weighted votes</summary>
          <p>
            Optional JSON editor for additional entries, inhibition terms, and
            weighted alternatives. When supplied, this replaces the entry list
            above. Keep original patterns unique and in the same Pattern-ID
            class.
          </p>
          <label htmlFor="node-entries">Entry list (JSON)</label>
          <textarea
            id="node-entries"
            name="entries"
            autoComplete="off"
            rows={10}
            placeholder={JSON.stringify(node.entries, null, 2)}
          />
          <button
            type="button"
            className="text-button"
            onClick={(e) => {
              const area =
                e.currentTarget.parentElement?.querySelector('textarea');
              if (area) area.value = JSON.stringify(node.entries, null, 2);
            }}
          >
            Fill current entries
          </button>
        </details>
      )}
      {error && (
        <p role="alert" className="form-error">
          {error}
        </p>
      )}
      <button className="button primary full-width" type="submit">
        {node ? 'Save node' : 'Teach pattern'}
        <ArrowRight size={16} />
      </button>
    </form>
  );
}
