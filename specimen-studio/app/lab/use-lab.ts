import { useEffect, useRef, useState } from 'react';
import {
  activeJobs,
  artifactsOf,
  capabilities,
  loadSnapshot,
  modelStatus,
  type Artifact,
  type Capabilities,
  type ModelStatus,
  type Progress,
} from '@/lib/specimen/lab-api';
import { acceptSnapshot, type NativeSnapshot } from '@/lib/specimen/native';
import type { Specimen } from '@/lib/specimen/types';

/** Runs `fn` every `ms` while the tab is visible, and once on becoming visible. */
function useVisibleInterval(fn: () => void, ms: number) {
  const latest = useRef(fn);
  useEffect(() => {
    latest.current = fn;
  });
  useEffect(() => {
    const tick = () => {
      if (document.visibilityState === 'visible') latest.current();
    };
    const timer = setInterval(tick, ms);
    document.addEventListener('visibilitychange', tick);
    return () => {
      clearInterval(timer);
      document.removeEventListener('visibilitychange', tick);
    };
  }, [ms]);
}

/**
 * Lab state for the desktop edition: one foreground job at a time, with
 * progress, cancellation, and the models, jobs and artifacts it affects.
 */
export function useLab(onSnapshot: (s: Specimen) => void) {
  const [models, setModels] = useState<ModelStatus[] | null>(null);
  const [caps, setCaps] = useState<Capabilities | null>(null);
  const [jobs, setJobs] = useState<string[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [busy, setBusy] = useState('');
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [pollError, setPollError] = useState('');
  const job = useRef('');

  const refresh = () =>
    Promise.all([activeJobs(), modelStatus()]).then(
      ([j, m]) => {
        setJobs(j);
        if (m) setModels(m);
        setPollError('');
      },
      (e: unknown) =>
        setPollError(
          `Could not refresh jobs and models: ${e instanceof Error ? e.message : String(e)}`,
        ),
    );

  useEffect(() => {
    let live = true;
    void refresh();
    capabilities().then(
      (c) => live && setCaps(c),
      () => {},
    );
    loadSnapshot().then(
      (s) => live && setArtifacts(artifactsOf(s)),
      () => {},
    );
    return () => {
      live = false;
    };
    // refresh only touches state setters.
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useVisibleInterval(() => void refresh(), 5000);

  const progress: Progress = (event) => {
    const phase = typeof event.phase === 'string' ? event.phase : 'Working';
    setStatus(
      typeof event.received === 'number' && typeof event.total === 'number'
        ? `${phase}: ${Math.round((100 * event.received) / event.total)}%`
        : `${phase}…`,
    );
  };

  const applySnapshot = (s: NativeSnapshot) => {
    const latest = acceptSnapshot(s);
    if (latest.specimen) onSnapshot(latest.specimen);
    setArtifacts(artifactsOf(latest));
  };

  /** Runs one labelled job; a second job is refused while one runs. */
  const work = async (label: string, fn: (jobId: string) => Promise<void>) => {
    if (busy) return;
    const id = crypto.randomUUID();
    job.current = id;
    setBusy(label);
    setError('');
    setStatus(label);
    try {
      await fn(id);
      setStatus(`${label}: completed`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStatus(`${label}: stopped`);
    } finally {
      setBusy('');
      job.current = '';
      void refresh();
    }
  };

  return {
    models,
    caps,
    jobs,
    artifacts,
    busy,
    status,
    error,
    pollError,
    currentJob: () => job.current,
    progress,
    applySnapshot,
    work,
  };
}

export type Lab = ReturnType<typeof useLab>;
