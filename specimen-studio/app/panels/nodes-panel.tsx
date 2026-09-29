import { useState } from 'react';
import { Panel, Toolbar, ToolbarSpacer, WButton } from '@/components/wdbx';
import { GitBranch, Network, Pencil, Plus, Shield, X } from 'lucide-react';
import { type Specimen } from '@/lib/specimen/types';

import { Empty } from './common';
import { SearchField } from './common';

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
      <div className="panel table-panel">
        <div className="panel-heading">
          <h2>Learned nodes</h2>
          <span className="muted small">{nodes.length} results</span>
        </div>
        <div
          key={`${query}:${filter}`}
          className="table-scroll virtual-table"
          onScroll={(e) =>
            setNodeStart(
              Math.max(0, Math.floor(e.currentTarget.scrollTop / 76) - 2),
            )
          }
        >
          <table>
            <thead>
              <tr>
                <th>Node</th>
                <th>Original pattern</th>
                <th>Pattern ID</th>
                <th>Strength</th>
                <th>Entries</th>
                <th>
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
                    style={{
                      height: Math.min(nodeStart, nodes.length) * 76,
                      padding: 0,
                      border: 0,
                    }}
                  />
                </tr>
              )}
              {nodes.slice(nodeStart, nodeStart + 24).map((n, i) => (
                <tr key={n.ref} className="virtual-node-row">
                  <td aria-label={n.name}>
                    <button
                      aria-label={`Inspect ${n.name}`}
                      className="node-name-button"
                      onClick={() => onSelect(n.ref)}
                    >
                      <span className={`table-node-icon node-color-${i % 3}`}>
                        <Network size={17} />
                      </span>
                      <span>
                        <strong>{n.name}</strong>
                        <small>
                          {n.type === 'pattern'
                            ? 'Pattern node'
                            : `Context · Type ${n.type}`}
                        </small>
                      </span>
                    </button>
                  </td>
                  <td className="pattern-cell">
                    <code>{n.entries[0]?.pattern}</code>
                  </td>
                  <td>
                    <code className="subtle-code">{n.patternId}</code>
                  </td>
                  <td
                    aria-label={`Strength ${n.strength} of ${state.settings.maxStrength}`}
                  >
                    <div className="table-strength">
                      <span>
                        {n.strength} / {state.settings.maxStrength}
                      </span>
                      <div className="strength-bar">
                        <span
                          style={{
                            width: `${(n.strength / state.settings.maxStrength) * 100}%`,
                          }}
                        />
                      </div>
                    </div>
                  </td>
                  <td>{n.entries.length}</td>
                  <td>
                    <div className="row-actions">
                      <button
                        className="icon-button"
                        aria-label={`Edit ${n.name}`}
                        onClick={() => onEdit(n.ref)}
                      >
                        <Pencil size={16} />
                      </button>
                      <button
                        className="icon-button"
                        aria-label={`Remove ${n.name}`}
                        onClick={() => onRemove(n.ref)}
                      >
                        <X size={16} />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
              {nodes.length > nodeStart + 24 && (
                <tr aria-hidden="true">
                  <td
                    aria-label="Unrendered rows"
                    colSpan={6}
                    style={{
                      height: (nodes.length - nodeStart - 24) * 76,
                      padding: 0,
                      border: 0,
                    }}
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
          <p className="table-note">
            Scroll through all matching nodes, or refine your search.
          </p>
        )}
      </div>
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
