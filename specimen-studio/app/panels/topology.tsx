import { useEffect, useRef, useState } from 'react';
import {
  Activity,
  Boxes,
  Calculator,
  Check,
  Clock3,
  Lightbulb,
  MessageCircle,
  Pencil,
  RotateCcw,
  Shield,
  Sparkles,
} from 'lucide-react';
import {
  type Cycle,
  type Specimen,
  type SpecimenNode,
  type TraceStep,
  type Vote,
} from '@/lib/specimen/types';

export const icons = [
  Calculator,
  MessageCircle,
  RotateCcw,
  Clock3,
  Lightbulb,
  Sparkles,
  Activity,
  Shield,
];

export function Topology({
  nodes,
  selected,
  active,
  attachments,
  zoom,
  onSelect,
}: {
  nodes: SpecimenNode[];
  selected?: string;
  active: string[];
  attachments: Specimen['attachments'];
  zoom: number;
  onSelect: (id: string) => void;
}) {
  const viewport = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 640, height: 294 });
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      setSize({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (zoom === 1) viewport.current?.scrollTo({ left: 0, top: 0 });
  }, [zoom]);
  const priority = [
    ...nodes.filter((node) => node.ref === selected),
    ...nodes.filter(
      (node) => node.ref !== selected && active.includes(node.ref),
    ),
  ];
  const visible = [
    ...priority,
    ...nodes.filter((node) => !priority.includes(node)),
  ].slice(0, 16);
  const width = size.width * zoom;
  const height = size.height * zoom;
  const positions = visible.map((_, i) => {
    const angle = -Math.PI / 2 + (i * 2 * Math.PI) / visible.length;
    return {
      x: width / 2 + Math.cos(angle) * Math.max(0, width / 2 - 100),
      y: height / 2 + Math.sin(angle) * Math.max(0, height / 2 - 40),
    };
  });
  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- The named, focusable scroll region handles arrow-key scrolling across browser engines.
    <section
      className="topology"
      ref={viewport}
      // Keyboard users can scroll the zoomed graph without selecting a node.
      // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- Scroll containers require keyboard focus.
      tabIndex={0}
      aria-label="Scrollable specimen topology"
      onKeyDown={(event) => {
        const distance = 44;
        const delta = {
          ArrowLeft: { left: -distance },
          ArrowRight: { left: distance },
          ArrowUp: { top: -distance },
          ArrowDown: { top: distance },
        }[event.key];
        if (!delta) return;
        event.preventDefault();
        event.currentTarget.scrollBy(delta);
      }}
    >
      <div className="topology-stage" style={{ width, height }}>
        <svg
          className="topology-lines"
          viewBox={`0 0 ${width} ${height}`}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          {visible.map((n, i) => (
            <line
              key={n.ref}
              x1={width / 2}
              y1={height / 2}
              x2={positions[i].x}
              y2={positions[i].y}
              className={active.includes(n.ref) ? 'active-edge' : ''}
            />
          ))}
          {attachments.map((a) => {
            const from = visible.findIndex((n) => n.ref === a.from),
              to = visible.findIndex((n) => n.ref === a.to);
            return from >= 0 && to >= 0 ? (
              <line
                key={a.id}
                x1={positions[from].x}
                y1={positions[from].y}
                x2={positions[to].x}
                y2={positions[to].y}
                className="attachment-edge"
              />
            ) : null;
          })}
        </svg>
        <div className="orchestrator">
          <Boxes size={19} />
          Orchestration
        </div>
        {visible.map((n, i) => {
          const Icon = icons[i % icons.length];
          return (
            <button
              key={n.ref}
              className={`graph-node node-color-${i % 3} ${selected === n.ref ? 'selected' : ''} ${active.includes(n.ref) ? 'contributed' : ''}`}
              style={{ left: positions[i].x, top: positions[i].y }}
              onClick={() => onSelect(n.ref)}
              aria-label={`Inspect ${n.name}`}
              aria-pressed={selected === n.ref}
            >
              <span className="node-icon">
                <Icon size={21} />
              </span>
              <span className="node-label">{n.name}</span>
            </button>
          );
        })}
      </div>
      {!nodes.length && (
        <div className="network-empty">
          Add your first node in the Node Library.
        </div>
      )}
      {nodes.length > 16 && (
        <span className="graph-overflow">
          Showing 16 of {nodes.length} nodes. Browse all in the library.
        </span>
      )}
    </section>
  );
}

