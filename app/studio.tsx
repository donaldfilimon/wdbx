'use client';
/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Activity,
  ArrowDownToLine,
  ArrowRight,
  BookOpen,
  Boxes,
  Calculator,
  Check,
  ChevronDown,
  Clock3,
  Database,
  FileText,
  FlaskConical,
  GitBranch,
  Lightbulb,
  Menu,
  MessageCircle,
  Network,
  Pencil,
  Pin,
  Plus,
  RotateCcw,
  Save,
  Search,
  Settings2,
  Shield,
  Sparkles,
  ThumbsDown,
  ThumbsUp,
  Upload,
  X,
} from 'lucide-react';
import {
  addNode,
  feedback,
  log,
  maintenance,
  observeContext,
  removeNode,
  runCycle,
  saveResource,
  seedSpecimen,
  togglePin,
  validateSettings,
  validateSpecimen,
} from '@/lib/specimen/engine';
import {
  downloadText,
  loadWorkspace,
  persistWorkspace,
} from '@/lib/specimen/storage';
import {
  SUBSYSTEMS,
  uid,
  type Cycle,
  type Resource,
  type Settings,
  type Specimen,
  type SpecimenNode,
  type TraceStep,
} from '@/lib/specimen/types';
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';

import sections from '@/lib/specification.json';
import { VirtualList } from '@/components/virtual-list';
import { NativeLab } from '@/components/native-lab';
import { isDesktop, nativeReview, exportNative, importNative } from '@/lib/specimen/native';
import { useStudioTools } from '@/lib/webmcp';

type View =
  | 'studio'
  | 'nodes'
  | 'memory'
  | 'activity'
  | 'specification'
  | 'settings'
  | 'lab';
const views: { id: View; label: string; icon: typeof Network }[] = [
  { id: 'studio', label: 'Studio', icon: Network },
  { id: 'nodes', label: 'Node Library', icon: BookOpen },
  { id: 'memory', label: 'Memory', icon: Database },
  { id: 'activity', label: 'Activity', icon: Activity },
  { id: 'lab', label: 'Vision & models', icon: Sparkles },
  { id: 'specification', label: 'Specification', icon: FileText },
];
const icons = [
  Calculator,
  MessageCircle,
  RotateCcw,
  Clock3,
  Lightbulb,
  Sparkles,
  Activity,
  Shield,
];
const labels: Record<View, [string, string]> = {
  studio: ['Specimen studio', 'Teach a pattern. Trace a response.'],
  lab: ['Vision & local models', 'Inspect an image. Compose locally. Learn deliberately.'],
  nodes: ['Node library', 'Small patterns, reusable actions.'],
  memory: ['Memory', 'Supporting knowledge, close at hand.'],
  activity: ['Activity', 'Follow the changes that shape your specimen.'],
  specification: [
    'Specification',
    'The complete architecture, alongside your specimen.',
  ],
  settings: ['Settings', 'Set the boundaries for your specimen.'],
};
const formText = (form: FormData, key: string): string => {
  const v = form.get(key);
  if (typeof v !== 'string') throw new Error(`Missing field: ${key}`);
  return v;
};
const fmt = new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 });
const clock = (s: string) =>
  new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(s));

