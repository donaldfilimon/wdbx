// The worker path: a failure inside the worker must reject, never hang.
import { beforeAll, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import {
  initKernel,
  observeContext,
  runCycle,
  seedSpecimen,
  useKernelWorker,
} from '../lib/specimen/kernel';

beforeAll(async () => {
  useKernelWorker(true);
  await initKernel(
    readFileSync(
      new URL('../lib/specimen/wasm/specimen_wasm.wasm', import.meta.url),
    ),
    {
      worker: new URL('../lib/specimen/kernel-worker.ts', import.meta.url).href,
    },
  );
});

test('a cycle runs in the worker and streams trace steps', async () => {
  const seen = [];
  const { cycle } = await runCycle(seedSpecimen(), 'What is 2 + 2?', (t) =>
    seen.push(t),
  );
  expect(cycle.segments[0].text).toBe('4');
  expect(seen.length).toBeGreaterThan(0);
});

test('a trap inside the worker rejects and the next job still runs', async () => {
  const broken = { ...seedSpecimen(), events: null, history: null };
  await expect(observeContext(broken)).rejects.toMatchObject({
    code: 'Trap',
  });
  const { cycle } = await runCycle(seedSpecimen(), 'What is 2 + 2?');
  expect(cycle.segments[0].text).toBe('4');
}, 10000);
