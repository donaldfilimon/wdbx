/**
 * The specimen engine for both editions: the Rust kernel compiled to WASM.
 * Edits are synchronous calls on the main thread after `initKernel`; cycles
 * and reviews run in a Web Worker (cancellable, streaming trace steps). On
 * desktop, cycles/reviews/maintenance go to the native process instead
 * (persisted network, GPU, threaded scan); edits still run here, so both
 * editions share one mutation path. Semantics are the kernel's (native).
 */
import {
  instantiateKernel,
  KernelError,
  unwrap,
  type KernelInstance,
} from './kernel-core';
import { validateCollections } from './contracts';
import { isDesktop, nativeReview, runNative } from './native';
import type {
  Cycle,
  Entry,
  Resource,
  Settings,
  Specimen,
  SpecimenNode,
  TraceStep,
} from './types';

let kernel: KernelInstance | undefined;
let wasmBytes: ArrayBuffer | undefined;
let workerUrl: string | undefined;
let loadError: string | undefined;
let restarting: Promise<void> | undefined;
let worker: Worker | undefined;
let workerAllowed = true;
let nextJob = 1;

/**
 * Loads the kernel once. `source` is the .wasm bytes or its URL; `worker` is
 * the built URL of kernel-worker (without it, long commands run inline).
 */
export async function initKernel(
  source: BufferSource | string | URL,
  options: { worker?: string } = {},
) {
  if (kernel) {
    if (options.worker) workerUrl = options.worker;
    return;
  }
  try {
    await load(source);
    workerUrl = options.worker;
    loadError = undefined;
  } catch (err) {
    loadError = err instanceof Error ? err.message : String(err);
    throw new Error(`The specimen kernel failed to load: ${loadError}`);
  }
}