export default function Studio() {
  const [state, setState] = useState<Specimen>(() => seedSpecimen());
  const current = useRef(state);
  const [storageEnabled, setStorageEnabled] = useState(false);
  const [hydrated, setHydrated] = useState(false),
    [saveStatus, setSaveStatus] = useState('Opening workspace…'),
    [view, setView] = useState<View>('studio'),
    [mobileNav, setMobileNav] = useState(false),
    [selected, setSelected] = useState(''),
    [query, setQuery] = useState(''),
    [prompt, setPrompt] = useState(''),
    [busy, setBusy] = useState(false),
    [trace, setTrace] = useState<TraceStep[]>([]),
    [networkMode, setNetworkMode] = useState('network'),
    [notice, setNotice] = useState(''),
    [error, setError] = useState(''),
    [showProvenance, setShowProvenance] = useState(false),
    [chapter, setChapter] = useState(1),
    [dialog, setDialog] = useState<
      'node' | 'resource' | 'load' | 'reset' | 'attach' | null
    >(null),
    [editing, setEditing] = useState<string | undefined>(),
    [pendingLoad, setPendingLoad] = useState<Specimen | null>(null);
  const fileRef = useRef<HTMLInputElement>(null),
    abortRef = useRef<AbortController | null>(null),
    lastInteraction = useRef(0),
    noticeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined),
    undo = useRef<Specimen | null>(null);
  const selectedNode =
    state.nodes.find((n) => n.ref === selected) ?? state.nodes[0];
  const cycle = state.history.at(-1);
  const announce = useCallback((message: string) => {
    setNotice(message);
    clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(''), 6000);
  }, []);
  const commit = useCallback((s: Specimen) => {
    current.current = s;
    setState(s);
    lastInteraction.current = Date.now();
  }, []);
  const act = useCallback((fn: () => void) => {
    try {
      setError('');
      fn();
    } catch (e) {
      setError(
        e instanceof Error ? e.message : 'This action could not complete.',
      );
    }
  }, []);
  const nav = useCallback((v: View, section?: number) => {
    setView(v);
    setQuery('');
    setMobileNav(false);
    if (section) setChapter(section);
    const params = new URLSearchParams();
    params.set('view', v);
    if (section) params.set('chapter', String(section));
    window.history.pushState({}, '', `?${params}`);
    window.scrollTo({ top: 0, behavior: 'instant' });
  }, []);
  useEffect(() => {
    const restore = () => {
      const p = new URLSearchParams(location.search),
        v = p.get('view') as View;
      if ([...views.map((x) => x.id), 'settings'].includes(v)) setView(v);
      const c = Number(p.get('chapter'));
      if (c >= 1 && c <= 26) setChapter(c);
    };
    queueMicrotask(restore);
    lastInteraction.current = Date.now();
    window.addEventListener('popstate', restore);
    loadWorkspace()
      .then((saved) => {
        if (saved) commit(saved);
        setStorageEnabled(true);
        setSaveStatus('Stored on this device');
      })
      .catch((e) => {
        setError(
          `${e.message} Automatic saving is paused to preserve existing data. You can download this session or explicitly load a valid specimen.`,
        );
        setSaveStatus('Automatic saving paused');
      })
      .finally(() => setHydrated(true));
    return () => {
      window.removeEventListener('popstate', restore);
      clearTimeout(noticeTimer.current);
    };
  }, [commit]);
  useEffect(() => {
    if (!hydrated || !storageEnabled) return;
    queueMicrotask(() => setSaveStatus('Saving…'));
    let live = true;
    const t = setTimeout(() => {
      persistWorkspace(state)
        .then(() => {
          if (live) setSaveStatus('Stored on this device');
        })
        .catch((e) => {
          if (live) {
            setSaveStatus('Not saved');
            setError(e.message);
          }
        });
    }, 250);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [state, hydrated, storageEnabled]);
  useEffect(() => {
    const before = (e: BeforeUnloadEvent) => {
      if (
        saveStatus === 'Saving…' ||
        saveStatus === 'Not saved' ||
        saveStatus === 'Automatic saving paused'
      ) {
        e.preventDefault();
      }
    };
    window.addEventListener('beforeunload', before);
    return () => window.removeEventListener('beforeunload', before);
  }, [saveStatus]);
  useEffect(() => {
    if (!hydrated || !state.settings.maintenance || busy) return;
    const interval =
      (state.settings.idleMin +
        Math.random() * (state.settings.idleMax - state.settings.idleMin)) *
      1000;
    const timer = setInterval(() => {
      if (
        Date.now() - lastInteraction.current >= state.settings.idleMin * 1000 &&
        !abortRef.current
      ) {
        if (isDesktop()) {
          const controller = new AbortController(); abortRef.current = controller;
          void nativeReview(current.current, 'maintenance', Math.random() < .5 ? 'phagy' : 'mutation', controller.signal).then(commit).catch((e) => setError(e.message)).finally(() => { if (abortRef.current === controller) abortRef.current = null; });
          return;
        }
        const snapshot = current.current,
          next = maintenance(snapshot);
        if (next.nodes.some((n) => n.type !== 'pattern'))
          void observeContext(next)
            .then((reviewed) => {
              if (current.current === snapshot && !abortRef.current)
                commit(reviewed);
            })
            .catch((e) => setError(e.message));
        else commit(next);
      }
    }, interval);
    return () => clearInterval(timer);
  }, [state, hydrated, busy, commit]);
  const reviewContext = useCallback(async () => {
    if (busy || abortRef.current) return;
    const controller = new AbortController();
    abortRef.current = controller;
    const snapshot = current.current;
    setBusy(true);
    try {
      const next = isDesktop() ? await nativeReview(snapshot, 'review', undefined, controller.signal) : await observeContext(snapshot, controller.signal);
      if (current.current !== snapshot)
        throw new Error(
          'The workspace changed during review. Run context review again.',
        );
      commit(next);
      announce('Context review completed.');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Review could not complete.');
    } finally {
      setBusy(false);
      abortRef.current = null;
    }
  }, [busy, commit, announce]);
  const run = useCallback(
    async (raw: string) => {
      if (!hydrated || busy) return;
      if (abortRef.current) { abortRef.current.abort(); abortRef.current = null; }
      const input = raw.trim();
      if (!input) return;
      setError('');
      lastInteraction.current = Date.now();
      if (input === '/contributorfractal') {
        setShowProvenance(true);
        setPrompt('');
        return;
      }
      if (/^\/(right|wrong|wrng)(?:\s|$)/.test(input)) {
        act(() => {
          const [cmd, color] = input.slice(1).split(/\s+/);
          const c = current.current.history.at(-1);
          if (!c) throw new Error('Run a cycle before giving feedback.');
          const result = feedback(
            current.current,
            c.id,
            cmd === 'right',
            color,
          );
          commit(result.state);
          announce(result.message);
          setPrompt('');
        });
        return;
      }
      if (input.startsWith('/saveSpecimen')) {
        if (isDesktop()) { void persistWorkspace(current.current).then(exportNative).then(saved=>saved&&announce('Portable specimen saved.')).catch(e=>setError(e.message)); setPrompt(''); return; }
        downloadText('specimen.json', JSON.stringify(current.current, null, 2));
        announce('Specimen downloaded.');
        setPrompt('');
        return;
      }
      if (input.startsWith('/loadSpecimen')) {
        if (isDesktop()) { void importNative().then((snapshot) => { if (snapshot?.specimen) commit(snapshot.specimen); }).catch((e) => setError(e.message)); } else fileRef.current?.click();
        return;
      }
      if (input.startsWith('/learn')) {
        const id = input.slice(6).trim();
        const p =
          current.current.proposals.find(
            (p) => p.id === id || p.pattern === id,
          ) ?? current.current.proposals.find((p) => p.status === 'pending');
        if (!p) {
          setError(
            'No pending learning proposal. Open Activity to run a context review.',
          );
          return;
        }
        setPrompt(p.pattern);
        setEditing(undefined);
        setDialog('node');
        return;
      }
      if (input.startsWith('/addPattern')) {
        setEditing(undefined);
        setDialog('node');
        return;
      }
      const controller = new AbortController();
      abortRef.current = controller;
      setBusy(true);
      setTrace([]);
      try {
        const snapshot = current.current;
        const result = await runCycle(
          snapshot,
          input.replace(/^\/prompt\s+/, ''),
          setTrace,
          controller.signal,
        );
        if (current.current !== snapshot)
          throw new Error(
            'The workspace changed during this cycle. Run the prompt again.',
          );
        const finalState = result.state.nodes.some((n) => n.type === 'A')
          ? await observeContext(result.state, controller.signal)
          : result.state;
        if (current.current !== snapshot)
          throw new Error(
            'Workspace changed during residual review. Retry the prompt.',
          );
        commit(finalState);
        setPrompt('');
        setSelected(result.cycle.votes[0]?.nodeRef ?? '');
      } catch (e) {
        if (e instanceof Error && e.name !== 'AbortError') setError(e.message);
        else announce('Cycle cancelled. No partial changes were saved.');
      } finally {
        setBusy(false);
        abortRef.current = null;
      }
    },
    [act, announce, busy, commit, hydrated],
  );
  useStudioTools({
    read: () => current.current,
    openChapter: (n) => nav('specification', n),
    run: async (input) => {
      if (!hydrated || busy || abortRef.current)
        throw new Error('Wait for the current workspace action to finish.');
      const snapshot = current.current,
        controller = new AbortController();
      abortRef.current = controller;
      setBusy(true);
      setTrace([]);
      try {
        const result = await runCycle(
          snapshot,
          input,
          setTrace,
          controller.signal,
        );
        if (current.current !== snapshot)
          throw new Error(
            'Workspace changed during the cycle. Retry the prompt.',
          );
        commit(result.state);
        nav('studio');
        return result.cycle;
      } finally {
        setBusy(false);
        abortRef.current = null;
      }
    },
  });
  const giveFeedback = (right: boolean, color?: string) =>
    act(() => {
      if (!cycle) return;
      const result = feedback(state, cycle.id, right, color);
      commit(result.state);
      announce(result.message);
    });
  const save = () => {
    if (isDesktop()) { void persistWorkspace(current.current).then(exportNative).then((saved) => saved && announce('Portable specimen saved.')).catch((e) => setError(e.message)); return; }
    downloadText(
      `${state.name.replace(/[^a-z0-9]/gi, '-').toLowerCase()}.json`,
      JSON.stringify(state, null, 2),
    );
    announce('Complete specimen downloaded.');
  };
  const load = async (file?: File) => {
    if (!file) return;
    try {
      if (file.size > 20 * 1024 * 1024)
        throw new Error('Choose a specimen file smaller than 20 MB.');
      const loaded = validateSpecimen(JSON.parse(await file.text()));
      setPendingLoad(loaded);
      setDialog('load');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not read the specimen.');
    } finally {
      if (fileRef.current) fileRef.current.value = '';
    }
  };
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Skip to main content
      </a>
      <aside className={`sidebar ${mobileNav ? 'is-open' : ''}`}>
        <a
          className="brand"
          href="?view=studio"
          onClick={(e) => {
            e.preventDefault();
            nav('studio');
          }}
        >
          <Boxes size={39} strokeWidth={1.4} />
          <span>
            <strong translate="no">WDBX</strong>
            <small>Specimen Studio</small>
          </span>
        </a>
        <button
          className="icon-button mobile-close"
          aria-label="Close navigation"
          onClick={() => setMobileNav(false)}
        >
          <X />
        </button>
        <nav aria-label="Main navigation">
          {views.map(({ id, label, icon: Icon }) => (
            <a
              key={id}
              href={`?view=${id}`}
              className={`nav-link ${view === id ? 'active' : ''}`}
              aria-current={view === id ? 'page' : undefined}
              onClick={(e) => {
                if (!e.metaKey && !e.ctrlKey) {
                  e.preventDefault();
                  nav(id);
                }
              }}
            >
              <Icon size={20} />
              {label}
              {id === 'nodes' && (
                <span className="nav-count">{state.nodes.length}</span>
              )}
            </a>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <a
            className={`nav-link ${view === 'settings' ? 'active' : ''}`}
            href="?view=settings"
            onClick={(e) => {
              e.preventDefault();
              nav('settings');
            }}
          >
            <Settings2 size={20} />
            Settings
          </a>
          <div className="workspace-caption">
            <span className="status-dot" />
            <div>
              Local workspace<small>Your data stays on this device</small>
            </div>
          </div>
        </div>
      </aside>
      {mobileNav && (
        <button
          aria-label="Close navigation overlay"
          className="nav-backdrop"
          onClick={() => setMobileNav(false)}
        />
      )}
      <main id="main" className="main-area">
        <header className="page-header">
          <div className="heading-wrap">
            <button
              className="icon-button mobile-menu"
              aria-label="Open navigation"
              onClick={() => setMobileNav(true)}
            >
              <Menu />
            </button>
            <div>
              <h1>{labels[view][0]}</h1>
              <p>{labels[view][1]}</p>
            </div>
          </div>
          <div className="header-actions">
            {view === 'specification' ? (
              <a
                className="button outline"
                href="/WDBX-Specimen-Architecture-Specification.md"
                download
              >
                <ArrowDownToLine size={17} />
                Download Markdown
              </a>
            ) : (
              <>
                <button
                  className="button outline"
                  onClick={() => fileRef.current?.click()}
                  disabled={busy}
                >
                  <Upload size={17} />
                  Load
                </button>
                <button className="button primary" onClick={save}>
                  <Save size={17} />
                  <span>Save specimen</span>
                </button>
              </>
            )}
          </div>
        </header>
        <input
          ref={fileRef}
          className="sr-only"
          type="file"
          accept=".json,application/json"
          aria-label="Load specimen file"
          onChange={(e) => void load(e.target.files?.[0])}
        />
        {error && (
          <div className="error-banner" role="alert">
            <span>{error}</span>
            <button
              className="icon-button"
              aria-label="Dismiss error"
              onClick={() => setError('')}
            >
              <X size={16} />
            </button>
          </div>
        )}
        {view === 'studio' && (
          <>
            <div className="specimen-toolbar">
              <span className="specimen-name">
                <FileText size={18} />
                {state.name}
              </span>
              <span className="toolbar-divider" />
              <span className="ready-state">
                <span className={`status-dot ${busy ? 'busy' : ''}`} />
                {busy ? 'Processing' : 'Ready'}
              </span>
              <span className="toolbar-divider" />
              <button
                className="text-button"
                onClick={() => setDialog('reset')}
              >
                Starter specimen
                <ChevronDown size={15} />
              </button>
              <span className="toolbar-spacer" />
              <span className="muted small">
                {state.settings.brainstorm
                  ? 'Brainstorm mode'
                  : 'Conservative mode'}
              </span>
            </div>
            <div className="studio-grid">
              <section className="panel network-panel">
                <div className="panel-heading">
                  <h2>Live topology</h2>
                  <span className="muted small">
                    {state.nodes.length} nodes · 3 resolution tiers
                  </span>
                  <div className="segmented" aria-label="Topology view">
                    <button
                      className={networkMode === 'network' ? 'selected' : ''}
                      onClick={() => setNetworkMode('network')}
                      aria-pressed={networkMode === 'network'}
                    >
                      <Network size={15} />
                      Network
                    </button>
                    <button
                      className={networkMode === 'trace' ? 'selected' : ''}
                      onClick={() => setNetworkMode('trace')}
                      aria-pressed={networkMode === 'trace'}
                    >
                      <Activity size={15} />
                      Cycle trace
                    </button>
                  </div>
                </div>
                {networkMode === 'network' ? (
                  <Topology
                    nodes={state.nodes}
                    selected={selectedNode?.ref}
                    active={cycle?.votes.map((v) => v.nodeRef) ?? []}
                    attachments={state.attachments}
                    onSelect={setSelected}
                  />
                ) : (
                  <Trace
                    steps={busy ? trace : (cycle?.trace ?? [])}
                    busy={busy}
                  />
                )}
                <div className="graph-legend">
                  <span>
                    <i className="legend-dot green" />
                    Pattern node
                  </span>
                  <span>
                    <i className="legend-dot amber" />
                    Context node
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
                maxStrength={state.settings.maxStrength}
                onEdit={() => {
                  setEditing(selectedNode?.ref);
                  setDialog('node');
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
              <section className="panel conversation-panel">
                <div className="panel-heading">
                  <h2>Try your specimen</h2>
                  <span className="toolbar-spacer" />
                  {cycle && (
                    <>
                      <button
                        className={`text-button ${showProvenance ? 'teal' : ''}`}
                        onClick={() => setShowProvenance(!showProvenance)}
                      >
                        <GitBranch size={15} />
                        Contributors
                      </button>
                      <button
                        className={`icon-button ${cycle.pinned ? 'teal' : ''}`}
                        aria-label={
                          cycle.pinned ? 'Unpin answer' : 'Pin answer'
                        }
                        onClick={() =>
                          act(() => commit(togglePin(state, cycle.id)))
                        }
                      >
                        <Pin size={16} />
                      </button>
                    </>
                  )}
                </div>
                <div className="suggestions">
                  {[
                    'What is 2 + 2?',
                    'Say hello 3 times',
                    'What can you recall?',
                  ].map((p) => (
                    <button key={p} disabled={busy} onClick={() => void run(p)}>
                      {p}
                    </button>
                  ))}
                </div>
                {cycle ? (
                  <div className="answer-body">
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
                                    onClick={() =>
                                      giveFeedback(true, seg.color)
                                    }
                                  >
                                    <ThumbsUp size={14} />
                                  </button>
                                  <button
                                    aria-label={`Wrong ${seg.color} segment`}
                                    onClick={() =>
                                      giveFeedback(false, seg.color)
                                    }
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
                    {showProvenance && cycle.visual?.synthesis && <details><summary>Transient computation evidence</summary><pre>{JSON.stringify(cycle.visual.synthesis,null,2)}</pre></details>}
                    {showProvenance && cycle.votes.some(v => v.evidence) && <details className="native-evidence"><summary>Comparison and retrieval evidence</summary>{cycle.votes.map(v => <div key={v.id}><strong>{state.nodes.find(n=>n.ref===v.nodeRef)?.name ?? 'Removed node'}</strong><pre>{JSON.stringify({evidence:v.evidence,binding:v.binding,resources:v.resources,origin:v.origin,group:v.group},null,2)}</pre></div>)}</details>}
                    <div className="answer-footer">
                      <span className="muted small">
                        {cycle.votes.length
                          ? `${[...new Set(cycle.votes.map((v) => state.nodes.find((n) => n.ref === v.nodeRef)?.name ?? 'Removed node'))].join(', ')} contributed · confidence ${fmt.format(Math.max(...cycle.votes.map((v) => v.confidence)))}`
                          : 'No node contributed'}
                      </span>
                      <div className="feedback-actions">
                        <button
                          className="button outline small-button"
                          disabled={
                            !cycle.votes.length ||
                            cycle.feedback.includes('all')
                          }
                          onClick={() => giveFeedback(true)}
                        >
                          <ThumbsUp size={15} />
                          Right
                        </button>
                        <button
                          className="button outline small-button"
                          disabled={
                            !cycle.votes.length ||
                            cycle.feedback.includes('all')
                          }
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
                      <p>
                        Run a starter prompt or teach it something of your own.
                      </p>
                    </div>
                  </div>
                )}
              </section>
            </div>
            <form
              className="composer"
              onSubmit={(e) => {
                e.preventDefault();
                void run(prompt);
              }}
            >
              <MessageCircle size={22} />
              <label htmlFor="prompt" className="sr-only">
                Prompt your specimen
              </label>
              <input
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
                  <ArrowRight size={19} />
                </button>
              )}
            </form>
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
          </>
        )}
        {view === 'lab' && <NativeLab specimen={state} onSnapshot={commit} />}
        {view === 'nodes' && (
          <NodesView
            state={state}
            query={query}
            setQuery={setQuery}
            onAdd={() => {
              setEditing(undefined);
              setDialog('node');
            }}
            onEdit={(ref) => {
              setEditing(ref);
              setDialog('node');
            }}
            onRemove={(ref) =>
              act(() => {
                const previous = state;
                commit(removeNode(state, ref));
                announce('Node removed. Use Undo to restore it.');
                undo.current = previous;
              })
            }
            onAttach={() => setDialog('attach')}
            onSelect={(ref) => {
              setSelected(ref);
              nav('studio');
            }}
          />
        )}
        {view === 'memory' && (
          <MemoryView
            state={state}
            query={query}
            setQuery={setQuery}
            onAdd={() => {
              setEditing(undefined);
              setDialog('resource');
            }}
            onEdit={(ref) => {
              setEditing(ref);
              setDialog('resource');
            }}
            onPin={(id) => act(() => commit(togglePin(state, id)))}
            onRemove={(ref) =>
              act(() => {
                undo.current = state;
                const s = structuredClone(state);
                s.resources = s.resources.filter((r) => r.ref !== ref);
                commit(s);
                announce('Memory removed. Use Undo to restore it.');
              })
            }
          />
        )}
        {view === 'activity' && (
          <ActivityView
            state={state}
            busy={busy}
            onReview={() => void reviewContext()}
            onMaintenance={(mode) =>
              act(() => {
                if (isDesktop()) { void nativeReview(current.current, 'maintenance', mode).then(commit).then(() => announce('Maintenance completed.')).catch((e) => setError(e.message)); } else { commit(maintenance(state, mode)); announce('Maintenance completed.'); }
              })
            }
            onLearn={(pattern) => {
              setPrompt(pattern);
              setEditing(undefined);
              setDialog('node');
            }}
          />
        )}
        {view === 'specification' && (
          <Specification
            chapter={chapter}
            query={query}
            setQuery={setQuery}
            onChapter={(n) => nav('specification', n)}
          />
        )}
        {view === 'settings' && (
          <SettingsView
            key={state.name + JSON.stringify(state.settings)}
            state={state}
            onSave={(settings, name) =>
              act(() => {
                validateSettings(settings);
                if (
                  state.nodes.some(
                    (n) =>
                      n.entries.length > settings.entryLimit ||
                      n.strength > settings.maxStrength,
                  )
                )
                  throw new Error(
                    'Existing nodes exceed those limits. Adjust them before reducing the limits.',
                  );
                const next = { ...state, settings, name: name.trim() };
                validateSpecimen(next);
                commit(next);
                announce('Settings saved.');
              })
            }
          />
        )}
      </main>
      <div aria-live="polite" className={`toast ${notice ? 'visible' : ''}`}>
        <Check size={17} />
        <span>{notice}</span>
        {notice.includes('Undo') && (
          <button
            onClick={() => {
              if (undo.current) {
                commit(undo.current);
                undo.current = null;
                announce('Restored.');
              }
            }}
          >
            Undo
          </button>
        )}
      </div>
      <Dialog
        open={dialog !== null}
        onOpenChange={(open) => {
          if (!open) setDialog(null);
        }}
      >
        <DialogContent className="studio-dialog">
          <DialogTitle>
            {dialog === 'node'
              ? editing
                ? 'Edit node'
                : 'Teach a pattern'
              : dialog === 'resource'
                ? editing
                  ? 'Edit memory'
                  : 'Add a memory'
                : dialog === 'load'
                  ? 'Load this specimen?'
                  : dialog === 'attach'
                    ? 'Attach two nodes'
                    : 'Start a fresh specimen?'}
          </DialogTitle>
          <DialogDescription>
            {dialog === 'node'
              ? 'A pattern activates a node. Its vote defines the action.'
              : dialog === 'resource'
                ? 'Store supporting information in a scoped, indexed resource.'
                : dialog === 'load'
                  ? 'This replaces the active workspace. Download your current specimen first if you want to keep it.'
                  : dialog === 'attach'
                    ? 'Delegate unmatched input to a related node.'
                    : 'This replaces your workspace with the starter specimen. Save your current work first.'}
          </DialogDescription>
          {dialog === 'node' && (
            <NodeForm
              node={state.nodes.find((n) => n.ref === editing)}
              defaultPattern={!editing ? prompt : ''}
              onSubmit={(value) => {
                {
                  const next = addNode(state, value, editing);
                  const proposal = next.proposals.find(
                    (p) => p.pattern === value.pattern,
                  );
                  if (proposal) proposal.status = 'learned';
                  commit(next);
                  setDialog(null);
                  announce(editing ? 'Node updated.' : 'Pattern learned.');
                }
              }}
            />
          )}
          {dialog === 'resource' && (
            <ResourceForm
              resource={state.resources.find((r) => r.ref === editing)}
              state={state}
              onSubmit={(value) => {
                commit(saveResource(state, value, editing));
                setDialog(null);
                announce('Memory saved.');
              }}
            />
          )}
          {dialog === 'load' && pendingLoad && (
            <>
              <div className="load-summary">
                <strong>{pendingLoad.name}</strong>
                <p>
                  {pendingLoad.nodes.length} nodes ·{' '}
                  {pendingLoad.resources.length} memories ·{' '}
                  {pendingLoad.history.length} conversations
                </p>
              </div>
              <div className="modal-actions">
                <button className="button outline" onClick={save}>
                  Save current specimen
                </button>
                <button
                  className="button primary"
                  onClick={() => {
                    commit(pendingLoad);
                    setStorageEnabled(true);
                    setDialog(null);
                    setSelected('');
                    announce('Specimen loaded.');
                  }}
                >
                  Load specimen
                </button>
              </div>
            </>
          )}
          {dialog === 'reset' && (
            <div className="modal-actions">
              <button className="button outline" onClick={save}>
                Save current specimen
              </button>
              <button
                className="button primary"
                onClick={() => {
                  commit(seedSpecimen());
                  setStorageEnabled(true);
                  setDialog(null);
                  setSelected('');
                  announce('Starter specimen ready.');
                }}
              >
                Create starter specimen
              </button>
            </div>
          )}
          {dialog === 'attach' && (
            <AttachmentForm
              state={state}
              onSave={(a) => {
                const s = structuredClone(state);
                s.attachments.push({ ...a, id: uid() });
                log(
                  s,
                  'attachment',
                  'Nodes attached',
                  'A semantic handoff was configured.',
                );
                commit(s);
                setDialog(null);
                announce('Attachment added.');
              }}
            />
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Topology({
  nodes,
  selected,
  active,
  attachments,
  onSelect,
}: {
  nodes: SpecimenNode[];
  selected?: string;
  active: string[];
  attachments: Specimen['attachments'];
  onSelect: (id: string) => void;
}) {
  const visible = nodes.slice(0, 16);
  const positions = visible.map((_, i) => {
    const angle = -Math.PI / 2 + (i * 2 * Math.PI) / visible.length;
    return { x: 50 + Math.cos(angle) * 35, y: 49 + Math.sin(angle) * 32 };
  });
  return (
    <div className="topology">
      <svg
        className="topology-lines"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        {visible.map((n, i) => (
          <line
            key={n.ref}
            x1="50"
            y1="49"
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
            style={{ left: `${positions[i].x}%`, top: `${positions[i].y}%` }}
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
    </div>
  );
}
function NodeInspector({
  node,
  maxStrength,
  onEdit,
  onJitter,
}: {
  node?: SpecimenNode;
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
        <span className="inspector-label">Original pattern</span>
        <div className="code-box">{node.entries[0]?.pattern}</div>
        <span className="inspector-label">Action</span>
        <div className="code-box">
          {node.entries[0]?.alternatives[0]?.action}
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
function Trace({ steps, busy }: { steps: TraceStep[]; busy: boolean }) {
  return (
    <div className="trace-list">
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
    </div>
  );
}
/* eslint-disable jsx-a11y/prefer-tag-over-role -- Inline SVG requires an image role; img cannot contain vector primitives. */
function VisualOutput({ visual }: { visual: NonNullable<Cycle['visual']> }) {
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
function SearchField({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
}) {
  return (
    <div className="search-field">
      <Search size={17} />
      <input
        name="search"
        aria-label={placeholder.replace('…', '')}
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
      {value && (
        <button
          className="icon-button"
          aria-label="Clear search"
          onClick={() => onChange('')}
        >
          <X size={15} />
        </button>
      )}
    </div>
  );
}
function NodesView({
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
  const [nodeStart,setNodeStart] = useState(0);
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
      <div className="workspace-tools">
        <SearchField
          value={query}
          onChange={(value) => { setNodeStart(0); setQuery(value); }}
          placeholder="Find a pattern or node…"
        />
        <select
          name="node-type-filter"
          aria-label="Filter node type"
          value={filter}
          onChange={(e) => {setNodeStart(0); setFilter(e.target.value);}}
        >
          <option value="all">All node types</option>
          <option value="pattern">Pattern</option>
          <option value="A">Type A · residual</option>
          <option value="B">Type B · idle</option>
        </select>
        <span className="toolbar-spacer" />
        <button
          className="button outline"
          onClick={onAttach}
          disabled={state.nodes.length < 2}
        >
          <GitBranch size={16} />
          Attach nodes
        </button>
        <button className="button primary" onClick={onAdd}>
          <Plus size={17} />
          Add pattern
        </button>
      </div>
      <div className="panel table-panel">
        <div className="panel-heading">
          <h2>Learned nodes</h2>
          <span className="muted small">{nodes.length} results</span>
        </div>
        <div key={`${query}:${filter}`} className="table-scroll virtual-table" onScroll={e => setNodeStart(Math.max(0,Math.floor(e.currentTarget.scrollTop / 76)-2))}>
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
              {nodeStart > 0 && <tr aria-hidden="true"><td aria-label="Unrendered rows" colSpan={6} style={{height:Math.min(nodeStart,nodes.length)*76,padding:0,border:0}} /></tr>}
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
              {nodes.length > nodeStart+24 && <tr aria-hidden="true"><td aria-label="Unrendered rows" colSpan={6} style={{height:(nodes.length-nodeStart-24)*76,padding:0,border:0}} /></tr>}
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
      <div className="workspace-footnote">
        <Shield size={17} />
        <p>
          Pattern IDs may repeat. Every node keeps its own original patterns,
          votes, and strength.
        </p>
      </div>
      <div className="panel attachment-list">
        <div className="panel-heading">
          <h2>Semantic attachments</h2>
          <span className="muted small">{state.attachments.length} links</span>
        </div>
        {state.attachments.length ? (
          state.attachments.map((a) => (
            <div className="attachment-row" key={a.id}>
              <GitBranch size={17} />
              <strong>{state.nodes.find((n) => n.ref === a.from)?.name}</strong>
              <span>{a.bidirectional ? '↔' : '→'}</span>
              <strong>{state.nodes.find((n) => n.ref === a.to)?.name}</strong>
              <span className="muted">
                {a.hard
                  ? 'Hard attachment'
                  : `${Math.round(a.affinity * 100)}% affinity`}
              </span>
            </div>
          ))
        ) : (
          <p className="panel-description">
            Connect related nodes to hand off the part of an input that did not
            match.
          </p>
        )}
      </div>
    </>
  );
}
function MemoryView({
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
        <div className="segmented">
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
        </div>
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
          <VirtualList key={`${query}:${tab}`} items={history} rowHeight={190} renderItem={(h) => (
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
          )} />
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
function ActivityView({
  state,
  onMaintenance,
  onLearn,
  onReview,
  busy,
}: {
  state: Specimen;
  onMaintenance: (m: 'phagy' | 'mutation') => void;
  onLearn: (pattern: string) => void;
  onReview: () => void;
  busy: boolean;
}) {
  return (
    <>
      <div className="workspace-tools">
        <button className="button outline" disabled={busy} onClick={onReview}>
          <Search size={16} />
          Review context
        </button>
        <span className="muted">
          A record of cycles, learning, and maintenance
        </span>
        <span className="toolbar-spacer" />
        <button
          className="button outline"
          disabled={busy}
          onClick={() => onMaintenance('mutation')}
        >
          <GitBranch size={16} />
          Run mutation scan
        </button>
        <button
          className="button outline"
          disabled={busy}
          onClick={() => onMaintenance('phagy')}
        >
          <Shield size={16} />
          Run PHAGY
        </button>
      </div>
      {state.proposals.some((p) => p.status === 'pending') && (
        <section className="panel proposals">
          <div className="panel-heading">
            <h2>Learning proposals</h2>
          </div>
          {state.proposals
            .filter((p) => p.status === 'pending')
            .map((p) => (
              <div className="proposal" key={p.id}>
                <div>
                  <strong>{p.pattern}</strong>
                  <p>
                    {p.evidence} unmatched prompts suggest a missing pattern.
                    Supply its response to approve learning.
                  </p>
                </div>
                <button
                  className="button primary"
                  onClick={() => onLearn(p.pattern)}
                >
                  Review & learn
                </button>
              </div>
            ))}
        </section>
      )}
      <section className="panel activity-panel">
        {state.events.slice(0, 100).map((event) => (
          <div className="activity-row" key={event.id}>
            <span
              className={`event-icon ${event.type === 'feedback' ? 'amber' : ''}`}
            >
              <Activity size={17} />
            </span>
            <div>
              <strong>{event.title}</strong>
              <p>{event.detail}</p>
            </div>
            <time dateTime={event.createdAt}>{clock(event.createdAt)}</time>
          </div>
        ))}
      </section>
      <section className="runtime-note">
        <h2>About this runtime</h2>
        <p>
          This browser studio runs local pattern matching, procedural sigils,
          votes, feedback, memory, attachments, and bounded maintenance. Its
          synthesis is fixed and does not train a language model. The complete
          design remains available in Specification. Read the{' '}
          <a href="/RUNTIME-PROFILE.md" download>
            browser runtime profile
          </a>{' '}
          for exact behavior and limits.
        </p>
      </section>
    </>
  );
}
function Specification({
  chapter,
  query,
  setQuery,
  onChapter,
}: {
  chapter: number;
  query: string;
  setQuery: (v: string) => void;
  onChapter: (n: number) => void;
}) {
  const current = sections.find((s) => s.number === chapter) ?? sections[0];
  const filtered = sections.filter((s) =>
    `${s.title} ${s.text}`.toLowerCase().includes(query.toLowerCase()),
  );
  return (
    <div className="reference-layout">
      <aside className="chapter-nav">
        <SearchField
          value={query}
          onChange={setQuery}
          placeholder="Search specification…"
        />
        <span className="chapter-count">{filtered.length} chapters</span>
        <nav aria-label="Specification chapters">
          {filtered.map((s) => (
            <a
              key={s.number}
              href={`?view=specification&chapter=${s.number}`}
              className={s.number === chapter ? 'active' : ''}
              aria-current={s.number === chapter ? 'page' : undefined}
              onClick={(e) => {
                if (!e.metaKey && !e.ctrlKey) {
                  e.preventDefault();
                  onChapter(s.number);
                }
              }}
            >
              <span>{String(s.number).padStart(2, '0')}</span>
              {s.title}
            </a>
          ))}
        </nav>
        {!filtered.length && (
          <p className="muted">No chapters match that search.</p>
        )}
      </aside>
      <article className="reference-article">
        <div className="reference-meta">
          <span>{String(current.number).padStart(2, '0')} / 26</span>
          <span>{Math.ceil(current.words / 230)} min read</span>
        </div>
        <h2>{current.title}</h2>
        <div
          className="reference-content"
          dangerouslySetInnerHTML={{ __html: current.html }}
        />
        <div className="chapter-pagination">
          {chapter > 1 ? (
            <a
              href={`?view=specification&chapter=${chapter - 1}`}
              onClick={(e) => {
                e.preventDefault();
                onChapter(chapter - 1);
              }}
            >
              ← {sections[chapter - 2].title}
            </a>
          ) : (
            <span />
          )}
          {chapter < 26 && (
            <a
              href={`?view=specification&chapter=${chapter + 1}`}
              onClick={(e) => {
                e.preventDefault();
                onChapter(chapter + 1);
              }}
            >
              {sections[chapter].title} →
            </a>
          )}
        </div>
      </article>
    </div>
  );
}
function SettingsView({
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
        {isDesktop() && <div className="setting-row"><div><label htmlFor="native-gpu">Accelerate transient computation</label><p>Try the graphics processor; report CPU fallback when unavailable.</p></div><input id="native-gpu" type="checkbox" checked={!!draft.gpu} onChange={e=>setDraft({...draft,gpu:e.target.checked})}/></div>}
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
function Empty({ title, text }: { title: string; text: string }) {
  return (
    <div className="empty-state">
      <Boxes size={30} />
      <h3>{title}</h3>
      <p>{text}</p>
    </div>
  );
}
type NodeInput = {
  name: string;
  pattern: string;
  action: string;
  tone?: SpecimenNode['tone'];
  type?: SpecimenNode['type'];
  contextId?: string;
  entries?: SpecimenNode['entries'];
};
function NodeForm({
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
function ResourceForm({
  resource,
  state,
  onSubmit,
}: {
  resource?: Resource;
  state: Specimen;
  onSubmit: (v: Omit<Resource, 'ref' | 'patternId'>) => void;
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
            text: formText(f, 'term'),
            value: formText(f, 'value'),
            subsystem: formText(f, 'subsystem'),
            resourceId: formText(f, 'link'),
            valence: Number(f.get('valence')),
            intensity: Number(f.get('intensity')),
          });
        } catch (err) {
          setError(
            err instanceof Error ? err.message : 'Check the memory fields.',
          );
        }
      }}
    >
      <label htmlFor="memory-term">Word, phrase, or concept</label>
      <input
        id="memory-term"
        name="term"
        autoComplete="off"
        required
        defaultValue={resource?.text ?? ''}
        placeholder="e.g. curiosity…"
      />
      <label htmlFor="memory-value">Supporting information</label>
      <textarea
        id="memory-value"
        name="value"
        autoComplete="off"
        required
        defaultValue={resource?.value ?? ''}
        placeholder="Add a definition, synonyms, or a contextual rule…"
      />
      <div className="form-columns">
        <div>
          <label htmlFor="memory-subsystem">Subsystem</label>
          <select
            id="memory-subsystem"
            name="subsystem"
            defaultValue={resource?.subsystem ?? 'dictionary'}
          >
            {SUBSYSTEMS.map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </div>
        <div>
          <label htmlFor="memory-link">Link to node ID</label>
          <select
            id="memory-link"
            name="link"
            defaultValue={resource?.resourceId ?? ''}
          >
            <option value="">Unlinked</option>
            {[...new Set(state.nodes.map((n) => n.patternId))].map((id) => (
              <option key={id}>{id}</option>
            ))}
          </select>
        </div>
      </div>
      <div className="form-columns">
        <div>
          <label htmlFor="memory-valence">Valence (−1 to 1)</label>
          <input
            id="memory-valence"
            name="valence"
            type="number"
            min="-1"
            max="1"
            step="0.1"
            defaultValue={resource?.valence ?? 0}
          />
        </div>
        <div>
          <label htmlFor="memory-intensity">Intensity (0 to 1)</label>
          <input
            id="memory-intensity"
            name="intensity"
            type="number"
            min="0"
            max="1"
            step="0.1"
            defaultValue={resource?.intensity ?? 0.5}
          />
        </div>
      </div>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <button className="button primary full-width" type="submit">
        Save memory
        <Check size={16} />
      </button>
    </form>
  );
}
function AttachmentForm({
  state,
  onSave,
}: {
  state: Specimen;
  onSave: (a: Omit<Specimen['attachments'][number], 'id'>) => void;
}) {
  const [error, setError] = useState('');
  return (
    <form
      className="editor-form"
      onSubmit={(e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const from = formText(f, 'from'),
          to = formText(f, 'to');
        if (from === to) {
          setError('Choose two different nodes.');
          return;
        }
        if (state.attachments.some((a) => a.from === from && a.to === to)) {
          setError('Those nodes are already attached.');
          return;
        }
        onSave({
          from,
          to,
          bidirectional: f.get('bidirectional') === 'on',
          hard: f.get('hard') === 'on',
          affinity: Number(f.get('affinity')),
        });
      }}
    >
      {['from', 'to'].map((field, i) => (
        <div key={field}>
          <label htmlFor={`attach-${field}`}>
            {field === 'from' ? 'Issuing node' : 'Attached node'}
          </label>
          <select
            id={`attach-${field}`}
            name={field}
            defaultValue={state.nodes[i]?.ref}
          >
            {state.nodes.map((n) => (
              <option key={n.ref} value={n.ref}>
                {n.name} · {n.patternId}
              </option>
            ))}
          </select>
        </div>
      ))}
      <label htmlFor="attach-affinity">Attachment affinity (0 to 1)</label>
      <input
        id="attach-affinity"
        name="affinity"
        type="number"
        min="0"
        max="1"
        step="0.05"
        defaultValue="0.7"
      />
      <label className="checkbox-label">
        <input type="checkbox" name="hard" />
        Hard attachment (always schedule)
      </label>
      <label className="checkbox-label">
        <input type="checkbox" name="bidirectional" />
        Bidirectional handoff
      </label>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <button className="button primary full-width" type="submit">
        Create attachment
        <GitBranch size={16} />
      </button>
    </form>
  );
}
