import { useState } from 'react';
import { Check } from 'lucide-react';
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
      max: 10,
      step: 0.1,
      help: 'Temporary offset. Endpoints and crystallized qualified scores stay fixed.',
    },
    {
      key: 'initialStrength',
      label: 'Initial strength',
      min: 1,
      max: 100,
      help: 'Starting strength for newly learned nodes.',
    },
    {
      key: 'maxStrength',
      label: 'Maximum strength',
      min: 1,
      max: 100,
      help: 'Nodes crystallize at this value.',
    },
    {
      key: 'entryLimit',
      label: 'Entries per node',
      min: 1,
      max: 100,
      help: 'Maximum original pattern/vote pairings in one node.',
    },
    {
      key: 'scanLimit',
      label: 'Active scan ceiling',
      min: 1,
      max: 10000,
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
      max: 10000,
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
      max: 1000,
      help: 'Pins are preserved beyond rolling history.',
    },
    {
      key: 'maxRafts',
      label: 'Maximum Index Rafts',
      min: 1,
      max: 32,
      help: 'Cooperative traversal tasks for large collections.',
    },
    {
      key: 'chunkSize',
      label: 'Traversal chunk size',
      min: 1,
      max: 10000,
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
      min: 10,
      max: 3600,
      help: 'Draw a new random interval before every idle event.',
    },
    {
      key: 'idleMax',
      label: 'Maximum idle interval (seconds)',
      min: 10,
      max: 7200,
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
  return (
    <form
      className="settings-form"
      onSubmit={(e) => {
        e.preventDefault();
        onSave(draft, name);
      }}
    >
      <section className="panel settings-section">
        <div className="panel-heading">
          <h2>Workspace</h2>
        </div>
        <div className="setting-row">
          <div>
            <label htmlFor="specimen-name">Specimen name</label>
            <p>A name for this saved collection.</p>
          </div>
          <input
            id="specimen-name"
            name="specimen-name"
            autoComplete="off"
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={100}
          />
        </div>
        {isDesktop() && (
          <div className="setting-row">
            <div>
              <label htmlFor="native-gpu">
                Accelerate transient computation
              </label>
              <p>
                Try the graphics processor; report CPU fallback when
                unavailable.
              </p>
            </div>
            <input
              id="native-gpu"
              type="checkbox"
              checked={!!draft.gpu}
              onChange={(e) => setDraft({ ...draft, gpu: e.target.checked })}
            />
          </div>
        )}
        {(['brainstorm', 'maintenance'] as const).map((key) => (
          <div className="setting-row" key={key}>
            <div>
              <label htmlFor={`setting-${key}`}>
                {key === 'brainstorm' ? 'Brainstorm mode' : 'Idle maintenance'}
              </label>
              <p>
                {key === 'brainstorm'
                  ? 'Broader bounded jitter and synthesis variation.'
                  : 'Periodically choose a mutation scan or PHAGY while idle.'}
              </p>
            </div>
            <input
              type="checkbox"
              id={`setting-${key}`}
              checked={draft[key]}
              onChange={(e) => setDraft({ ...draft, [key]: e.target.checked })}
            />
          </div>
        ))}
      </section>
      <section className="panel settings-section">
        <div className="panel-heading">
          <h2>Behavior & resource limits</h2>
        </div>
        {fields.map((f) => (
          <div className="setting-row" key={f.key}>
            <div>
              <label htmlFor={`setting-${f.key}`}>{f.label}</label>
              <p>{f.help}</p>
            </div>
            <input
              id={`setting-${f.key}`}
              name={f.key}
              type="number"
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
      </section>
      <div className="runtime-note">
        <h2>Browser runtime profile</h2>
        <p>
          One cooperative JavaScript execution lane. Device storage uses
          IndexedDB. Visual synthesis uses fixed features, with no online
          learning. GPU/NPU model acceleration and learned vision are
          architecture-level extensions, not active controls in this studio.
        </p>
      </div>
      <div className="settings-actions">
        <button
          type="button"
          className="button outline"
          onClick={() => {
            setDraft(state.settings);
            setName(state.name);
          }}
        >
          Discard changes
        </button>
        <button type="submit" className="button primary">
          <Check size={17} />
          Save settings
        </button>
      </div>
    </form>
  );
}
