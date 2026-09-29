/// <reference lib="webworker" />
/**
 * Runs long kernel commands (cycle, review) off the main thread. Messages:
 * `{type:'init', bytes}` once, then `{id, request}`; replies are
 * `{id, type:'trace', step}` while running and `{id, type:'result', reply}`.
 * Cancellation terminates this worker (see kernel.ts).
 */
import { instantiateKernel, type KernelInstance } from './kernel-core';

let kernel: Promise<KernelInstance> | undefined;

self.onmessage = async (event: MessageEvent) => {
  const data = event.data as
    | { type: 'init'; bytes: ArrayBuffer }
    | { id: number; request: object };
  if ('type' in data) {
    kernel = instantiateKernel(data.bytes);
    return;
  }
  const k = await kernel!;
  const reply = k.call(data.request, (step) =>
    self.postMessage({ id: data.id, type: 'trace', step }),
  );
  self.postMessage({ id: data.id, type: 'result', reply });
};
