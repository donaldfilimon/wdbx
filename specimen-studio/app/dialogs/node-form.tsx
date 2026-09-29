import { useState } from 'react';
import { ArrowRight } from 'lucide-react';
import {
  control,
  Disclosure,
  Field,
  FieldRow,
  FormError,
  WButton,
} from '@/components/wdbx';
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
      className="grid gap-3"
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
      <Field htmlFor="node-name" label="Node name">
        <input
          className={control}
          id="node-name"
          name="name"
          autoComplete="off"
          required
          maxLength={100}
          defaultValue={node?.name ?? ''}
          placeholder="e.g. Favorite color…"
        />
      </Field>
      <FieldRow>
        <Field htmlFor="node-type" label="Node type">
          <select
            className={control}
            id="node-type"
            name="type"
            defaultValue={node?.type ?? 'pattern'}
          >
            <option value="pattern">Pattern node</option>
            <option value="A">Type A · residual supervisor</option>
            <option value="B">Type B · idle observer</option>
          </select>
        </Field>
        <Field htmlFor="node-tone" label="Tone">
          <select
            className={control}
            id="node-tone"
            name="tone"
            defaultValue={node?.tone ?? 'neutral'}
          >
            {['neutral', 'warm', 'curious', 'cautious'].map((t) => (
              <option key={t}>{t}</option>
            ))}
          </select>
        </Field>
      </FieldRow>
      <Field htmlFor="node-pattern" label="Original pattern">
        <textarea
          className={`${control} min-h-20 font-mono`}
          id="node-pattern"
          name="pattern"
          spellCheck={false}
          autoComplete="off"
          required
          maxLength={2000}
          defaultValue={node?.entries[0]?.pattern ?? defaultPattern}
          placeholder="e.g. what is your favorite color…"
        />
      </Field>
      <Field htmlFor="node-action" label="Response or action">
        <textarea
          className={`${control} min-h-20 font-mono`}
          id="node-action"
          name="action"
          spellCheck={false}
          autoComplete="off"
          required
          maxLength={8000}
          defaultValue={node?.entries[0]?.alternatives[0]?.action ?? ''}
          placeholder="e.g. Forest green. Or a scoped action sigil…"
        />
      </Field>
      <Disclosure summary="Sigils & context">
        <p className="m-0">
          Use plain response text or <code>&calc(&current_input)</code>,{' '}
          <code>&repeat(&current_input)</code>, <code>&time&</code>,{' '}
          <code>&recall(&current_input)</code>,{' '}
          <code>&imagine(&current_input)</code>.
        </p>
        <Field htmlFor="node-context" label="Context magnet (Type A / B)">
          <input
            className={control}
            id="node-context"
            name="context"
            autoComplete="off"
            defaultValue={node?.contextId ?? ''}
            placeholder="e.g. recent output…"
          />
        </Field>
        <p className="m-0">
          Custom automata use a bounded eight-operation tape interpreter.
          Programs stop after 10,000 instructions.
        </p>
      </Disclosure>
      {node && (
        <Disclosure summary="All entries & weighted votes">
          <p className="m-0">
            Optional JSON editor for additional entries, inhibition terms, and
            weighted alternatives. When supplied, this replaces the entry list
            above. Keep original patterns unique and in the same Pattern-ID
            class.
          </p>
          <Field htmlFor="node-entries" label="Entry list (JSON)">
            <textarea
              className={`${control} font-mono`}
              id="node-entries"
              name="entries"
              autoComplete="off"
              rows={10}
              placeholder={JSON.stringify(node.entries, null, 2)}
            />
          </Field>
          <WButton
            variant="ghost"
            className="justify-self-start"
            onClick={() => {
              const area = document.getElementById('node-entries');
              if (area instanceof HTMLTextAreaElement)
                area.value = JSON.stringify(node.entries, null, 2);
            }}
          >
            Fill current entries
          </WButton>
        </Disclosure>
      )}
      {error && <FormError>{error}</FormError>}
      <WButton variant="primary" type="submit" className="w-full">
        {node ? 'Save node' : 'Teach pattern'}
        <ArrowRight aria-hidden="true" size={16} />
      </WButton>
    </form>
  );
}
