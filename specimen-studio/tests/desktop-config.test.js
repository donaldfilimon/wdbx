import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const conf = JSON.parse(
  readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url)),
);
const directive = (name) =>
  conf.app.security.csp
    .split(';')
    .map((d) => d.trim().split(/\s+/))
    .find(([n]) => n === name) ?? [];

test('the desktop CSP lets the webview fetch and compile the kernel WASM', () => {
  // fetch() is governed by connect-src (default-src does not apply once
  // connect-src is present); WebAssembly compilation needs wasm-unsafe-eval.
  expect(directive('connect-src')).toContain("'self'");
  expect(directive('script-src')).toContain("'wasm-unsafe-eval'");
});

test('desktop hooks build the kernel WASM before the web assets', () => {
  for (const hook of ['beforeDevCommand', 'beforeBuildCommand'])
    expect(conf.build[hook].startsWith('bun run build:wasm && ')).toBe(true);
});
