import { useState } from 'react';
import { Database, MessageCircle, Pencil, Pin, Plus, X } from 'lucide-react';
import { SUBSYSTEMS, type Specimen } from '@/lib/specimen/types';

import { VirtualList } from '@/components/virtual-list';
import { Empty } from './common';
import { SearchField } from './common';
import { clock } from '../state/format';

export function MemoryView({
  state,
  query,
  setQuery,
  onAdd,
  onEdit,
  onPin,
  onRemove,
}: {
  state: Specimen;
  query: string;
  setQuery: (v: string) => void;
  onAdd: () => void;
  onEdit: (id: string) => void;
  onPin: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  const [tab, setTab] = useState('resources'),
    [subsystem, setSubsystem] = useState('all');
  const resources = state.resources.filter(
    (r) =>
      (subsystem === 'all' || r.subsystem === subsystem) &&
      `${r.text} ${r.value}`.toLowerCase().includes(query.toLowerCase()),
  );
  const history = state.history
    .filter(
      (h) =>
        (tab !== 'pinned' || h.pinned) &&
        `${h.input} ${h.segments.map((x) => x.text).join(' ')}`
          .toLowerCase()
          .includes(query.toLowerCase()),
    )
    .slice()
    .reverse();
  return (
    <>
      <div className="workspace-tools">
        <fieldset className="segmented">
          <legend className="sr-only">Memory view</legend>
          {[
            ['resources', 'Resources'],
            ['history', 'Conversation'],
            ['pinned', 'Pinned'],
          ].map(([id, label]) => (
            <button
              key={id}
              className={tab === id ? 'selected' : ''}
              aria-pressed={tab === id}
              onClick={() => setTab(id)}
            >
              {label}
            </button>
          ))}
        </fieldset>
        <span className="toolbar-spacer" />
        {tab === 'resources' && (
          <button className="button primary" onClick={onAdd}>
            <Plus size={17} />
            Add memory
          </button>
        )}
      </div>
      <div className="workspace-tools">
        <SearchField
          value={query}
          onChange={setQuery}
          placeholder="Search your memory…"
        />
        {tab === 'resources' && (
          <select
            name="subsystem-filter"
            aria-label="Memory subsystem"
            value={subsystem}
            onChange={(e) => setSubsystem(e.target.value)}
          >
            <option value="all">All subsystems</option>
            {SUBSYSTEMS.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        )}
      </div>
      {tab === 'resources' ? (
        <div className="panel table-panel">
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Term</th>
                  <th>Supporting information</th>
                  <th>Subsystem</th>
                  <th>Linked node ID</th>
                  <th>
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {resources.slice(0, 100).map((r) => (
                  <tr key={r.ref}>
                    <td>
                      <strong>{r.text}</strong>
                    </td>
                    <td className="memory-value">{r.value}</td>
                    <td>
                      <span className="subsystem-tag">{r.subsystem}</span>
                    </td>
                    <td>
                      <code className="subtle-code">
                        {r.resourceId || 'Unlinked'}
                      </code>
                    </td>
                    <td>
                      <div className="row-actions">
                        <button
                          className="icon-button"
                          aria-label={`Edit memory ${r.text}`}
                          onClick={() => onEdit(r.ref)}
                        >
                          <Pencil size={16} />
                        </button>
                        <button
                          className="icon-button"
                          aria-label={`Remove memory ${r.text}`}
                          onClick={() => onRemove(r.ref)}
                        >
                          <X size={16} />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {!resources.length && (
            <Empty
              title="No matching memories"
              text="Add a term, definition, or contextual resource to support your specimen."
            />
          )}
        </div>
      ) : (
        <div className="history-list">
          <VirtualList
            key={`${query}:${tab}`}
            items={history}
            rowHeight={190}
            renderItem={(h) => (
              <article className="panel history-record" key={h.id}>
                <div className="history-heading">
                  <span>
                    <MessageCircle size={17} />
                    {h.input}
                  </span>
                  <button
                    className={`icon-button ${h.pinned ? 'teal' : ''}`}
                    aria-label={`${h.pinned ? 'Unpin' : 'Pin'} ${h.input}`}
                    onClick={() => onPin(h.id)}
                  >
                    <Pin size={17} />
                  </button>
                </div>
                <p>{h.segments.map((s) => s.text).join('\n\n')}</p>
                <small>
                  {clock(h.createdAt)} · {h.votes.length} contributors ·{' '}
                  {h.status}
                </small>
              </article>
            )}
          />
          {!history.length && (
            <Empty
              title={
                tab === 'pinned'
                  ? 'Nothing pinned yet'
                  : 'Your conversations will live here'
              }
              text={
                tab === 'pinned'
                  ? 'Pin a response in the studio to retain it beyond ordinary history.'
                  : 'Run your first prompt in the studio.'
              }
            />
          )}
        </div>
      )}
      <div className="workspace-footnote">
        <Database size={17} />
        <p>
          Resources are indexed at insertion. Only relevant retrieved
          information enters orchestration.
        </p>
      </div>
    </>
  );
}
