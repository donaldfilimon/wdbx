import { expect, test } from 'bun:test';
import { resolve } from 'node:path';

const repository = resolve(import.meta.dir, '..');
const script = resolve(repository, 'scripts/sign-notarize-macos.py');

test('refuses an Apple Development signing identity', () => {
  const result = Bun.spawnSync(
    [
      'python3',
      script,
      '--input',
      'missing.dmg',
      '--identity',
      'Apple Development: Example Developer (TEAMID)',
      '--keychain-profile',
      'missing-profile',
      '--output-dir',
      'work/notarized',
    ],
    { cwd: repository, stderr: 'pipe', stdout: 'pipe' },
  );

  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain(
    'Developer ID Application identity is required',
  );
});
