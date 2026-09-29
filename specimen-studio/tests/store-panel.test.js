import { expect, test } from 'bun:test';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { BrowserStoreView, DesktopStoreView } from '../app/panels/store-panel';

const info = {
  root: '/tmp/store',
  writerId: 'aaaaaaaa-1111-2222-3333-444444444444',
  heads: {
    'aaaaaaaa-1111-2222-3333-444444444444': 7,
    'bbbbbbbb-1111-2222-3333-444444444444': 2,
  },
  committedTransactions: 9,
  kvCount: 1,
  vectorCount: 0,
  spatialCount: 0,
  auditCount: 0,
  auditHeads: [],
  auditDag: { ok: true, error: null },
  snapshotKey: {
    key: 'studio/snapshot',
    versionId: 'v-1',
    writerId: 'aaaaaaaa-1111-2222-3333-444444444444',
    sequence: 7,
    conflicts: 1,
    bytes: 2048,
  },
  schema: 'wdbx.native.v2',
  revision: 7,
  records: { nodes: 3, entries: 5 },
  tombstones: [
    { name: 'Greeting', ref: 'n1', deletedAt: '2026-09-29T06:00:00Z' },
  ],
  disk: {
    store: { files: 4, bytes: 4096, skipped: 1 },
    assets: { files: 0, bytes: 0, skipped: 0 },
  },
};

test('desktop view shows frontier, key version, conflicts and tombstones', () => {
  const out = renderToStaticMarkup(h(DesktopStoreView, { info }));
  expect(out).toContain('studio/snapshot');
  expect(out).toContain('1 unresolved concurrent version');
  // A new writer id is minted per launch: say session, not app or device.
  expect(out).toContain('This session');
  expect(out).toContain('Writer sessions');
  expect(out).toContain('1 entry could not be read');
  expect(out).toContain('Greeting');
  expect(out).toContain('2.0 KiB');
  expect(out).toContain('the studio writes no audit blocks');
  expect(out).not.toContain('hashes match');
});

test('a failed audit DAG is an alert naming the error', () => {
  const out = renderToStaticMarkup(
    h(DesktopStoreView, {
      info: {
        ...info,
        auditCount: 2,
        auditDag: { ok: false, error: 'audit cycle reaches x' },
      },
    }),
  );
  expect(out).toContain('role="alert"');
  expect(out).toContain('audit cycle reaches x');
});

test('browser view is honest about best-effort storage and offers persistence', () => {
  const report = {
    bytes: 1500,
    updatedAt: '2026-09-29T06:00:00.000Z',
    records: { nodes: 2 },
    usage: 4096,
    quota: 1024 ** 3,
    persisted: false,
  };
  const out = renderToStaticMarkup(
    h(BrowserStoreView, { report, onPersist: () => {} }),
  );
  expect(out).toContain('best-effort');
  expect(out).toContain('Keep storage persistent');
  expect(out).toContain('no version history');
  const kept = renderToStaticMarkup(
    h(BrowserStoreView, {
      report: { ...report, persisted: true },
      onPersist: () => {},
    }),
  );
  expect(kept).not.toContain('Keep storage persistent');
});
