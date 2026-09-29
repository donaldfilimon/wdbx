import { useState } from 'react';
import { Panel, Toolbar, ToolbarSpacer, WButton } from '@/components/wdbx';
import { Database, MessageCircle, Pencil, Pin, Plus, X } from 'lucide-react';
import { SUBSYSTEMS, type Specimen } from '@/lib/specimen/types';

import { VirtualList } from '@/components/virtual-list';
import { Empty } from './common';
import { SearchField, tableCls, tdCls, thCls } from './common';
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
      <Toolbar label="Memory views">
        <fieldset className="m-0 inline-flex gap-1 rounded-lg border border-line bg-surface-2 p-1">
          <legend className="sr-only">Memory view</legend>
          {[
            ['resources', 'Resources'],
            ['history', 'Conversation'],
            ['pinned', 'Pinned'],
          ].map(([id, label]) => (
            <button
              key={id}
              type="button"
              className={`min-h-9 rounded-md px-3 text-sm font-medium focus-visible:outline-2 focus-visible:outline-focus ${tab === id ? 'bg-raised text-ink shadow-sm' : 'text-muted-foreground hover:text-ink'}`}
              aria-pressed={tab === id}
              onClick={() => setTab(id)}
            >
              {label}
            </button>
          ))}
        </fieldset>
        <ToolbarSpacer />
        {tab === 'resources' && (
          <WButton variant="primary" onClick={onAdd}>
            <Plus size={17} />
            Add memory
          </WButton>
        )}
      </Toolbar>
      <Toolbar label="Memory search">
        <SearchField
          value={query}
          onChange={setQuery}
          placeholder="Search your memory…"
        />
        {tab === 'resources' && (
          <select
            name="subsystem-filter"
            className="min-h-11 rounded-lg border border-line-strong bg-raised px-3 text-sm text-ink focus-visible:outline-2 focus-visible:outline-focus"
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
      </Toolbar>
      {tab === 'resources' ? (
        <Panel title="Resources" description={`${resources.length} results`}>
          <div className="overflow-auto">
            <table className={tableCls}>
              <thead>
                <tr>
                  <th className={thCls}>Term</th>
                  <th className={thCls}>Supporting information</th>
                  <th className={thCls}>Subsystem</th>
                  <th className={thCls}>Linked node ID</th>
                  <th className={thCls}>
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {resources.slice(0, 100).map((r) => (
                  <tr key={r.ref} className="hover:bg-surface">
                    <td className={tdCls}>
                      <strong className="text-ink">{r.text}</strong>
                    </td>
                    <td className={`${tdCls} leading-relaxed text-ink-soft`}>
                      {r.value}
                    </td>
                    <td className={tdCls}>
                      <span className="inline-block rounded bg-surface-2 px-2 py-1 text-xs text-ink-soft">
                        {r.subsystem}
                      </span>
                    </td>
                    <td className={tdCls}>
                      <code className="rounded bg-surface-2 px-1.5 py-1 text-xs text-muted-foreground">
                        {r.resourceId || 'Unlinked'}
                      </code>
                    </td>
                    <td className={tdCls}>
                      <div className="flex gap-1">
                        <WButton
                          variant="ghost"
                          size="icon"
                          aria-label={`Edit memory ${r.text}`}
                          onClick={() => onEdit(r.ref)}
                        >
                          <Pencil aria-hidden="true" size={16} />
                        </WButton>
                        <WButton
                          variant="ghost"
                          size="icon"
                          aria-label={`Remove memory ${r.text}`}
                          onClick={() => onRemove(r.ref)}
                        >
                          <X aria-hidden="true" size={16} />
                        </WButton>
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
        </Panel>
      ) : (
        <div className="grid gap-4">
          <VirtualList
            key={`${query}:${tab}`}
            items={history}
            rowHeight={190}
            renderItem={(h) => (
              <article
                className="history-record h-full overflow-hidden rounded-xl border border-line bg-raised px-5 py-4"
                key={h.id}
              >
                <div className="history-heading flex items-center justify-between gap-4 font-semibold text-ink">
                  <span className="flex min-w-0 items-center gap-3">
                    <MessageCircle
                      aria-hidden="true"
                      size={17}
                      className="shrink-0"
                    />
                    <span className="truncate">{h.input}</span>
                  </span>
                  <WButton
                    variant="ghost"
                    size="icon"
                    className={h.pinned ? 'text-teal-strong' : undefined}
                    aria-label={`${h.pinned ? 'Unpin' : 'Pin'} ${h.input}`}
                    onClick={() => onPin(h.id)}
                  >
                    <Pin aria-hidden="true" size={17} />
                  </WButton>
                </div>
                <p className="my-3 line-clamp-3 whitespace-pre-wrap text-ink-soft [overflow-wrap:anywhere]">
                  {h.segments.map((s) => s.text).join('\n\n')}
                </p>
                <small className="text-xs text-muted-foreground">
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
      <div className="mx-1 my-5 flex items-start gap-2.5 text-sm text-muted-foreground">
        <Database aria-hidden="true" size={17} className="mt-0.5 shrink-0" />
        <p className="m-0">
          Resources are indexed at insertion. Only relevant retrieved
          information enters orchestration.
        </p>
      </div>
    </>
  );
}
