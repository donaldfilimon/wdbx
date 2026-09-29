import { useState } from 'react';
import { Check } from 'lucide-react';
import { Panel, WButton } from '@/components/wdbx';
import { type Settings, type Specimen } from '@/lib/specimen/types';

import { isDesktop } from '@/lib/specimen/native';

export function SettingsView({
  state,
  onSave,
}: {
  state: Specimen;
  onSave: (s: Settings, name: string) => void;
}) {
  const [draft, setDraft] = useState(state.settings),
    [name, setName] = useState(state.name);
  const fields: {
    key: keyof Settings;
    label: string;
    min: number;
    max: number;
    step?: number;
    help: string;
  }[] = [
    {
      key: 'voteThreshold',
      label: 'Vote threshold',
      min: -100,
      max: 100,
      help: 'Minimum confidence to contribute a vote.',
    },
    {
      key: 'jitter',
      label: 'Jitter magnitude',
      min: 0,
      max: 100,
      step: 0.1,
      help: 'Temporary offset. Endpoints and crystallized qualified scores stay fixed.',
    },
    {
      key: 'initialStrength',
      label: 'Initial strength',
      min: 1,
      max: 1000,
      help: 'Starting strength for newly learned nodes.',
    },
    {
      key: 'maxStrength',
      label: 'Maximum strength',
      min: 1,
      max: 1000,
      help: 'Nodes crystallize at this value.',
    },
    {
      key: 'entryLimit',
      label: 'Entries per node',
      min: 1,
      max: 1000,
      help: 'Maximum original pattern/vote pairings in one node.',
    },
    {
      key: 'scanLimit',
      label: 'Active scan ceiling',
      min: 1,
      max: 1000,
      help: 'Shared comparison limit, including attached nodes.',
    },
    {
      key: 'fanoutLimit',
      label: 'Fan-out ceiling',
      min: 1,
      max: 100,
      help: 'Maximum input variants per cycle.',
    },
    {
      key: 'maxNodes',
      label: 'Node population limit',
      min: 1,
      max: 100000,
      help: 'Maximum nodes stored in this workspace.',
    },
    {
      key: 'historyLimit',
      label: 'History record limit',
      min: 1,
      max: 100000,
      help: 'Oldest unpinned records roll off at this limit.',
    },
    {
      key: 'pinLimit',
      label: 'Pinned record limit',
      min: 1,
      max: 10000,
      help: 'Pins are preserved beyond rolling history.',
    },
    {
      key: 'maxRafts',
      label: 'Maximum Index Rafts',
      min: 1,
      max: 64,
      help: 'Cooperative traversal tasks for large collections.',
    },
    {
      key: 'chunkSize',
      label: 'Traversal chunk size',
      min: 1,
      max: 4096,
      help: 'Complete one chunk before starting another.',
    },
    {
      key: 'raftThreshold',
      label: 'Raft activation threshold',
      min: 1,
      max: 10000,
      help: 'Use simple traversal for smaller lists.',
    },
    {
      key: 'idleMin',
      label: 'Minimum idle interval (seconds)',
      min: 1,
      max: 86400,
      help: 'Draw a new random interval before every idle event.',
    },
    {
      key: 'idleMax',
      label: 'Maximum idle interval (seconds)',
      min: 1,
      max: 86400,
      help: 'Upper bound for the idle maintenance interval.',
    },
    {
      key: 'transientWidth',
      label: 'Transient synthesis width',
      min: 8,
      max: 128,
      help: 'Fixed sparse features used in ephemeral visual composition.',
    },
  ];
  const row =
    'grid gap-3 border-b border-line px-4 py-3 last:border-b-0 sm:grid-cols-[1fr_minmax(8rem,14rem)] sm:items-center';
  const label = 'text-sm font-semibold text-ink';
  const help = 'm-0 mt-0.5 text-xs text-muted-foreground';
  const field =
    'min-h-11 w-full rounded-lg border border-line-strong bg-raised px-3 text-sm text-ink focus-visible:outline-2 focus-visible:outline-focus';
  const check = 'size-5 justify-self-start accent-teal sm:justify-self-end';
  return (
    <form
      className="grid gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        onSave(draft, name);
      }}
    >
      <Panel title="Workspace">
        <div className={row}>
          <div>
            <label className={label} htmlFor="specimen-name">
              Specimen name
            </label>
            <p className={help}>A name for this saved collection.</p>
          </div>
          <input
            id="specimen-name"
            name="specimen-name"
            autoComplete="off"
            className={field}
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={100}
          />
        </div>
        {isDesktop() && (
          <div className={row}>
            <div>
              <label className={label} htmlFor="native-gpu">
                Accelerate transient computation
              </label>
              <p className={help}>
                Try the graphics processor; report CPU fallback when
                unavailable.
              </p>
            </div>
            <input
              id="native-gpu"
              type="checkbox"
              className={check}
              checked={!!draft.gpu}
              onChange={(e) => setDraft({ ...draft, gpu: e.target.checked })}
            />
          </div>
        )}
        {(['brainstorm', 'maintenance'] as const).map((key) => (
          <div className={row} key={key}>
            <div>
              <label className={label} htmlFor={`setting-${key}`}>
                {key === 'brainstorm' ? 'Brainstorm mode' : 'Idle maintenance'}
              </label>
              <p className={help}>
                {key === 'brainstorm'
                  ? 'Broader bounded jitter and synthesis variation.'
                  : 'Periodically choose a mutation scan or PHAGY while idle.'}
              </p>
            </div>
            <input
              type="checkbox"
              id={`setting-${key}`}
              className={check}
              checked={draft[key]}
              onChange={(e) => setDraft({ ...draft, [key]: e.target.checked })}
            />
          </div>
        ))}
      </Panel>
      <Panel
        title="Behavior & resource limits"
        description="Bounds are the kernel's native limits, shared by both editions."
      >
        {fields.map((f) => (
          <div className={row} key={f.key}>
            <div>
              <label className={label} htmlFor={`setting-${f.key}`}>
                {f.label}
              </label>
              <p className={help}>{f.help}</p>
            </div>
            <input
              id={`setting-${f.key}`}
              name={f.key}
              type="number"
              className={field}
              min={f.min}
              max={f.max}
              step={f.step ?? 1}
              value={Number(draft[f.key])}
              onChange={(e) =>
                setDraft({ ...draft, [f.key]: Number(e.target.value) })
              }
            />
          </div>
        ))}
      </Panel>
      <Panel title="Runtime">
        <p className="m-0 px-4 py-3 text-sm text-ink-soft">
          The Rust specimen kernel runs here as WebAssembly: edits on the page,
          cycles and reviews in a background worker. The browser keeps data in
          IndexedDB and uses a CPU-only default network; GPU acceleration,
          vision and model tools are desktop features.
        </p>
      </Panel>
      <div className="flex flex-wrap justify-end gap-2">
        <WButton
          onClick={() => {
            setDraft(state.settings);
            setName(state.name);
          }}
        >
          Discard changes
        </WButton>
        <WButton type="submit" variant="primary">
          <Check size={17} />
          Save settings
        </WButton>
      </div>
    </form>
  );
}
