import { expect, test } from 'bun:test';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

test('retains the failing package verifier output in the runner log', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'wdbx-package-diagnostics-'));
  try {
    const verifier = join(directory, 'verifier.py');
    await writeFile(
      verifier,
      'import sys\nprint("fixture payload digest differs", file=sys.stderr)\nsys.exit(7)\n',
    );
    const result = Bun.spawnSync([
      'python3',
      '-c',
      'import importlib.util,sys\n' +
        'spec=importlib.util.spec_from_file_location("packages",sys.argv[1])\n' +
        'module=importlib.util.module_from_spec(spec)\nspec.loader.exec_module(module)\n' +
        'module.run(sys.executable,sys.argv[2])',
      resolve(import.meta.dir, '../scripts/check-linux-packages.py'),
      verifier,
    ]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(
      'fixture payload digest differs',
    );
    expect(result.stderr.toString()).toContain('exit status 7');
  } finally {
    await rm(directory, { recursive: true });
  }
});