export function NodeInspector({
  node,
  vote,
  maxStrength,
  onEdit,
  onJitter,
}: {
  node?: SpecimenNode;
  vote?: Vote;
  maxStrength: number;
  onEdit: () => void;
  onJitter: () => void;
}) {
  if (!node)
    return (
      <aside className="panel inspector">
        <h2>Node inspector</h2>
        <p className="muted">Add a node to inspect its pattern and vote.</p>
      </aside>
    );
  return (
    <aside className="panel inspector">
      <div className="panel-heading">
        <h2>Node inspector</h2>
      </div>
      <div className="inspector-identity">
        <span className="identity-icon">
          <Calculator size={24} />
        </span>
        <div>
          <h3>{node.name}</h3>
          <span className="muted small">
            {node.type === 'pattern'
              ? 'Pattern node'
              : `Context node · Type ${node.type}`}
          </span>
        </div>
      </div>
      <div className="inspector-section">
        <div className="label-row">
          <span>Pattern ID</span>
          <code>{node.patternId}</code>
        </div>
        <div className="label-row">
          <span>Strength</span>
          <span>
            {node.strength} / {maxStrength}
          </span>
        </div>
        <div className="strength-bar">
          <span style={{ width: `${(node.strength / maxStrength) * 100}%` }} />
        </div>
        {node.strength === maxStrength && (
          <span className="crystal-label">Crystallized</span>
        )}
      </div>
      <div className="inspector-section">
        <span className="inspector-label">
          {vote ? 'Contributing pattern' : 'First stored pattern'}
        </span>
        <div className="code-box">
          {node.entries.find((entry) => entry.id === vote?.entryRef)?.pattern ??
            node.entries[0]?.pattern}
        </div>
        <span className="inspector-label">Action</span>
        <div className="code-box">
          {vote?.action ?? node.entries[0]?.alternatives[0]?.action}
        </div>
      </div>
      <div className="inspector-section">
        <div className="switch-label">
          Jitter enabled
          <button
            role="switch"
            aria-checked={node.jitter}
            aria-label={`Jitter for ${node.name}`}
            className={`toggle ${node.jitter ? 'on' : ''}`}
            onClick={onJitter}
          >
            <span />
          </button>
        </div>
        <button className="button primary full-width" onClick={onEdit}>
          <Pencil size={16} />
          Edit node
        </button>
      </div>
    </aside>
  );
}

export function Trace({
  steps,
  busy,
  outcome,
  input,
}: {
  steps: TraceStep[];
  busy: boolean;
  outcome: string;
  input: string;
}) {
  return (
    <section
      className="trace-list"
      // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- Scrollable trace output needs a keyboard focus target.
      tabIndex={0}
      aria-label="Cycle trace details"
    >
      {outcome !== 'idle' && (
        <output className="trace-outcome">
          {outcome === 'complete'
            ? 'Completed'
            : outcome === 'cancelled'
              ? 'Cancelled · partial changes discarded'
              : outcome === 'failed'
                ? 'Failed · partial changes discarded'
                : 'Processing'}
          : {input}
        </output>
      )}
      {steps.length ? (
        steps.map((step, i) => (
          <div key={step.id} className="trace-step">
            <span>{String(i + 1).padStart(2, '0')}</span>
            <div>
              <strong>{step.phase}</strong>
              <p>{step.detail}</p>
            </div>
            <Check size={16} />
          </div>
        ))
      ) : (
        <div className="trace-empty">
          <Activity size={28} />
          <h3>Every step has a reason.</h3>
          <p>Run a prompt to follow retrieval, voting, and composition.</p>
        </div>
      )}
      {busy && <output>Processing…</output>}
    </section>
  );
}
/* eslint-disable jsx-a11y/prefer-tag-over-role -- Inline SVG requires an image role; img cannot contain vector primitives. */

export function VisualOutput({
  visual,
}: {
  visual: NonNullable<Cycle['visual']>;
}) {
  return (
    <figure className="visual-output">
      {/* eslint-disable-next-line jsx-a11y/prefer-tag-over-role -- Inline SVG needs its image role and accessible name. */}
      <svg
        viewBox="-160 -90 320 180"
        role="img"
        aria-label="Ephemeral visual composition from the prompt’s structural features"
      >
        {visual.xArray.map((x, i) => (
          <circle
            key={i}
            cx={x}
            cy={visual.yArray[i]}
            r={3 + visual.brightnessArray[i] * 8}
            fill={visual.colorArray[i]}
            opacity={visual.brightnessArray[i]}
          />
        ))}
      </svg>
      <figcaption>Ephemeral visual study · fixed sparse synthesis</figcaption>
    </figure>
  );
}

// Secondary workspaces share the same specimen state and persistence contract.
/* eslint-enable jsx-a11y/prefer-tag-over-role */
