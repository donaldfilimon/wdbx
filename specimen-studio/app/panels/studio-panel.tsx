/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */
import {
  Activity,
  ArrowRight,
  Check,
  FlaskConical,
  GitBranch,
  Maximize2,
  MessageCircle,
  Network,
  Pin,
  Plus,
  Sparkles,
  ThumbsDown,
  ThumbsUp,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import { togglePin } from '@/lib/specimen/kernel';

import { NodeInspector } from './topology';
import type { StudioModel } from '../state/use-studio';
import { Topology } from './topology';
import { Trace } from './topology';
import type { View } from '../state/navigation';
import { VisualOutput } from './topology';
import { clock } from '../state/format';
import { fmt } from '../state/format';

export function StudioPanel({
  m,
  nav,
}: {
  m: StudioModel;
  nav: (v: View, section?: number) => void;
}) {
  const {
    state,
    commit,
    act,
    openDialog,
    setEditing,
    saveStatus,
    setSelected,
    selectedNode,
    prompt,
    setPrompt,
    promptRef,
    busy,
    activeTrace,
    runOutcome,
    runInput,
    networkMode,
    setNetworkMode,
    topologyZoom,
    setTopologyZoom,
    showProvenance,
    setShowProvenance,
    cycle,
    feedbackUnavailable,
    maxConfidence,
    run,
    giveFeedback,
    abortRef,
  } = m;
  return (
    <>
      <div className="studio-console" aria-busy={busy}>
        <section className="console-telemetry" aria-label="Workspace overview">
          <div className="console-identity">
            <span className={`status-beacon ${busy ? 'is-running' : ''}`}>
              <Activity size={16} aria-hidden="true" />
            </span>
            <div>
              <span className="telemetry-label">Runtime</span>
              <strong>
                {busy ? 'Cycle in progress' : 'Local · system ready'}
              </strong>
            </div>
          </div>
          <div className="telemetry-stat">
            <span className="telemetry-label">Pattern nodes</span>
            <strong>{state.nodes.length}</strong>
          </div>
          <div className="telemetry-stat">
            <span className="telemetry-label">Memory records</span>
            <strong>{state.resources.length}</strong>
          </div>
          <div className="telemetry-stat">
            <span className="telemetry-label">Last confidence</span>
            <strong>
              {maxConfidence === null ? '—' : fmt.format(maxConfidence)}
            </strong>
          </div>
          <div className="telemetry-stat telemetry-storage">
            <span className="telemetry-label">Workspace</span>
            <strong>
              {saveStatus.replace(
                'Stored on this device',
                'Local · stored on device',
              )}
            </strong>
          </div>
        </section>
        <nav className="workflow-rail" aria-label="Specimen workflow">
          <button
            className="workflow-step"
            onClick={() => {
              setEditing(undefined);
              openDialog('node');
            }}
          >
            <span className="workflow-index">01</span>
            <span>
              <strong>Teach pattern</strong>
              <small>Shape reusable behavior</small>
            </span>
            <Plus size={16} aria-hidden="true" />
          </button>
          <button
            className="workflow-step is-primary"
            onClick={() => promptRef.current?.focus()}
          >
            <span className="workflow-index">02</span>
            <span>
              <strong>Prompt specimen</strong>
              <small>Run a local cycle</small>
            </span>
            <ArrowRight size={16} aria-hidden="true" />
          </button>
          <button
            className="workflow-step"
            onClick={() => {
              setNetworkMode('trace');
              document.querySelector('.network-panel')?.scrollIntoView({
                behavior: window.matchMedia('(prefers-reduced-motion: reduce)')
                  .matches
                  ? 'auto'
                  : 'smooth',
                block: 'center',
              });
            }}
          >
            <span className="workflow-index">03</span>
            <span>
              <strong>Inspect trace</strong>
              <small>Follow every phase</small>
            </span>
            <Activity size={16} aria-hidden="true" />
          </button>
          <button
            className="workflow-step"
            onClick={() => {
              if (cycle) setShowProvenance(true);
              else nav('activity');
            }}
          >
            <span className="workflow-index">04</span>
            <span>
              <strong>Refine memory</strong>
              <small>Review evidence and learn</small>
            </span>
            <Sparkles size={16} aria-hidden="true" />
          </button>
        </nav>
        <form
          className="composer"
          onSubmit={(e) => {
            e.preventDefault();
            void run(prompt);
          }}
        >
          <div className="composer-heading">
            <div>
              <strong>Prompt specimen</strong>
              <span>Provide input and context to the local cycle.</span>
            </div>
            <span>
              {state.settings.brainstorm
                ? 'Brainstorm mode'
                : 'Conservative mode'}
            </span>
          </div>
          <div className="composer-row">
            <MessageCircle size={21} aria-hidden="true" />
            <label htmlFor="prompt" className="sr-only">
              Prompt your specimen
            </label>
            <input
              ref={promptRef}
              id="prompt"
              name="prompt"
              autoComplete="off"
              placeholder="Give your specimen a prompt…"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              maxLength={8000}
            />
            {busy ? (
              <button
                type="button"
                className="button outline"
                onClick={() => abortRef.current?.abort()}
              >
                Cancel
              </button>
            ) : (
              <button className="button primary" type="submit">
                Run cycle
                <ArrowRight size={18} />
              </button>
            )}
          </div>
          <div className="composer-suggestions">
            <span>Starter prompts</span>
            {[
              'What is 2 + 2?',
              'Say hello 3 times',
              'What can you recall?',
            ].map((starter) => (
              <button
                key={starter}
                type="button"
                disabled={busy}
                onClick={() => void run(starter)}
              >
                {starter}
              </button>
            ))}
          </div>
        </form>
        <div className="studio-grid">
          <section className="panel conversation-panel">
            <div className="panel-heading">
              <h2>Latest result</h2>
              <span className="toolbar-spacer" />
              {cycle && (
                <>
                  <button
                    className={`text-button ${showProvenance ? 'teal' : ''}`}
                    aria-label="Run dossier"
                    aria-expanded={showProvenance}
                    aria-controls="run-dossier"
                    onClick={() => setShowProvenance(!showProvenance)}
                  >
                    <GitBranch size={15} />
                    Run dossier
                  </button>
                  <button
                    className={`icon-button ${cycle.pinned ? 'teal' : ''}`}
                    aria-label={cycle.pinned ? 'Unpin answer' : 'Pin answer'}
                    onClick={() =>
                      act(() => commit(togglePin(state, cycle.id)))
                    }
                  >
                    <Pin size={16} />
                  </button>
                </>
              )}
            </div>
            {cycle ? (
              <div className="answer-body">
                <p className="result-context">
                  Saved result · {cycle.input} · {clock(cycle.createdAt)}
                </p>
                <div
                  id="run-dossier"
                  hidden={!showProvenance}
                  className="dossier-summary"
                >
                  Vote scores are engine rankings, not calibrated probabilities.
                </div>
                {cycle.visual && <VisualOutput visual={cycle.visual} />}
                <div className="answer-segments">
                  {cycle.segments.map((seg) => (
                    <div
                      className={`answer-segment ${showProvenance ? `provenance-${seg.color}` : ''}`}
                      key={seg.id}
                    >
                      <div className="answer-text">{seg.text}</div>
                      {showProvenance && (
                        <div className="provenance-detail">
                          <span className="segment-color">{seg.color}</span>
                          <span>
                            {seg.contributors
                              .map(
                                (ref) =>
                                  state.nodes.find((n) => n.ref === ref)
                                    ?.name ?? 'Removed node',
                              )
                              .join(', ') || 'System response'}
                          </span>
                          <span> · {seg.transformations.join(' → ')}</span>
                          {seg.contributors.length > 0 && (
                            <span className="segment-feedback">
                              <button
                                aria-label={`Right ${seg.color} segment`}
                                disabled={feedbackUnavailable(seg.contributors)}
                                onClick={() => giveFeedback(true, seg.color)}
                              >
                                <ThumbsUp size={14} />
                              </button>
                              <button
                                aria-label={`Wrong ${seg.color} segment`}
                                disabled={feedbackUnavailable(seg.contributors)}
                                onClick={() => giveFeedback(false, seg.color)}
                              >
                                <ThumbsDown size={14} />
                              </button>
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
                {showProvenance && cycle.visual?.synthesis && (
                  <details>
                    <summary>Transient computation evidence</summary>
                    <pre>{JSON.stringify(cycle.visual.synthesis, null, 2)}</pre>
                  </details>
                )}
                {showProvenance && cycle.votes.length > 0 && (
                  <details className="native-evidence">
                    <summary>Comparison and retrieval evidence</summary>
                    {cycle.votes.map((v) => (
                      <div key={v.id}>
                        <button
                          className="text-button"
                          onClick={() => setSelected(v.nodeRef)}
                        >
                          {state.nodes.find((n) => n.ref === v.nodeRef)?.name ??
                            'Removed node'}
                        </button>
                        <pre
                          // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- axe requires scrollable regions to be keyboard-focusable.
                          tabIndex={0}
                          aria-label="Vote evidence (JSON)"
                        >
                          {JSON.stringify(
                            {
                              input: v.input,
                              matchedPattern: state.nodes
                                .find((n) => n.ref === v.nodeRef)
                                ?.entries.find(
                                  (entry) => entry.id === v.entryRef,
                                )?.pattern,
                              action: v.action,
                              baseScore: v.base,
                              confidenceScore: v.confidence,
                              strength: v.strength,
                              evidence: v.evidence,
                              binding: v.binding,
                              resources: v.resources.map(
                                (ref) =>
                                  state.resources.find(
                                    (resource) => resource.ref === ref,
                                  ) ?? {
                                    id: ref,
                                    status: 'Record no longer available',
                                  },
                              ),
                              origin: v.origin,
                              group: v.group,
                            },
                            null,
                            2,
                          )}
                        </pre>
                      </div>
                    ))}
                  </details>
                )}
                <div className="answer-footer">
                  <span className="muted small">
                    {cycle.votes.length
                      ? `${[...new Set(cycle.votes.map((v) => state.nodes.find((n) => n.ref === v.nodeRef)?.name ?? 'Removed node'))].join(', ')} contributed · confidence ${fmt.format(Math.max(...cycle.votes.map((v) => v.confidence)))}`
                      : 'No node contributed'}
                  </span>
                  <div className="feedback-actions">
                    <button
                      className="button outline small-button"
                      disabled={feedbackUnavailable(
                        cycle.segments.flatMap(
                          (segment) => segment.contributors,
                        ),
                      )}
                      onClick={() => giveFeedback(true)}
                    >
                      <ThumbsUp size={15} />
                      Right
                    </button>
                    <button
                      className="button outline small-button"
                      disabled={feedbackUnavailable(
                        cycle.segments.flatMap(
                          (segment) => segment.contributors,
                        ),
                      )}
                      onClick={() => giveFeedback(false)}
                    >
                      <ThumbsDown size={15} />
                      Wrong
                    </button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="empty-answer">
                <FlaskConical size={24} />
                <div>
                  <strong>Your specimen is ready.</strong>
                  <p>Run a starter prompt or teach it something of your own.</p>
                </div>
              </div>
            )}
          </section>
          <section className="panel network-panel">
            <div className="panel-heading">
              <div className="panel-title-stack">
                <h2>Live topology</h2>
                <span className="muted small">
                  {Math.min(16, state.nodes.length)} of {state.nodes.length}{' '}
                  nodes · schematic layout
                </span>
              </div>
              <div className="network-controls">
                <fieldset className="segmented">
                  <legend className="sr-only">Topology view</legend>
                  <button
                    type="button"
                    className={networkMode === 'network' ? 'selected' : ''}
                    onClick={() => setNetworkMode('network')}
                    aria-pressed={networkMode === 'network'}
                  >
                    <Network size={15} />
                    Network
                  </button>
                  <button
                    type="button"
                    className={networkMode === 'trace' ? 'selected' : ''}
                    onClick={() => setNetworkMode('trace')}
                    aria-pressed={networkMode === 'trace'}
                  >
                    <Activity size={15} />
                    Cycle trace
                  </button>
                </fieldset>
                {networkMode === 'network' && (
                  <fieldset className="zoom-controls">
                    <legend className="sr-only">Topology zoom</legend>
                    <button
                      type="button"
                      aria-label="Zoom out topology"
                      disabled={topologyZoom <= 0.8}
                      onClick={() =>
                        setTopologyZoom((value) =>
                          Math.max(0.8, Number((value - 0.1).toFixed(2))),
                        )
                      }
                    >
                      <ZoomOut size={15} />
                    </button>
                    <button
                      type="button"
                      className="zoom-fit"
                      aria-label="Fit topology to canvas"
                      title={`Current zoom ${Math.round(topologyZoom * 100)} percent`}
                      onClick={() => setTopologyZoom(1)}
                    >
                      <Maximize2 size={14} />
                      <span>{Math.round(topologyZoom * 100)}%</span>
                    </button>
                    <button
                      type="button"
                      aria-label="Zoom in topology"
                      disabled={topologyZoom >= 1.3}
                      onClick={() =>
                        setTopologyZoom((value) =>
                          Math.min(1.3, Number((value + 0.1).toFixed(2))),
                        )
                      }
                    >
                      <ZoomIn size={15} />
                    </button>
                  </fieldset>
                )}
              </div>
            </div>
            {networkMode === 'network' ? (
              <Topology
                nodes={state.nodes}
                selected={selectedNode?.ref}
                active={cycle?.votes.map((v) => v.nodeRef) ?? []}
                attachments={state.attachments}
                zoom={topologyZoom}
                onSelect={setSelected}
              />
            ) : (
              <Trace
                steps={activeTrace}
                busy={busy}
                outcome={runOutcome}
                input={runInput}
              />
            )}
            <div className="graph-legend">
              <span>
                <span className="legend-spectrum" aria-hidden="true">
                  <i className="legend-dot green" />
                  <i className="legend-dot amber" />
                  <i className="legend-dot violet" />
                </span>
                Pattern nodes
              </span>
              <span>
                <i className="legend-edge" aria-hidden="true" />
                Attachment
              </span>
              <span>
                <i className="legend-dot solid" />
                Orchestration
              </span>
              <span className="muted legend-tail">
                Select a node to inspect it
              </span>
            </div>
          </section>
          <NodeInspector
            node={selectedNode}
            vote={cycle?.votes.find(
              (vote) => vote.nodeRef === selectedNode?.ref,
            )}
            maxStrength={state.settings.maxStrength}
            onEdit={() => {
              setEditing(selectedNode?.ref);
              openDialog('node');
            }}
            onJitter={() =>
              act(() => {
                const s = structuredClone(state),
                  n = s.nodes.find((x) => x.ref === selectedNode?.ref);
                if (n) n.jitter = !n.jitter;
                commit(s);
              })
            }
          />
        </div>
        <div className="cycle-phase-strip" aria-label="Completed cycle phases">
          {[
            'Prepare',
            'Retrieve',
            'Index Rafts',
            'Deep scan',
            'Vote',
            'Compose',
          ].map((phase, index) => {
            const complete = activeTrace.some((step) => step.phase === phase);

            return (
              <span key={phase} className={complete ? 'is-complete' : ''}>
                <i>{complete ? <Check size={12} /> : index + 1}</i>
                {phase}
              </span>
            );
          })}
        </div>
        <footer className="studio-footer">
          <span>{saveStatus}</span>
          <a
            href="?view=specification"
            onClick={(e) => {
              e.preventDefault();
              nav('specification');
            }}
          >
            View specification
            <ArrowRight size={15} />
          </a>
        </footer>
      </div>
    </>
  );
}