async function fetchBytes(url: string | URL): Promise<ArrayBuffer> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${url}`);
  return res.arrayBuffer();
}

async function load(source: BufferSource | string | URL) {
  const bytes =
    typeof source === 'string' || source instanceof URL
      ? await fetchBytes(source)
      : source instanceof ArrayBuffer
        ? source
        : new Uint8Array(
            (source as ArrayBufferView).buffer,
            (source as ArrayBufferView).byteOffset,
            (source as ArrayBufferView).byteLength,
          ).slice().buffer;
  wasmBytes = bytes;
  kernel = await instantiateKernel(bytes);
}

export const kernelReady = () => kernel !== undefined;

/** Tests run long commands on the calling thread instead of a Worker. */
export function useKernelWorker(allowed: boolean) {
  workerAllowed = allowed;
}

function k(): KernelInstance {
  if (loadError)
    throw new Error(`The specimen kernel failed to load: ${loadError}`);
  if (!kernel) throw new Error('The specimen kernel is still loading.');
  if (kernel.poisoned) {
    // Replace a trapped instance in the background; calls resume once ready.
    restarting ??= instantiateKernel(wasmBytes!).then((fresh) => {
      kernel = fresh;
      restarting = undefined;
    });
    throw new KernelError('Trap', 'The specimen kernel is restarting.');
  }
  return kernel;
}

const call = <T>(request: object) => unwrap<T>(k().call(request));

/** An empty, valid placeholder until the kernel seeds or loads a specimen. */
export const PLACEHOLDER: Specimen = {
  schema: 'wdbx.studio.v1',
  name: 'Specimen 001',
  nodes: [],
  resources: [],
  attachments: [],
  history: [],
  events: [],
  mutations: [],
  proposals: [],
  atp: { valence: 0.2, intensity: 0.25, lastUpdate: 0 },
  settings: {
    voteThreshold: 62,
    jitter: 1.5,
    maxStrength: 10,
    initialStrength: 5,
    entryLimit: 20,
    scanLimit: 1000,
    fanoutLimit: 12,
    historyLimit: 100000,
    pinLimit: 100,
    maxNodes: 1000,
    maxRafts: 8,
    maxThreads: 1,
    chunkSize: 256,
    raftThreshold: 128,
    brainstorm: false,
    maintenance: false,
    adaptiveThreading: false,
    idleMin: 60,
    idleMax: 120,
    transientWidth: 32,
    seed: 104729,
  },
  updatedAt: '1970-01-01T00:00:00.000Z',
};

export const seedSpecimen = () => call<Specimen>({ op: 'seed' });

export const addNode = (
  state: Specimen,
  input: {
    name: string;
    pattern: string;
    action: string;
    tone?: SpecimenNode['tone'];
    type?: SpecimenNode['type'];
    contextId?: string;
    entries?: Entry[];
  },
  ref?: string,
) => call<Specimen>({ op: 'addNode', state, input, ref });

export const addEntry = (
  state: Specimen,
  nodeRef: string,
  pattern: string,
  action: string,
) => call<Specimen>({ op: 'addEntry', state, nodeRef, pattern, action });

export const removeNode = (state: Specimen, ref: string) =>
  call<Specimen>({ op: 'removeNode', state, ref });

export const saveResource = (
  state: Specimen,
  input: Omit<Resource, 'ref' | 'patternId'>,
  ref?: string,
) => call<Specimen>({ op: 'saveResource', state, input, ref });

export const feedback = (
  state: Specimen,
  cycleId: string,
  right: boolean,
  color?: string,
) =>
  call<{ state: Specimen; message: string }>({
    op: 'feedback',
    state,
    cycleId,
    right,
    color,
  });

export const togglePin = (state: Specimen, id: string) =>
  call<Specimen>({ op: 'togglePin', state, id });

/** Appends an event to `state` in place (newest first, 500 kept). */
export function log(
  state: Specimen,
  type: string,
  title: string,
  detail: string,
): void {
  const next = call<Specimen>({ op: 'log', state, type, title, detail });
  state.events = next.events;
  state.updatedAt = next.updatedAt;
}

export function validateSettings(settings: Settings): void {
  call({ op: 'validateSettings', settings });
}

export const migrateIds = (state: Specimen) =>
  call<Specimen>({ op: 'migrateIds', state });

/** One checkpoint chunk of a raft scan and its disjoint raft ranges. */
export interface RaftChunk {
  start: number;
  end: number;
  rafts: [number, number][];
}

/** A raft scan's partition: totals plus the first `limit` chunks. */
export interface RaftPlan {
  chunkSize: number;
  checkpoints: number;
  maxRafts: number;
  chunks: RaftChunk[];
}

/** The kernel's partition of a raft scan over `len` candidates. */
export const raftPlan = (
  len: number,
  chunk: number,
  workers: number,
  limit: number,
) => call<RaftPlan>({ op: 'raftPlan', len, chunk, workers, limit });

/** Structural checks for untrusted files, then the kernel's native rules. */
export function validateSpecimen(value: unknown): Specimen {
  if (!value || typeof value !== 'object')
    throw new Error('Choose a WDBX Studio JSON file.');
  const s = value as Specimen;
  if (s.schema !== 'wdbx.studio.v1')
    throw new Error('This file is not a supported WDBX Studio specimen.');
  if (
    typeof s.name !== 'string' ||
    s.name.length > 100 ||
    !s.settings ||
    !s.atp
  )
    throw new Error('Specimen metadata is missing.');
  for (const key of [
    'nodes',
    'resources',
    'attachments',
    'history',
    'events',
    'mutations',
    'proposals',
  ] as const)
    if (!Array.isArray(s[key]))
      throw new Error(`The ${key} collection is missing.`);
  validateCollections(s);
  // Repair files from the retired TypeScript engine before native checks.
  const migrated = call<Specimen>({ op: 'migrateIds', state: s });
  call({ op: 'validate', state: migrated });
  return migrated;
}

/** Runs PHAGY or vote mutation (a coin flip when `mode` is omitted). */
export const maintenance = (
  state: Specimen,
  mode: 'phagy' | 'mutation' = Math.random() < 0.5 ? 'phagy' : 'mutation',
) => call<Specimen>({ op: 'maintain', state, mode });

function abortError() {
  return new DOMException('The operation was aborted.', 'AbortError');
}

/**
 * Cooperative checkpoint: yields to the event loop, then honors a pending
 * abort, before long work is dispatched.
 */
async function checkpoint(signal?: AbortSignal) {
  await new Promise((resolve) => setTimeout(resolve, 0));
  if (signal?.aborted) throw abortError();
}

function spawnWorker(): Worker {
  const w = new Worker(workerUrl!, { type: 'module' });
  w.postMessage({ type: 'init', bytes: wasmBytes!.slice(0) });
  return w;
}

async function runLong<T>(
  request: object,
  onStep: ((step: TraceStep) => void) | undefined,
  signal?: AbortSignal,
): Promise<T> {
  await checkpoint(signal);
  if (kernel?.poisoned) {
    // Long commands can wait for the fresh instance a trap requires.
    try {
      k();
    } catch {
      // k() started the restart; wait for it below.
    }
    await restarting;
  }
  k();
  if (!workerAllowed || !workerUrl || typeof Worker === 'undefined') {
    return unwrap<T>(k().call(request, (step) => onStep?.(step as TraceStep)));
  }
  worker ??= spawnWorker();
  const w = worker;
  const id = nextJob++;
  return new Promise<T>((resolve, reject) => {
    const cleanup = () => {
      w.removeEventListener('message', onMessage);
      w.removeEventListener('error', onError);
      w.removeEventListener('messageerror', onError);
      signal?.removeEventListener('abort', onAbort);
    };
    const retire = () => {
      cleanup();
      w.terminate();
      if (worker === w) worker = undefined;
    };
    const onAbort = () => {
      retire();
      reject(abortError());
    };
    const onError = (event: Event) => {
      retire();
      const detail = (event as ErrorEvent).message || 'worker error';
      reject(
        new KernelError('Trap', `The specimen worker failed (${detail}).`),
      );
    };
    const onMessage = (event: MessageEvent) => {
      const data = event.data as {
        id: number;
        type: 'trace' | 'result';
        step?: TraceStep;
        reply?: Parameters<typeof unwrap>[0];
      };
      if (data.id !== id) return;
      if (data.type === 'trace') return onStep?.(data.step!);
      cleanup();
      try {
        resolve(unwrap<T>(data.reply!));
      } catch (e) {
        reject(e);
      }
    };
    w.addEventListener('message', onMessage);
    w.addEventListener('error', onError);
    w.addEventListener('messageerror', onError);
    signal?.addEventListener('abort', onAbort, { once: true });
    w.postMessage({ id, request });
  });
}

export async function runCycle(
  state: Specimen,
  input: string,
  onTrace?: (trace: TraceStep[]) => void,
  signal?: AbortSignal,
): Promise<{ state: Specimen; cycle: Cycle }> {
  if (isDesktop()) return runNative(state, input, onTrace, signal);
  if (!input.trim()) throw new Error('Enter a prompt first.');
  if (input.length > 8000)
    throw new Error('Keep prompts under 8,000 characters.');
  const trace: TraceStep[] = [];
  return runLong(
    { op: 'cycle', state, input },
    (step) => {
      trace.push(step);
      onTrace?.([...trace]);
    },
    signal,
  );
}

/** Type A and Type B review of recent history (native semantics). */
export async function observeContext(
  state: Specimen,
  signal?: AbortSignal,
): Promise<Specimen> {
  if (isDesktop()) return nativeReview(state, 'review', undefined, signal);
  return runLong({ op: 'review', state }, undefined, signal);
}

export { KernelError };
