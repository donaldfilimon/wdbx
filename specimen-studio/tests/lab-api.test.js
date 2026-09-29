import { expect, mock, test } from 'bun:test';

const sent = [];
let revision = 3;
const snapshot = (extra = {}) => ({
  schema: 'wdbx.native.v2',
  revision,
  specimen: { name: 'S' },
  visuals: [],
  artifacts: [],
  network: null,
  tombstones: [],
  nativeIds: {},
  ...extra,
});
mock.module('@tauri-apps/api/core', () => ({
  Channel: class {
    onmessage = () => {};
  },
  invoke: async (_cmd, { request }) => {
    sent.push(request);
    switch (request.op) {
      case 'models':
        return [
          {
            model: { id: 'qwen3-4b', kind: 'text', name: 'Qwen' },
            installed: true,
            loaded: false,
          },
        ];
      case 'jobs':
        return ['job-1'];
      case 'capabilities':
        return {
          runtime: 'desktop',
          storage: 'WDBX v2',
          ocr: true,
          textGeneration: true,
          imageGeneration: true,
          gpu: 'optional wgpu',
          nativeVersion: '0.2.0',
        };
      case 'generate':
        revision += 1;
        return {
          snapshot: snapshot(),
          artifact: {
            id: 'a1',
            model: 'qwen3-4b',
            text: 'hi',
            prompt: 'p',
            seed: 1,
            modelDigest: 'd',
            createdAt: 't',
          },
        };
      case 'snapshot':
        return snapshot();
      case 'edit':
        revision += 1;
        return snapshot();
      default:
        return true;
    }
  },
}));
const lab = await import('../lib/specimen/lab-api');

test('model status and capabilities come back typed', async () => {
  expect(await lab.modelStatus()).toEqual([
    {
      model: { id: 'qwen3-4b', kind: 'text', name: 'Qwen' },
      installed: true,
      loaded: false,
    },
  ]);
  expect((await lab.capabilities()).ocr).toBe(true);
  expect(await lab.activeJobs()).toEqual(['job-1']);
});

test('generate sends the job, model, prompt and seed and returns the artifact', async () => {
  // Writes carry the revision the app accepted (loadNative), not one merely
  // read for display (loadSnapshot no longer accepts).
  const { loadNative, persistNative } = await import('../lib/specimen/native');
  await loadNative();
  await persistNative({ name: `lab-${Math.random()}` }).catch(() => {});
  const base = sent.findLast((r) => r.op === 'edit')?.revision;
  expect(typeof base).toBe('number');
  const { artifact } = await lab.generate('qwen3-4b', 'p', 7, 'job-2');
  expect(sent.at(-1)).toMatchObject({
    op: 'generate',
    modelId: 'qwen3-4b',
    prompt: 'p',
    seed: 7,
    jobId: 'job-2',
  });
  // The accepted revision after that edit: never older, never missing.
  expect(sent.at(-1).revision).toBeGreaterThanOrEqual(base);
  expect(artifact.id).toBe('a1');
});

test('artifacts are parsed, not cast: malformed records are dropped', () => {
  const parsed = lab.artifactsOf(
    snapshot({
      artifacts: [
        {
          id: 'ok',
          model: 'm',
          prompt: 'p',
          seed: 1,
          modelDigest: 'd',
          createdAt: 't',
          text: 'x',
        },
        { id: 42 },
        null,
      ],
    }),
  );
  expect(parsed.map((a) => a.id)).toEqual(['ok']);
});

test('a busy model store is reported, not treated as an empty list', async () => {
  const { invoke } = await import('@tauri-apps/api/core');
  expect(lab.isBusy({ busy: true })).toBe(true);
  expect(lab.isBusy([])).toBe(false);
  expect(invoke).toBeDefined();
});
