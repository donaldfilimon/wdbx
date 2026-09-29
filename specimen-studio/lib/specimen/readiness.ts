/**
 * One-shot load state with a promise and change subscriptions: waiters get a
 * rejection on failure instead of hanging, and React can subscribe to status.
 */
export type LoadStatus = 'loading' | 'ready' | 'failed';

export function createReadiness() {
  let status: LoadStatus = 'loading';
  let message = '';
  const listeners = new Set<() => void>();
  let resolve!: () => void;
  let reject!: (e: Error) => void;
  const ready = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  // A failure nobody awaited is reported through status, not as unhandled.
  ready.catch(() => {});
  const settle = (next: LoadStatus) => {
    if (status !== 'loading') return;
    status = next;
    for (const listener of listeners) listener();
  };
  return {
    ready,
    status: () => status,
    error: () => message,
    subscribe: (onChange: () => void) => {
      listeners.add(onChange);
      return () => {
        listeners.delete(onChange);
      };
    },
    markReady: () => {
      if (status !== 'loading') return;
      resolve();
      settle('ready');
    },
    markFailed: (e: unknown) => {
      if (status !== 'loading') return;
      message = e instanceof Error ? e.message : String(e);
      reject(new Error(message));
      settle('failed');
    },
  };
}
