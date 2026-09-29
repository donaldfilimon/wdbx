import { expect, mock, test } from 'bun:test';

const sent = [];
let revision = 5;
mock.module('@tauri-apps/api/core', () => ({
  Channel: class {
    onmessage = () => {};
  },
  invoke: async (_cmd, { request }) => {
    sent.push(request);
    if (request.op === 'network' && request.revision !== revision)
      throw {
        code: 'StaleRevision',
        message: 'Workspace changed; refresh before retrying',
      };
    if (request.op === 'network') revision += 1;
    return {
      schema: 'wdbx.native.v2',
      revision,
      specimen: null,
      network: request.network ?? { version: 1, layers: [] },
    };
  },
}));
const { loadNativeNetwork, persistNativeNetwork, loadNative } =
  await import('../lib/specimen/native');

test('a network edit commits at the revision it was loaded from', async () => {
  const loaded = await loadNativeNetwork();
  expect(loaded.revision).toBe(5);
  const saved = await persistNativeNetwork(
    { version: 1, layers: ['edited'] },
    loaded.revision,
  );
  expect(sent.at(-1)).toMatchObject({ op: 'network', revision: 5 });
  expect(saved).toMatchObject({ revision: 6, network: { layers: ['edited'] } });
});

test('an import in between makes the edit stale instead of overwriting it', async () => {
  const loaded = await loadNativeNetwork(); // revision 6
  revision = 7; // another path (an import) committed meanwhile
  await loadNative(); // the app sees the new snapshot
  await expect(
    persistNativeNetwork({ version: 1, layers: ['old edit'] }, loaded.revision),
  ).rejects.toMatchObject({ code: 'StaleRevision' });
});

test('reading the network does not move the revision other writes are checked against', async () => {
  const { persistNative } = await import('../lib/specimen/native');
  const accepted = (await loadNative(), revision); // this app has seen `accepted`
  revision = accepted + 5; // another writer committed since
  await loadNativeNetwork(); // must be a read, not an accept
  sent.length = 0;
  await persistNative({ name: 'changed', nodes: [] }).catch(() => {});
  expect(sent.find((r) => r.op === 'edit')?.revision).toBe(accepted);
});
