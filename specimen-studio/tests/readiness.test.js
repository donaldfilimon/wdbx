import { expect, test } from 'bun:test';
import { createReadiness } from '../lib/specimen/readiness';

test('ready settles, notifies and reports status', async () => {
  const r = createReadiness();
  const seen = [];
  const off = r.subscribe(() => seen.push(r.status()));
  expect(r.status()).toBe('loading');
  r.markReady();
  await r.ready;
  expect(r.status()).toBe('ready');
  expect(seen).toEqual(['ready']);
  off();
  r.markReady();
  expect(seen).toEqual(['ready']);
});

test('a failed load rejects waiters instead of hanging', async () => {
  const r = createReadiness();
  const seen = [];
  r.subscribe(() => seen.push(r.status()));
  r.markFailed(new Error('404 for kernel.wasm'));
  await expect(r.ready).rejects.toThrow('404 for kernel.wasm');
  expect(r.status()).toBe('failed');
  expect(r.error()).toBe('404 for kernel.wasm');
  expect(seen).toEqual(['failed']);
});
