// Build-time URLs of the kernel WASM and its worker (Vite assets). Kept out of
// kernel.ts so Bun tests can load the kernel without Vite.
import wasm from './wasm/specimen_wasm.wasm?url';
// oxlint-disable-next-line import/default -- Vite virtual module; oxlint cannot resolve `?worker&url`.
import worker from './kernel-worker.ts?worker&url';

export const kernelUrls = { wasm, worker };
