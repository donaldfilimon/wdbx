import { Channel, invoke } from '@tauri-apps/api/core';
import type { Cycle, Specimen, TraceStep } from './types';
export const isDesktop = () =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export interface NativeSnapshot {
  schema: 'wdbx.native.v2';
  revision: number;
  specimen: Specimen | null;
  visuals: Record<string, unknown>[];
  artifacts: Record<string, unknown>[];
  network: unknown;
  tombstones: unknown[];
  nativeIds: Record<string, string>;
}
export class NativeError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = 'NativeError';
    this.code = code;
  }
}
let revision = 0;
let saved = '';
let latestSnapshot: NativeSnapshot | undefined;
let queue: Promise<unknown> = Promise.resolve();
function serial<T>(fn: () => Promise<T>): Promise<T> {
  const next = queue.then(fn, fn);
  queue = next.catch(() => {});
  return next;
}
export async function callNative<T>(
  request: Record<string, unknown>,
  onProgress?: (event: Record<string, unknown>) => void,
): Promise<T> {
  const progress = new Channel<Record<string, unknown>>();
  progress.onmessage = (event) => onProgress?.(event);
  try {
    return await invoke<T>('native_call', { request, progress });
  } catch (e) {
    const detail = e as { message?: string; code?: string };
    throw new NativeError(
      detail.code ?? 'Runtime',
      detail.message ?? String(e),
    );
  }
}
export function acceptSnapshot(snapshot: NativeSnapshot) {
  if (latestSnapshot && snapshot.revision < revision) return latestSnapshot;
  revision = snapshot.revision;
  saved = JSON.stringify(snapshot.specimen);
  latestSnapshot = snapshot;
  return snapshot;
}
export async function loadNative() {
  const snapshot = acceptSnapshot(
    await callNative<NativeSnapshot>({ op: 'snapshot' }),
  );
  return snapshot.specimen;
}
export function persistNative(specimen: Specimen) {
  return serial(async () => {
    const encoded = JSON.stringify(specimen);
    if (encoded === saved) return;
    acceptSnapshot(
      await callNative<NativeSnapshot>({ op: 'edit', revision, specimen }),
    );
  });
}
export async function runNative(
  source: Specimen,
  input: string,
  onTrace?: (trace: TraceStep[]) => void,
  signal?: AbortSignal,
) {
  await persistNative(source);
  return serial(async () => {
    signal?.throwIfAborted();
    const jobId = crypto.randomUUID();
    const trace: TraceStep[] = [];
    const cancel = () => {
      void invoke('cancel_job', { jobId });
    };
    signal?.addEventListener('abort', cancel, { once: true });
    try {
      const result = await callNative<{
        snapshot: NativeSnapshot;
        cycle: Cycle;
      }>({ op: 'run', revision, input, jobId }, (event) => {
        trace.push(event as unknown as TraceStep);
        onTrace?.([...trace]);
      });
      acceptSnapshot(result.snapshot);
      return { state: result.snapshot.specimen!, cycle: result.cycle };
    } finally {
      signal?.removeEventListener('abort', cancel);
    }
  });
}
export async function nativeReview(
  source: Specimen,
  mode: 'review' | 'maintenance',
  maintenanceMode?: string,
  signal?: AbortSignal,
) {
  await persistNative(source);
  return serial(async () => {
    const jobId = crypto.randomUUID();
    const cancel = () => {
      void invoke('cancel_job', { jobId });
    };
    signal?.addEventListener('abort', cancel, { once: true });
    try {
      const result = await callNative<{ snapshot: NativeSnapshot }>({
        op: mode,
        revision,
        jobId,
        mode: maintenanceMode,
      });
      return acceptSnapshot(result.snapshot).specimen!;
    } finally {
      signal?.removeEventListener('abort', cancel);
    }
  });
}
export function nativeOperation<T>(
  request: Record<string, unknown>,
  progress?: (event: Record<string, unknown>) => void,
) {
  const operation = async () => {
    const result = await callNative<T>({ ...request, revision }, progress);
    const r = result as { snapshot?: NativeSnapshot; schema?: string };
    if (r.snapshot) acceptSnapshot(r.snapshot);
    else if (r.schema === 'wdbx.native.v2')
      acceptSnapshot(result as NativeSnapshot);
    return result;
  };
  return [
    'analyze',
    'generate',
    'installModel',
    'loadModel',
    'removeModel',
    'unloadModel',
  ].includes(String(request.op))
    ? operation()
    : serial(operation);
}
export function cancelNative(jobId: string) {
  return invoke('cancel_job', { jobId });
}
export async function exportNative() {
  const { save } = await import('@tauri-apps/plugin-dialog');
  const path = await save({
    defaultPath: 'specimen.wdbxspecimen',
    filters: [{ name: 'WDBX specimen', extensions: ['wdbxspecimen'] }],
  });
  if (path) await nativeOperation({ op: 'export', path });
  return !!path;
}
export async function importNative() {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const path = await open({
    multiple: false,
    filters: [{ name: 'WDBX specimen', extensions: ['wdbxspecimen', 'json'] }],
  });
  if (typeof path === 'string')
    return await nativeOperation<NativeSnapshot>({ op: 'import', path });
  return null;
}
export async function pickNativeFile() {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({ multiple: false });
  return typeof selected === 'string' ? selected : undefined;
}
/** The desktop snapshot's network (the one cycles use) and its revision. */
export async function loadNativeNetwork() {
  const snapshot = acceptSnapshot(
    await callNative<NativeSnapshot>({ op: 'snapshot' }),
  );
  return { network: snapshot.network, revision: snapshot.revision };
}
/**
 * Commits a network edit at `base`, the revision the edited network was
 * loaded from, so a change committed since (an import, another edit) makes
 * this a `StaleRevision` error instead of being overwritten.
 */
export function persistNativeNetwork(network: unknown, base: number) {
  return serial(async () => {
    const snapshot = acceptSnapshot(
      await callNative<NativeSnapshot>({
        op: 'network',
        revision: base,
        network,
      }),
    );
    return { network: snapshot.network, revision: snapshot.revision };
  });
}
