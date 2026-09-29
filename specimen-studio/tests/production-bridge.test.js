import { expect, test } from 'bun:test';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';

const script = resolve(
  import.meta.dir,
  '../scripts/check-production-bridge.py',
);

function check(mode, input) {
  return Bun.spawnSync(
    [
      'python3',
      '-c',
      `
import importlib.util, json, pathlib, sys
spec = importlib.util.spec_from_file_location('bridge', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
if sys.argv[2] == 'assets':
    print(json.dumps(module.inspect_assets(pathlib.Path(sys.argv[3]))))
else:
    print(json.dumps(module.verify_dependency_tree(sys.argv[3])))
`,
      script,
      mode,
      input,
    ],
    { stdout: 'pipe', stderr: 'pipe' },
  );
}

for (const [label, javascript, error] of [
  ['production assets', 'console.log("studio");', null],
  ['native call bridge', 'window.__wdbxTestCall=callNative;', '__wdbxTestCall'],
  ['WDIO bridge', 'window.wdioTauri={};', 'wdioTauri'],
  ['WDIO spy', 'window.__wdio_spy__={};', '__wdio_spy__'],
  ['missing scripts', null, 'no JavaScript'],
  ['empty scripts', '', 'asset is empty'],
]) {
  test(`production exclusion ${error ? 'rejects' : 'accepts'} ${label}`, async () => {
    const directory = await mkdtemp(
      resolve(tmpdir(), 'wdbx-production-assets-'),
    );
    try {
      await writeFile(
        resolve(directory, 'index.html'),
        '<div id="root"></div>',
      );
      await mkdir(resolve(directory, 'assets'));
      if (javascript !== null)
        await writeFile(resolve(directory, 'assets/app.js'), javascript);
      const result = check('assets', directory);
      if (error) {
        expect(result.exitCode).not.toBe(0);
        expect(result.stderr.toString()).toContain(error);
      } else {
        expect(result.exitCode, result.stderr.toString()).toBe(0);
        const files = JSON.parse(result.stdout.toString());
        expect(files).toHaveLength(2);
        expect(files[1].sha256).toMatch(/^[a-f0-9]{64}$/);
      }
    } finally {
      await rm(directory, { recursive: true });
    }
  });
}

for (const forbidden of [
  'tauri-plugin-wdio',
  'tauri-plugin-wdio-webdriver',
  'wdio-webdriver',
]) {
  test(`production exclusion rejects normal dependency ${forbidden}`, () => {
    const result = check(
      'tree',
      `wdbx-studio-desktop v0.2.0\ntauri v2.0.0\n${forbidden} v1.3.0`,
    );
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain('contains test dependencies');
  });
}

test('production exclusion accepts the normal production graph', () => {
  const result = check(
    'tree',
    'wdbx-studio-desktop v0.2.0\ntauri v2.0.0\nserde v1.0.0',
  );
  expect(result.exitCode, result.stderr.toString()).toBe(0);
});

test('production exclusion rejects an empty or unrelated dependency graph', () => {
  for (const tree of [
    '',
    'specimen-core v0.2.0\ntauri v2.0.0',
    'wdbx-studio-desktop v0.2.0',
  ]) {
    expect(check('tree', tree).exitCode).not.toBe(0);
  }
});
