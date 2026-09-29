import { expect, test } from 'bun:test';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const root = new URL('..', import.meta.url);
const wasmPath = new URL(
  'target/wasm32-unknown-unknown/release/specimen_wasm.wasm',
  root,
);

async function probe(text) {
  const { instance } = await WebAssembly.instantiate(
    readFileSync(wasmPath),
    {},
  );
  const { memory, alloc, probe_raw } = instance.exports;
  const input = new TextEncoder().encode(text);
  const ptr = alloc(input.length);
  new Uint8Array(memory.buffer, ptr, input.length).set(input);
  const packed = probe_raw(ptr, input.length);
  const outPtr = Number(packed >> 32n);
  const outLen = Number(packed & 0xffffffffn);
  return new TextDecoder().decode(
    new Uint8Array(memory.buffer, outPtr, outLen),
  );
}

test('wasm probe reproduces the native golden digest', async () => {
  const starter = JSON.parse(
    readFileSync(new URL('conformance/starter.json', root), 'utf8'),
  );
  const out = await probe(
    JSON.stringify({ specimen: starter, input: 'What is 2 + 2?' }),
  );
  const golden = readFileSync(
    new URL('conformance/p0-probe.sha256', root),
    'utf8',
  ).trim();
  expect(createHash('sha256').update(out).digest('hex')).toBe(golden);
});

// Known divergence (VALIDATION.md, P0 finding 3): f64::sin and f64::exp differ
// in the last bit between the native and WASM libm. `test.failing` passes while
// the digests differ and fails once they match; then promote it to `test`.
test.failing(
  'wasm probe reproduces the native imagine golden digest',
  async () => {
    const starter = JSON.parse(
      readFileSync(new URL('conformance/starter.json', root), 'utf8'),
    );
    starter.atp.lastUpdate = 1767225538766;
    const out = await probe(
      JSON.stringify({
        specimen: starter,
        input: 'imagine a happy blue square',
      }),
    );
    const golden = readFileSync(
      new URL('conformance/p0-probe-imagine.sha256', root),
      'utf8',
    ).trim();
    expect(createHash('sha256').update(out).digest('hex')).toBe(golden);
  },
);

test('wasm probe reports malformed input as an error', async () => {
  const out = JSON.parse(await probe('{not json'));
  expect(out.error.code).toBe('MalformedSave');
});
