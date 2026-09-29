'use client';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Check, X } from 'lucide-react';
import { DialogHost } from './dialogs/dialog-host';
import { AppSidebar } from './shell/app-sidebar';
import { buildCommands, type ShellCommand } from './shell/commands';
import { CommandPalette } from './shell/command-palette';
import { ConsoleDock } from './shell/console-dock';
import { PaneLayout } from './shell/pane-layout';
import { matchShortcut } from './shell/shortcuts';
import {
  applyTheme,
  resolveTheme,
  storeTheme,
  type Theme,
} from './shell/theme';
import { TopBar } from './shell/top-bar';
import { ViewOutlet } from './shell/view-outlet';
import { VIEWS, parseRoute, routeSearch, type View } from './state/navigation';
import { useStudio } from './state/use-studio';

function safeStorage(): Storage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

export default function Studio() {
  const [view, setView] = useState<View>('studio'),
    [split, setSplit] = useState<View | undefined>(),
    [mobileNav, setMobileNav] = useState(false),
    [mobileViewport, setMobileViewport] = useState(false),
    [wide, setWide] = useState(false),
    [collapsed, setCollapsed] = useState(false),
    [dockOpen, setDockOpen] = useState(false),
    [paletteOpen, setPaletteOpen] = useState(false),
    [theme, setTheme] = useState<Theme>('dark'),
    [queries, setQueries] = useState({ primary: '', side: '' }),
    [chapter, setChapter] = useState(1);
  const mobileMenuRef = useRef<HTMLButtonElement>(null),
    mobileCloseRef = useRef<HTMLButtonElement>(null),
    sidebarRef = useRef<HTMLElement>(null),
    navWasOpen = useRef(false),
    route = useRef<{ view: View; split?: View; chapter?: number }>({
      view: 'studio',
    }),
    lastSide = useRef<View | undefined>(undefined);
  const go = useCallback(
    (next: { view: View; split?: View; chapter?: number }, scroll: boolean) => {
      const split = next.split === next.view ? undefined : next.split;
      route.current = { ...next, split };
      setView(next.view);
      setSplit(split);
      if (split) lastSide.current = split;
      if (next.chapter) setChapter(next.chapter);
      window.history.pushState({}, '', routeSearch(route.current));
      if (scroll) window.scrollTo({ top: 0, behavior: 'instant' });
    },
    [],
  );
  const nav = useCallback(
    (v: View, section?: number) => {
      setQueries({ primary: '', side: '' });
      setMobileNav(false);
      go({ view: v, split: route.current.split, chapter: section }, true);
    },
    [go],
  );
  const openSide = useCallback(
    (v: View) => go({ ...route.current, split: v }, false),
    [go],
  );
  const closeSide = useCallback(
    () => go({ ...route.current, split: undefined }, false),
    [go],
  );
  const openChapterInSide = useCallback(
    (n: number) => go({ ...route.current, chapter: n }, false),
    [go],
  );
  const toggleSplit = useCallback(() => {
    if (route.current.split) return closeSide();
    const fallback =
      lastSide.current && lastSide.current !== route.current.view
        ? lastSide.current
        : VIEWS.find((v) => v !== route.current.view && v !== 'settings');
    if (fallback) openSide(fallback);
  }, [closeSide, openSide]);
  const toggleTheme = useCallback(() => {
    setTheme((current) => {
      const next: Theme = current === 'dark' ? 'light' : 'dark';
      applyTheme(document.documentElement, next);
      storeTheme(safeStorage(), next);
      return next;
    });
  }, []);
  useEffect(() => {
    const restore = () => {
      const r = parseRoute(location.search);
      route.current = r;
      setView(r.view);
      setSplit(r.split);
      if (r.split) lastSide.current = r.split;
      if (r.chapter) setChapter(r.chapter);
    };
    queueMicrotask(() => {
      restore();
      setTheme(resolveTheme(safeStorage()));
    });
    window.addEventListener('popstate', restore);
    return () => window.removeEventListener('popstate', restore);
  }, []);
  useEffect(() => {
    const query = window.matchMedia('(min-width: 1024px)');
    const update = () => setWide(query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
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
    notice,
    error,
    setError,
    commit,
    announce,
    popUndo,
    load,
    fileRef,
    openDialog,
    save,
  } = model;
  // Built when the palette opens (it is modal, so the view cannot change
  // underneath it); building during render would read refs in render.
  const [commands, setCommands] = useState<ShellCommand[]>([]);
  const openPalette = useCallback(() => {
    setCommands(
      buildCommands({
        view,
        split,
        go: (v) => nav(v),
        openSide,
        closeSide,
        save,
        load: () => fileRef.current?.click(),
        reset: () => openDialog('reset'),
        toggleTheme,
        toggleSidebar: () =>
          mobileViewport ? setMobileNav((o) => !o) : setCollapsed((c) => !c),
        toggleDock: () => setDockOpen((o) => !o),
      }),
    );
    setPaletteOpen(true);
  }, [
    view,
    split,
    nav,
    openSide,
    closeSide,
    save,
    fileRef,
    openDialog,
    toggleTheme,
    mobileViewport,
  ]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const action = matchShortcut(event);
      if (!action || action === 'close') return;
      event.preventDefault();
      if (action === 'palette') {
        if (paletteOpen) setPaletteOpen(false);
        else openPalette();
      } else if (action === 'sidebar')
        if (mobileViewport) setMobileNav((o) => !o);
        else setCollapsed((c) => !c);
      else if (action === 'dock') setDockOpen((o) => !o);
      else if (action === 'split') toggleSplit();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [mobileViewport, toggleSplit, openPalette, paletteOpen]);
  const outlet = (v: View, pane: 'primary' | 'side') => (
    <ViewOutlet
      view={v}
      model={model}
      nav={nav}
      query={queries[pane]}
      setQuery={(q) => setQueries((all) => ({ ...all, [pane]: q }))}
      chapter={chapter}
      onChapter={(n) =>
        pane === 'primary' ? nav('specification', n) : openChapterInSide(n)
      }
    />
  );
  return (
    <div
      className={`app-shell ${collapsed && !mobileViewport ? 'sidebar-collapsed' : ''}`}
    >
      <a className="skip-link" href="#main">
        Skip to main content
      </a>
      <AppSidebar
        view={view}
        nav={nav}
        nodeCount={state.nodes.length}
        mobileNav={mobileNav}
        setMobileNav={setMobileNav}
        mobileViewport={mobileViewport}
        collapsed={collapsed && !mobileViewport}
        toggleCollapsed={() => setCollapsed((c) => !c)}
        theme={theme}
        toggleTheme={toggleTheme}
        sidebarRef={sidebarRef}
        mobileCloseRef={mobileCloseRef}
      />
      <main
        id="main"
        className={`main-area ${split && wide ? 'has-split' : ''}`}
        inert={mobileViewport && mobileNav ? true : undefined}
      >
        <TopBar
          view={view}
          model={model}
          mobileNav={mobileNav}
          setMobileNav={setMobileNav}
          mobileMenuRef={mobileMenuRef}
          openPalette={openPalette}
          dockOpen={dockOpen}
          toggleDock={() => setDockOpen((o) => !o)}
        />
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
        <PaneLayout
          primary={outlet(view, 'primary')}
          side={split ? outlet(split, 'side') : null}
          sideView={split}
          wide={wide}
          onCloseSide={closeSide}
        />
        {dockOpen && <ConsoleDock events={state.events} />}
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
      <CommandPalette
        open={paletteOpen}
        onOpenChange={setPaletteOpen}
        commands={commands}
      />
      <DialogHost m={model} />
    </div>
  );
}
