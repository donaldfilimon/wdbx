import { expect, test } from 'bun:test';
import {
  browserStoreReport,
  formatBytes,
  recordCounts,
} from '../lib/specimen/store-info';

const specimen = {
  schema: 'wdbx.studio.v1',
  nodes: [
    { ref: 'a', entries: [{}, {}] },
    { ref: 'b', entries: [{}] },
  ],
  resources: [{}],
  attachments: [],
  history: [{}, {}, {}],
  events: [{}],
  mutations: [],
  proposals: [],
  updatedAt: '2026-09-29T06:00:00.000Z',
};

test('formatBytes uses binary units and stays exact for small sizes', () => {
  expect(formatBytes(0)).toBe('0 B');
  expect(formatBytes(1023)).toBe('1023 B');
  expect(formatBytes(1536)).toBe('1.5 KiB');
  expect(formatBytes(5 * 1024 ** 3)).toBe('5.0 GiB');
  expect(formatBytes(undefined)).toBe('—');
});

test('recordCounts counts collections and entries', () => {
  expect(recordCounts(specimen)).toEqual({
    nodes: 2,
    entries: 3,
    resources: 1,
    attachments: 0,
    history: 3,
    events: 1,
    mutations: 0,
    proposals: 0,
  });
});

test('browserStoreReport measures the stored record in UTF-8 bytes', () => {
  const r = browserStoreReport(specimen, { usage: 2048, quota: 4096 }, true);
  expect(r.bytes).toBe(
    new TextEncoder().encode(JSON.stringify(specimen)).length,
  );
  expect(r.updatedAt).toBe('2026-09-29T06:00:00.000Z');
  expect(r.records.entries).toBe(3);
  expect(r.usage).toBe(2048);
  expect(r.quota).toBe(4096);
  expect(r.persisted).toBe(true);
});

test('browserStoreReport is honest when nothing is saved or APIs are missing', () => {
  const r = browserStoreReport(null, null, null);
  expect(r).toEqual({
    bytes: 0,
    updatedAt: null,
    records: null,
    usage: null,
    quota: null,
    persisted: null,
  });
});
