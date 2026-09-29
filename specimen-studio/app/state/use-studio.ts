'use client';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  feedback,
  maintenance,
  observeContext,
  runCycle,
  seedSpecimen,
  validateSpecimen,
  initKernel,
  migrateIds,
  PLACEHOLDER,
} from '@/lib/specimen/kernel';
import { kernelUrls } from '@/lib/specimen/kernel-url';
import {
  downloadText,
  loadWorkspace,
  persistWorkspace,
} from '@/lib/specimen/storage';
import { type Specimen, type TraceStep } from '@/lib/specimen/types';

import {
  isDesktop,
  nativeReview,
  exportNative,
  importNative,
} from '@/lib/specimen/native';
import { useStudioTools } from '@/lib/webmcp';
import type { View } from './navigation';

/** Specimen state and every handler the panels use; layout lives in Studio. */
export function useStudio(nav: (v: View, section?: number) => void) {
  const [state, setState] = useState<Specimen>(PLACEHOLDER);
  const current = useRef(state);
  const [storageEnabled, setStorageEnabled] = useState(false);
  const [hydrated, setHydrated] = useState(false),
    [saveStatus, setSaveStatus] = useState('Opening workspace…'),
    [selected, setSelected] = useState(''),
    [prompt, setPrompt] = useState(''),
    [busy, setBusy] = useState(false),
    [trace, setTrace] = useState<TraceStep[]>([]),
    [runOutcome, setRunOutcome] = useState<
      'idle' | 'running' | 'complete' | 'cancelled' | 'failed'
    >('idle'),
    [runInput, setRunInput] = useState(''),
    [networkMode, setNetworkMode] = useState('network'),
    [topologyZoom, setTopologyZoom] = useState(1),
    [notice, setNotice] = useState(''),
    [error, setError] = useState(''),
    [showProvenance, setShowProvenance] = useState(false),
    [dialog, setDialog] = useState<
      'node' | 'resource' | 'load' | 'reset' | 'attach' | null
    >(null),
    [editing, setEditing] = useState<string | undefined>(),
    [pendingLoad, setPendingLoad] = useState<Specimen | null>(null);
  const fileRef = useRef<HTMLInputElement>(null),
    promptRef = useRef<HTMLInputElement>(null),
    dialogReturnRef = useRef<HTMLElement | null>(null),
    abortRef = useRef<AbortController | null>(null),
    lastInteraction = useRef(0),
    noticeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined),
    undo = useRef<Specimen | null>(null);
  const selectedNode =
    state.nodes.find((n) => n.ref === selected) ?? state.nodes[0];
  const cycle = state.history.at(-1);
  const activeTrace = runOutcome === 'idle' ? (cycle?.trace ?? []) : trace;
  const ratedContributors = new Set(
    cycle?.segments
      .filter(
        (segment) =>
          cycle.feedback.includes('all') ||
          cycle.feedback.includes(segment.color),
      )
      .flatMap((segment) => segment.contributors) ?? [],
  );
  const feedbackUnavailable = (refs: string[]) =>
    busy || refs.length === 0 || refs.some((ref) => ratedContributors.has(ref));
  const maxConfidence = cycle?.votes.length
    ? Math.max(...cycle.votes.map((vote) => vote.confidence))
    : null;
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
  const openDialog = useCallback(
    (
      next: 'node' | 'resource' | 'load' | 'reset' | 'attach',
      preserveReturn = false,
    ) => {
      if (!preserveReturn && document.activeElement instanceof HTMLElement)
        dialogReturnRef.current = document.activeElement;
      setDialog(next);
    },
    [],
  );
  useEffect(() => {
    if (dialog !== null || !dialogReturnRef.current) return;
    const target = dialogReturnRef.current;
    dialogReturnRef.current = null;
    const frame = requestAnimationFrame(() => {
      if (target.isConnected) target.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [dialog]);
  useEffect(() => {
    lastInteraction.current = Date.now();
    initKernel(kernelUrls.wasm, { worker: kernelUrls.worker })
      .then(loadWorkspace)
      .then((saved) => {
        commit(saved ? migrateIds(saved) : seedSpecimen());
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
    return () => clearTimeout(noticeTimer.current);
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
          const controller = new AbortController();
          abortRef.current = controller;
          void nativeReview(
            current.current,
            'maintenance',
            Math.random() < 0.5 ? 'phagy' : 'mutation',
            controller.signal,
          )
            .then(commit)
            .catch((e) => setError(e.message))
            .finally(() => {
              if (abortRef.current === controller) abortRef.current = null;
            });
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
      const next = isDesktop()
        ? await nativeReview(snapshot, 'review', undefined, controller.signal)
        : await observeContext(snapshot, controller.signal);
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
      if (abortRef.current) {
        abortRef.current.abort();
        abortRef.current = null;
      }
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
        if (isDesktop()) {
          void persistWorkspace(current.current)
            .then(exportNative)
            .then((saved) => saved && announce('Portable specimen saved.'))
            .catch((e) => setError(e.message));
          setPrompt('');
          return;
        }
        downloadText('specimen.json', JSON.stringify(current.current, null, 2));
        announce('Specimen downloaded.');
        setPrompt('');
        return;
      }
      if (input.startsWith('/loadSpecimen')) {
        if (isDesktop()) {
          void importNative()
            .then((snapshot) => {
              if (snapshot?.specimen) commit(snapshot.specimen);
            })
            .catch((e) => setError(e.message));
        } else fileRef.current?.click();
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
        openDialog('node');
        return;
      }
      if (input.startsWith('/addPattern')) {
        setEditing(undefined);
        openDialog('node');
        return;
      }
      const controller = new AbortController();
      abortRef.current = controller;
      setBusy(true);
      setTrace([]);
      setRunOutcome('running');
      setRunInput(input);
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
        setRunOutcome('complete');
        setPrompt('');
        setSelected(result.cycle.votes[0]?.nodeRef ?? '');
      } catch (e) {
        setRunOutcome(
          e instanceof Error && e.name === 'AbortError'
            ? 'cancelled'
            : 'failed',
        );
        if (e instanceof Error && e.name !== 'AbortError') setError(e.message);
        else announce('Cycle cancelled. No partial changes were saved.');
      } finally {
        setBusy(false);
        abortRef.current = null;
      }
    },
    [act, announce, busy, commit, hydrated, openDialog],
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
      setRunOutcome('running');
      setRunInput(input);
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
        setRunOutcome('complete');
        return result.cycle;
      } catch (e) {
        setRunOutcome(
          e instanceof Error && e.name === 'AbortError'
            ? 'cancelled'
            : 'failed',
        );
        throw e;
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
    if (isDesktop()) {
      void persistWorkspace(current.current)
        .then(exportNative)
        .then((saved) => saved && announce('Portable specimen saved.'))
        .catch((e) => setError(e.message));
      return;
    }
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
      openDialog('load', true);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not read the specimen.');
    } finally {
      if (fileRef.current) fileRef.current.value = '';
    }
  };
  const setUndo = (previous: Specimen | null) => {
    undo.current = previous;
  };
  const popUndo = () => {
    const previous = undo.current;
    undo.current = null;
    return previous;
  };
  const setDialogReturn = (target: HTMLElement | null) => {
    dialogReturnRef.current = target;
  };
  return {
    state,
    commit,
    act,
    announce,
    openDialog,
    dialog,
    setDialog,
    editing,
    setEditing,
    pendingLoad,
    setPendingLoad,
    saveStatus,
    storageEnabled,
    setStorageEnabled,
    hydrated,
    selected,
    setSelected,
    selectedNode,
    prompt,
    setPrompt,
    promptRef,
    busy,
    trace,
    activeTrace,
    runOutcome,
    runInput,
    networkMode,
    setNetworkMode,
    topologyZoom,
    setTopologyZoom,
    notice,
    setNotice,
    error,
    setError,
    showProvenance,
    setShowProvenance,
    cycle,
    ratedContributors,
    feedbackUnavailable,
    maxConfidence,
    run,
    reviewContext,
    giveFeedback,
    save,
    load,
    fileRef,
    abortRef,
    current,
    setDialogReturn,
    setUndo,
    popUndo,
    setRunOutcome,
    setTrace,
  };
}

export type StudioModel = ReturnType<typeof useStudio>;
