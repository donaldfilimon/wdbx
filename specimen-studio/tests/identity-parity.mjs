// Standalone, opt-in runtime witness. No corpus or golden is read or rewritten.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const [nativePath, wasmPath, defaultWasmPath, outputPath] = process.argv.slice(2);
assert.ok(nativePath && wasmPath && defaultWasmPath && outputPath,
  'usage: bun tests/identity-parity.mjs NATIVE_JSON FEATURE_WASM DEFAULT_WASM OUTPUT_JSON');
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const range = (first, last) => Array.from({ length: last - first + 1 }, (_, i) => first + i);
const native = readFileSync(nativePath);
const wasm = readFileSync(wasmPath);
const production = readFileSync(defaultWasmPath);
const defaultModule = await WebAssembly.compile(production);
const defaultExports = WebAssembly.Module.exports(defaultModule).map(({ name }) => name);
assert.ok(!defaultExports.includes('identity_probe_raw'), 'production must omit identity export');
for (const name of ['memory', 'alloc', 'free', 'call', 'probe_raw']) {
  assert.ok(defaultExports.includes(name), `missing production export ${name}`);
}
const unexpectedHostCall = () => { throw new Error('identity fixture invoked a host service'); };
const { instance } = await WebAssembly.instantiate(wasm, {
  env: { now_ms: unexpectedHostCall, random_u32: unexpectedHostCall, progress: unexpectedHostCall },
});
const { identity_probe_raw: probe, free, memory } = instance.exports;
assert.equal(typeof probe, 'function', 'missing opt-in identity export');
assert.equal(typeof free, 'function', 'missing buffer release');
assert.ok(memory instanceof WebAssembly.Memory);
const packed = probe();
assert.equal(typeof packed, 'bigint');
const ptr = Number((packed >> 32n) & 0xffffffffn);
const len = Number(packed & 0xffffffffn);
assert.ok(ptr > 0 && len > 0 && len <= 65536, 'invalid bounded result buffer');
assert.ok(ptr + len <= memory.buffer.byteLength, 'result exceeds WASM memory');
let actual;
try {
  actual = Buffer.from(new Uint8Array(memory.buffer, ptr, len));
} finally {
  free(ptr, len);
}
writeFileSync(outputPath, actual);
assert.deepEqual(actual, native, 'native and WASM runtime bytes differ');
const out = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(actual));
assert.equal(out.schema, 'task8-identity-mechanism-probe-v1');
assert.equal(Object.hasOwn(out, 'error'), false, 'shared error output is not parity');
assert.deepEqual(out.capacity, {
  full: 'Full', full_unchanged: true,
  collision_error: 'DuplicateRef', collision_unchanged: true,
  spill: { kind: 'Inserted', node: 2, entry: 1021 },
  repeat: { kind: 'Duplicate', node: 1, entry: 1001 },
  nodes: [1, 2], refs: range(1001, 1021),
});
assert.deepEqual(out.collision, {
  refs: [2001, 2002], original_bytes: [[195, 169], [101, 204, 129]],
});
assert.deepEqual(out.keys, {
  lookups: [[3001], [3002], [3003], [3004], [3005]],
  wrong_key_error: 'KeyMismatch', unchanged: true,
});
assert.deepEqual(out.lookup, {
  array: range(4001, 4064), indexed: range(4001, 4065), rebuilt: range(4001, 4065),
  reversed: range(4001, 4065).reverse(), final_original: 'indexed 64',
});
assert.deepEqual(out.resources, {
  array_links: range(5001, 5064), indexed_links: range(5001, 5065),
  structural: range(5001, 5066), unlinked_original: 'unlinked original',
  rebuilt_links: range(5001, 5065), reversed_links: range(5001, 5065).reverse(),
});
assert.deepEqual(out.invalid, {
  insert_error: 'InvalidWeight', insert_unchanged: true,
  import_error: 'DuplicateRef', import_unchanged: true,
});
assert.deepEqual(out.inherited_order, [...range(1001, 1065), 2001, 2002, 2003, 2001, 2002]);
assert.deepEqual(Object.keys(out).sort(),
  ['schema', 'capacity', 'collision', 'keys', 'lookup', 'resources', 'invalid', 'inherited_order'].sort());
console.log(`default-wasm sha256=${hash(production)} identity_probe_raw=absent`);
console.log(`feature-wasm sha256=${hash(wasm)} identity_probe_raw=executed`);
console.log(`native-output sha256=${hash(native)} bytes=${native.length}`);
console.log(`wasm-output sha256=${hash(actual)} bytes=${actual.length}`);
console.log('PASS: exact runtime bytes and independent fixture properties');
