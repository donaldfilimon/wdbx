import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { BROWSER_EDITION_LOSS, lossDeclaration } from '../lib/specimen/canonical-loss';

/**
 * The native edition's own vocabulary, read from its manifest.
 *
 * The two editions share one closed vocabulary: the native gate reads this
 * edition's manifest into the native ids, so an id one edition declares is an
 * id the other knows. The read is a structural check, like the native gate's
 * own.
 */
const NATIVE_MANIFEST = readFileSync(
  new URL(
    '../native/specimen-core/src/canonical/manifest.rs',
    import.meta.url,
  ),
  'utf8',
);

const BROWSER_VOCABULARY = readFileSync(
  new URL('../lib/specimen/canonical-loss.ts', import.meta.url),
  'utf8',
);

/** Every id the native edition's vocabulary names. */
function nativeLossIds() {
  const ids = new Set();
  for (const match of NATIVE_MANIFEST.matchAll(/Self::[A-Za-z]+ => "([a-z_]+)"/g)) {
    ids.add(match[1]);
  }
  return ids;
}

/** Every id the browser edition's vocabulary names. */
function browserLossIds() {
  const union = BROWSER_VOCABULARY.match(/export type LossId =([\s\S]*?);/);
  expect(union).toBeTruthy();
  const ids = new Set();
  for (const match of union[1].matchAll(/'([a-z_]+)'/g)) {
    ids.add(match[1]);
  }
  return ids;
}

test('the browser edition declares what it drops from the canonical ledger', () => {
  expect(BROWSER_EDITION_LOSS).toBeTruthy();
  expect(BROWSER_EDITION_LOSS.edition).toBe('browser');
  expect(BROWSER_EDITION_LOSS.declarations.length).toBeGreaterThan(0);
  for (const declaration of BROWSER_EDITION_LOSS.declarations) {
    expect(declaration.id.length).toBeGreaterThan(0);
    expect(declaration.dropped.length).toBeGreaterThan(0);
    expect(declaration.reason.length).toBeGreaterThan(0);
  }
});

test('the browser edition names the canonical ledger it does not own', () => {
  const ledger = lossDeclaration('canonical_ledger');
  expect(ledger).toBeDefined();
  expect(ledger?.dropped).toContain('canonical episode ledger');
  expect(ledger?.reason).toContain('lossy projection');
});

test('the browser edition declares the commitment it drops', () => {
  const commitment = lossDeclaration('payload_commitment');
  expect(commitment).toBeDefined();
  expect(commitment?.dropped.length).toBeGreaterThan(0);
  expect(commitment?.reason.length).toBeGreaterThan(0);
});

test('the browser edition declares the signature evidence it drops', () => {
  const signature = lossDeclaration('signature_evidence');
  expect(signature).toBeDefined();
  expect(signature?.dropped.length).toBeGreaterThan(0);
  expect(signature?.reason.length).toBeGreaterThan(0);
});

test('the browser edition declares the replication scope it drops', () => {
  const replication = lossDeclaration('replication_scope');
  expect(replication).toBeDefined();
  expect(replication?.dropped.length).toBeGreaterThan(0);
  expect(replication?.reason.length).toBeGreaterThan(0);
});

test('the two editions share one loss vocabulary', () => {
  const native = nativeLossIds();
  const browser = browserLossIds();
  expect(native.size).toBeGreaterThan(0);
  for (const id of native) {
    expect(browser.has(id)).toBe(true);
  }
  for (const id of browser) {
    expect(native.has(id)).toBe(true);
  }
});
