import { expect, test } from 'bun:test';
import { resolve } from 'node:path';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';

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

for (const scenario of ['current', 'stale', 'altered']) {
  test(`macOS signing preflight ${scenario === 'current' ? 'accepts' : 'rejects'} ${scenario} portable image evidence`, async () => {
    const directory = await mkdtemp(
      resolve(tmpdir(), 'wdbx-signing-evidence-'),
    );
    try {
      const content = 'qualified portable image';
      await writeFile(
        resolve(directory, 'input.dmg'),
        scenario === 'altered' ? 'modified' : content,
      );
      await writeFile(
        resolve(directory, 'receipt.json'),
        JSON.stringify({
          sourceSha: (scenario === 'stale' ? 'b' : 'a').repeat(40),
          adHocSeal: {
            dmgSha256: createHash('sha256').update(content).digest('hex'),
          },
        }),
      );
      const result = Bun.spawnSync(
        [
          'python3',
          '-c',
          `
import importlib.util, pathlib, sys
spec = importlib.util.spec_from_file_location('signing', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
module.captured = lambda arguments: 'a' * 40
module.find_qualification_receipt = lambda image: image.parent / 'receipt.json'
module.verify_current_head(pathlib.Path(sys.argv[2]) / 'input.dmg')
`,
          script,
          directory,
        ],
        { stderr: 'pipe', stdout: 'pipe' },
      );
      if (scenario === 'current') {
        expect(result.exitCode, result.stderr.toString()).toBe(0);
      } else {
        expect(result.exitCode).not.toBe(0);
        expect(result.stderr.toString()).toContain(
          scenario === 'stale'
            ? 'does not match the current Git commit'
            : 'digest differs from its qualification receipt',
        );
      }
    } finally {
      await rm(directory, { recursive: true });
    }
  });
}
