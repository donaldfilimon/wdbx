/**
 * Loads the Rust specimen kernel (native/specimen-wasm) and speaks its JSON
 * command ABI: `call(ptr, len)` takes `{op, …}` and returns `{ok}` or
 * `{error: {code, message}}` as `(pointer << 32) | length`.
 */
export type KernelReply =
  | { ok: unknown }
  | { error: { code: string; message: string } };

export interface KernelInstance {
  call(request: object, onProgress?: (step: unknown) => void): KernelReply;
  /** True after a WebAssembly trap: the instance must be replaced. */
  readonly poisoned: boolean;
}

interface Exports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  free(ptr: number, len: number): void;
  call(ptr: number, len: number): bigint;
}

function randomU32(): number {
  const c = globalThis.crypto;
  if (c?.getRandomValues) return c.getRandomValues(new Uint32Array(1))[0];
  return Math.floor(Math.random() * 0x1_0000_0000) >>> 0;
}

export async function instantiateKernel(
  bytes: BufferSource,
): Promise<KernelInstance> {
  const bound: { exports?: Exports } = {};
  let listener: ((step: unknown) => void) | undefined;
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const read = (ptr: number, len: number) =>
    decoder.decode(new Uint8Array(bound.exports!.memory.buffer, ptr, len));
  const compiled = await WebAssembly.compile(bytes);
  const instance = await WebAssembly.instantiate(compiled, {
    env: {
      now_ms: () => Date.now(),
      random_u32: randomU32,
      progress: (ptr: number, len: number) => {
        if (listener) listener(JSON.parse(read(ptr, len)));
      },
    },
  });
  bound.exports = instance.exports as unknown as Exports;
  let poisoned = false;
  return {
    get poisoned() {
      return poisoned;
    },
    call(request, onProgress) {
      if (poisoned)
        throw new KernelError('Trap', 'The specimen kernel is restarting.');
      const e = bound.exports!;
      const input = encoder.encode(JSON.stringify(request));
      const ptr = e.alloc(input.length);
      new Uint8Array(e.memory.buffer, ptr, input.length).set(input);
      listener = onProgress;
      let packed: bigint;
      try {
        packed = e.call(ptr, input.length);
      } catch (err) {
        if (err instanceof WebAssembly.RuntimeError) {
          poisoned = true;
          throw new KernelError(
            'Trap',
            `The specimen kernel stopped unexpectedly (${err.message}). It is restarting; try again.`,
          );
        }
        throw err;
      } finally {
        listener = undefined;
      }
      const outPtr = Number(packed >> BigInt(32));
      const outLen = Number(packed & BigInt(0xffffffff));
      const text = read(outPtr, outLen);
      e.free(outPtr, outLen);
      return JSON.parse(text) as KernelReply;
    },
  };
}

/** An Error carrying the kernel's error code. */
export class KernelError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = code === 'Cancelled' ? 'AbortError' : 'KernelError';
  }
}

export function unwrap<T>(reply: KernelReply): T {
  if ('error' in reply)
    throw new KernelError(reply.error.code, reply.error.message);
  return reply.ok as T;
}
