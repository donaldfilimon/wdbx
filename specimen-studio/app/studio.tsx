'use client';
/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  ArrowDownToLine,
  Boxes,
  Check,
  FileText,
  Menu,
  RotateCcw,
  Save,
  Settings2,
  Upload,
  X,
} from 'lucide-react';
import {
  maintenance,
  removeNode,
  togglePin,
  validateSettings,
  validateSpecimen,
} from '@/lib/specimen/engine';

import { NativeLab } from '@/components/native-lab';
import { isDesktop, nativeReview } from '@/lib/specimen/native';
import { ActivityView } from './panels/activity-panel';
import { DialogHost } from './dialogs/dialog-host';
import { MemoryView } from './panels/memory-panel';
import { NodesView } from './panels/nodes-panel';
import { SettingsView } from './panels/settings-panel';
import { Specification } from './panels/specification-panel';
import { StudioPanel } from './panels/studio-panel';
import type { View } from './state/navigation';
import { labels } from './shell/view-meta';
import { useStudio } from './state/use-studio';
import { viewGroups } from './shell/view-meta';
import { views } from './shell/view-meta';

export default function Studio() {
  const [view, setView] = useState<View>('studio'),
    [mobileNav, setMobileNav] = useState(false),
    [mobileViewport, setMobileViewport] = useState(false),
    [query, setQuery] = useState(''),
    [chapter, setChapter] = useState(1);
  const mobileMenuRef = useRef<HTMLButtonElement>(null),
    mobileCloseRef = useRef<HTMLButtonElement>(null),
    sidebarRef = useRef<HTMLElement>(null),
    navWasOpen = useRef(false);
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
    window.addEventListener('popstate', restore);
    return () => window.removeEventListener('popstate', restore);
  }, []);
  useEffect(() => {
    const query = window.matchMedia('(max-width: 760px)');
    const update = () => {
      setMobileViewport(query.matches);
      if (!query.matches) setMobileNav(false);
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  useEffect(() => {
    if (!mobileNav) return;
    const containDrawerFocus = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setMobileNav(false);
        return;
      }
      if (event.key !== 'Tab') return;
      const drawer = sidebarRef.current;
      if (!drawer) return;
      const controls = [
        ...drawer.querySelectorAll<HTMLElement>(
          'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
        ),
      ].filter((element) => !element.hidden);
      if (!controls.length) return;
      event.preventDefault();
      const currentIndex = controls.findIndex(
        (element) => element === document.activeElement,
      );
      const direction = event.shiftKey ? -1 : 1;
      const nextIndex =
        currentIndex < 0
          ? event.shiftKey
            ? controls.length - 1
            : 0
          : (currentIndex + direction + controls.length) % controls.length;
      controls[nextIndex].focus();
    };
    window.addEventListener('keydown', containDrawerFocus);
    return () => window.removeEventListener('keydown', containDrawerFocus);
  }, [mobileNav]);
  useEffect(() => {
    if (!mobileViewport) {
      navWasOpen.current = false;
      return;
    }
    if (mobileNav) {
      navWasOpen.current = true;
      const frame = requestAnimationFrame(() =>
        mobileCloseRef.current?.focus(),
      );
      return () => cancelAnimationFrame(frame);
    }
    if (navWasOpen.current) {
      navWasOpen.current = false;
      const frame = requestAnimationFrame(() => mobileMenuRef.current?.focus());
      return () => cancelAnimationFrame(frame);
    }
  }, [mobileNav, mobileViewport]);
  const model = useStudio(nav);
  const {
    state,
    commit,
    act,
    announce,
    openDialog,
    setEditing,
    setSelected,
    setPrompt,
    busy,
    notice,
    error,
    setError,
    reviewContext,
    save,
    load,
    fileRef,
    current,
    setDialogReturn,
    setUndo,
    popUndo,
  } = model;
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Skip to main content
      </a>
      <aside
        ref={sidebarRef}
        id="workspace-navigation"
        className={`sidebar ${mobileNav ? 'is-open' : ''}`}
        aria-label="WDBX workspace"
        aria-hidden={mobileViewport && !mobileNav ? true : undefined}
        inert={mobileViewport && !mobileNav ? true : undefined}
      >
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
          ref={mobileCloseRef}
          className="icon-button mobile-close"
          aria-label="Close navigation"
          onClick={() => setMobileNav(false)}
        >
          <X />
        </button>
        <nav aria-label="Main navigation">
          {viewGroups.map((group) => (
            <div className="nav-group" key={group.label}>
              <span className="nav-group-label">{group.label}</span>
              {group.ids.map((id) => {
                const item = views.find((candidate) => candidate.id === id)!;
                const Icon = item.icon;
                return (
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
                    <Icon size={19} />
                    {item.label}
                    {id === 'nodes' && (
                      <span className="nav-count">{state.nodes.length}</span>
                    )}
                  </a>
                );
              })}
            </div>
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
      <main
        id="main"
        className="main-area"
        inert={mobileViewport && mobileNav ? true : undefined}
      >
        <header className="page-header">
          <div className="heading-wrap">
            <button
              ref={mobileMenuRef}
              className="icon-button mobile-menu"
              aria-label="Open navigation"
              aria-controls="workspace-navigation"
              aria-expanded={mobileNav}
              onClick={() => setMobileNav(true)}
            >
              <Menu />
            </button>
            <div>
              <div className="page-title-row">
                <h1>{labels[view][0]}</h1>
                {view === 'studio' && (
                  <span className="specimen-name header-specimen">
                    <FileText size={15} aria-hidden="true" />
                    {state.name}
                    <i className={`status-dot ${busy ? 'busy' : ''}`} />
                    <small>{busy ? 'Running' : 'Ready'}</small>
                  </span>
                )}
              </div>
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
                {view === 'studio' && (
                  <button
                    className="button utility starter-action"
                    aria-label="Reset to starter specimen"
                    onClick={() => openDialog('reset')}
                  >
                    <RotateCcw size={16} />
                    <span>Starter</span>
                  </button>
                )}
                <button
                  className="button utility"
                  aria-label="Load specimen"
                  onClick={(event) => {
                    setDialogReturn(event.currentTarget);
                    fileRef.current?.click();
                  }}
                  disabled={busy}
                >
                  <Upload size={17} />
                  <span>Load</span>
                </button>
                <button
                  className="button outline"
                  aria-label="Save specimen"
                  onClick={save}
                >
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
        {view === 'studio' && <StudioPanel m={model} nav={nav} />}
        {view === 'lab' && <NativeLab specimen={state} onSnapshot={commit} />}
        {view === 'nodes' && (
          <NodesView
            state={state}
            query={query}
            setQuery={setQuery}
            onAdd={() => {
              setEditing(undefined);
              openDialog('node');
            }}
            onEdit={(ref) => {
              setEditing(ref);
              openDialog('node');
            }}
            onRemove={(ref) =>
              act(() => {
                const previous = state;
                commit(removeNode(state, ref));
                announce('Node removed. Use Undo to restore it.');
                setUndo(previous);
              })
            }
            onAttach={() => openDialog('attach')}
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
              openDialog('resource');
            }}
            onEdit={(ref) => {
              setEditing(ref);
              openDialog('resource');
            }}
            onPin={(id) => act(() => commit(togglePin(state, id)))}
            onRemove={(ref) =>
              act(() => {
                setUndo(state);
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
                if (isDesktop()) {
                  void nativeReview(current.current, 'maintenance', mode)
                    .then(commit)
                    .then(() => announce('Maintenance completed.'))
                    .catch((e) => setError(e.message));
                } else {
                  commit(maintenance(state, mode));
                  announce('Maintenance completed.');
                }
              })
            }
            onLearn={(pattern) => {
              setPrompt(pattern);
              setEditing(undefined);
              openDialog('node');
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
              const previous = popUndo();
              if (previous) {
                commit(previous);
                announce('Restored.');
              }
            }}
          >
            Undo
          </button>
        )}
      </div>
      <DialogHost m={model} />
    </div>
  );
}
