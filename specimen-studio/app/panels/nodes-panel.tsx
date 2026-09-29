import { useState } from 'react';
import { Panel, Toolbar, ToolbarSpacer, WButton } from '@/components/wdbx';
import { GitBranch, Network, Pencil, Plus, Shield, X } from 'lucide-react';
import { type Specimen } from '@/lib/specimen/types';

import { Empty } from './common';
import {
  SearchField,
  tableCls as table,
  tdCls as td,
  thCls as th,
} from './common';

/** Rendered row height in px; the windowing math depends on it. */
const ROW = 76;
const TINTS = [
  'bg-info-soft text-teal',
  'bg-amber-soft text-amber',
  'bg-surface-2 text-violet',
];

export function NodesView({
  state,
  query,
  setQuery,
  onAdd,
  onEdit,
  onRemove,
  onAttach,
  onSelect,
}: {
  state: Specimen;
  query: string;
  setQuery: (v: string) => void;
  onAdd: () => void;
  onEdit: (ref: string) => void;
  onRemove: (ref: string) => void;
  onAttach: () => void;
  onSelect: (ref: string) => void;
}) {
  const [filter, setFilter] = useState('all');
  const [nodeStart, setNodeStart] = useState(0);
  const nodes = state.nodes.filter(
    (n) =>
      (filter === 'all' || n.type === filter) &&
      (
        n.name +
        ' ' +
        n.patternId +
        ' ' +
        n.entries.map((e) => e.pattern).join(' ')
      )
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  return (
    <>
      <Toolbar label="Node library tools">
        <SearchField
          value={query}
          onChange={(value) => {
            setNodeStart(0);
            setQuery(value);
          }}
          placeholder="Find a pattern or node…"
        />
        <select
          name="node-type-filter"
          className="min-h-11 rounded-lg border border-line-strong bg-raised px-3 text-sm text-ink focus-visible:outline-2 focus-visible:outline-focus"
          aria-label="Filter node type"
          value={filter}
          onChange={(e) => {
            setNodeStart(0);
            setFilter(e.target.value);
          }}
        >
          <option value="all">All node types</option>
          <option value="pattern">Pattern</option>
          <option value="A">Type A · residual</option>
          <option value="B">Type B · idle</option>
        </select>
        <ToolbarSpacer />
        <WButton onClick={onAttach} disabled={state.nodes.length < 2}>
          <GitBranch size={16} />
          Attach nodes
        </WButton>
        <WButton variant="primary" onClick={onAdd}>
          <Plus size={17} />
          Add pattern
        </WButton>
      </Toolbar>
      <Panel title="Learned nodes" description={`${nodes.length} results`}>
        <div
          key={`${query}:${filter}`}
          // Rows are exactly ROW px tall: the spacer rows and the scroll
          // handler window the list with that height.
          className="max-h-[600px] overflow-auto"
          onScroll={(e) =>
            setNodeStart(
              Math.max(0, Math.floor(e.currentTarget.scrollTop / ROW) - 2),
            )
          }
        >
          <table className={table}>
            <thead className="sticky top-0 z-[1] bg-raised">
              <tr>
                <th className={th}>Node</th>
                <th className={th}>Original pattern</th>
                <th className={th}>Pattern ID</th>
                <th className={th}>Strength</th>
                <th className={th}>Entries</th>
                <th className={th}>
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {nodeStart > 0 && (
                <tr aria-hidden="true">
                  <td
                    aria-label="Unrendered rows"
                    colSpan={6}
                    className="border-0 p-0"
                    style={{ height: Math.min(nodeStart, nodes.length) * ROW }}
                  />
                </tr>
              )}
              {nodes.slice(nodeStart, nodeStart + 24).map((n, i) => (
                <tr key={n.ref} className="h-[76px] hover:bg-surface">
                  <td className={`${td} min-w-52`} aria-label={n.name}>
                    <button
                      aria-label={`Inspect ${n.name}`}
                      className="flex min-h-11 max-w-60 items-center gap-3 rounded-md text-left focus-visible:outline-2 focus-visible:outline-focus"
                      onClick={() => onSelect(n.ref)}
                    >
                      <span
                        aria-hidden="true"
                        className={`grid size-9 shrink-0 place-items-center rounded-lg ${TINTS[i % 3]}`}
                      >
                        <Network size={17} />
                      </span>
                      <span className="min-w-0">
                        <strong className="block truncate text-sm font-semibold text-ink">
                          {n.name}
                        </strong>
                        <small className="mt-0.5 block truncate text-xs text-muted-foreground">
                          {n.type === 'pattern'
                            ? 'Pattern node'
                            : `Context · Type ${n.type}`}
                        </small>
                      </span>
                    </button>
                  </td>
                  <td className={td}>
                    <code className="block max-w-60 truncate text-xs text-ink-soft">
                      {n.entries[0]?.pattern}
                    </code>
                  </td>
                  <td className={td}>
                    <code className="block max-w-60 truncate rounded bg-surface-2 px-1.5 py-1 text-xs text-muted-foreground">
                      {n.patternId}
                    </code>
                  </td>
                  <td
                    className={td}
                    aria-label={`Strength ${n.strength} of ${state.settings.maxStrength}`}
                  >
                    <div className="min-w-[75px] text-xs">
                      <span>
                        {n.strength} / {state.settings.maxStrength}
                      </span>
                      <div className="mt-2 h-1 overflow-hidden rounded-full bg-surface-2">
                        <span
                          className="block h-full rounded-full bg-teal"
                          style={{
                            width: `${(n.strength / state.settings.maxStrength) * 100}%`,
                          }}
                        />
                      </div>
                    </div>
                  </td>
                  <td className={td}>{n.entries.length}</td>
                  <td className={td}>
                    <div className="flex gap-1">
                      <WButton
                        variant="ghost"
                        size="icon"
                        aria-label={`Edit ${n.name}`}
                        onClick={() => onEdit(n.ref)}
                      >
                        <Pencil aria-hidden="true" size={16} />
                      </WButton>
                      <WButton
                        variant="ghost"
                        size="icon"
                        aria-label={`Remove ${n.name}`}
                        onClick={() => onRemove(n.ref)}
                      >
                        <X aria-hidden="true" size={16} />
                      </WButton>
                    </div>
                  </td>
                </tr>
              ))}
              {nodes.length > nodeStart + 24 && (
                <tr aria-hidden="true">
                  <td
                    aria-label="Unrendered rows"
                    colSpan={6}
                    className="border-0 p-0"
                    style={{ height: (nodes.length - nodeStart - 24) * ROW }}
                  />
                </tr>
              )}
            </tbody>
          </table>
        </div>
        {!nodes.length && (
          <Empty
            title="No matching nodes"
            text="Try another search, or teach your specimen a new pattern."
          />
        )}
        {nodes.length > 100 && (
          <p className="m-0 p-3.5 text-xs text-muted-foreground">
            Scroll through all matching nodes, or refine your search.
          </p>
        )}
      </Panel>
      <p className="m-0 flex items-start gap-2 text-sm text-ink-soft">
        <Shield size={17} className="mt-0.5 shrink-0 text-teal" />
        Pattern IDs may repeat. Every node keeps its own original patterns,
        votes, and strength.
      </p>
      <Panel
        title="Semantic attachments"
        description={`${state.attachments.length} links`}
      >
        {state.attachments.length ? (
          <ul className="m-0 list-none divide-y divide-line p-0">
            {state.attachments.map((a) => (
              <li
                className="flex flex-wrap items-center gap-2 px-4 py-3 text-sm text-ink"
                key={a.id}
              >
                <GitBranch size={17} className="text-teal" />
                <strong>
                  {state.nodes.find((n) => n.ref === a.from)?.name}
                </strong>
                <span aria-hidden="true">{a.bidirectional ? '↔' : '→'}</span>
                <span className="sr-only">
                  {a.bidirectional ? 'linked both ways with' : 'links to'}
                </span>
                <strong>{state.nodes.find((n) => n.ref === a.to)?.name}</strong>
                <span className="text-muted-foreground">
                  {a.hard
                    ? 'Hard attachment'
                    : `${Math.round(a.affinity * 100)}% affinity`}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="m-0 px-4 py-3 text-sm text-muted-foreground">
            Connect related nodes to hand off the part of an input that did not
            match.
          </p>
        )}
      </Panel>
    </>
  );
}
