import { afterEach, expect, test } from 'bun:test';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const repository = resolve(import.meta.dir, '..');
const script = join(repository, 'scripts/package-evidence.py');
const temporary = [];
const headSha = '0123456789abcdef0123456789abcdef01234567';

afterEach(async () => {
  await Promise.all(
    temporary.splice(0).map((path) => rm(path, { recursive: true })),
  );
});

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'wdbx-package-evidence-'));
  temporary.push(root);
  const files = {
    application: 'target/release/app.bin',
    helper: 'src-tauri/binaries/helper',
    installer: 'target/release/bundle/app.dmg',
    receipt: 'work/package-qualification.json',
  };
  for (const [name, relative] of Object.entries(files)) {
    if (name === 'receipt') continue;
    await mkdir(dirname(join(root, relative)), { recursive: true });
  }
  await writeFile(join(root, files.application), 'app');
  await writeFile(join(root, files.helper), 'helper');
  await writeFile(join(root, files.installer), 'installer');
  return { root, files };
}

function run(...args) {
  return Bun.spawnSync(['python3', script, ...args], {
    cwd: repository,
    stderr: 'pipe',
    stdout: 'pipe',
  });
}

async function createReceipt() {
  const { root, files } = await fixture();
  const result = run(
    'create',
    '--root',
    root,
    '--output',
    files.receipt,
    '--head-sha',
    headSha,
    '--workflow-run-id',
    '12345',
    '--runner-label',
    'macos-14',
    '--runner-os',
    'macOS',
    '--runner-arch',
    'ARM64',
    '--application',
    files.application,
    '--helper',
    files.helper,
    '--installer',
    files.installer,
  );
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  return { root, files };
}

test('creates and verifies a current package qualification receipt', async () => {
  const { root, files } = await createReceipt();
  const receipt = JSON.parse(await readFile(join(root, files.receipt), 'utf8'));

  expect(receipt).toEqual({
    schema: 1,
    headSha,
    workflowRunId: '12345',
    runner: { label: 'macos-14', os: 'macOS', arch: 'ARM64' },
    application: {
      path: files.application,
      sha256:
        'a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333',
    },
    helpers: [
      {
        name: 'helper',
        path: files.helper,
        sha256:
          'e81d3b0e9d82feaaf5f6e55bdff24731d7eee08632ffa63801e6397290c5d20a',
      },
    ],
    installers: [
      {
        path: files.installer,
        sha256:
          '9c0d294c05fc1d88d698034609bb81c0c69196327594e4c69d2915c80fd9850c',
      },
    ],
  });

  const verified = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '12345',
    '--required-helper',
    'helper',
  );
  expect(verified.exitCode, verified.stderr.toString()).toBe(0);
  expect(JSON.parse(verified.stdout.toString())).toEqual({
    application: files.application,
    headSha,
    helpers: ['helper'],
    installers: [files.installer],
    verified: true,
    workflowRunId: '12345',
  });
});

test('rejects a receipt for a different source commit', async () => {
  const { root, files } = await createReceipt();
  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    'abcdef0123456789abcdef0123456789abcdef01',
    '--expected-run-id',
    '12345',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('Receipt source commit differs');
});

test('rejects a receipt from an unrelated workflow run', async () => {
  const { root, files } = await createReceipt();
  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '98765',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('Receipt workflow run differs');
});

test('rejects evidence from a different runner', async () => {
  const { root, files } = await createReceipt();
  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '12345',
    '--expected-runner-label',
    'macos-15-intel',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('Receipt runner differs');
});

test('rejects a malformed digest', async () => {
  const { root, files } = await createReceipt();
  const path = join(root, files.receipt);
  const receipt = JSON.parse(await readFile(path, 'utf8'));
  receipt.application.sha256 = 'not-a-digest';
  await writeFile(path, `${JSON.stringify(receipt, null, 2)}\n`);

  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '12345',
    '--required-helper',
    'helper',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('Invalid SHA-256 digest');
});

test('rejects a missing required helper', async () => {
  const { root, files } = await createReceipt();
  const path = join(root, files.receipt);
  const receipt = JSON.parse(await readFile(path, 'utf8'));
  receipt.helpers = [];
  await writeFile(path, `${JSON.stringify(receipt, null, 2)}\n`);

  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '12345',
    '--required-helper',
    'helper',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('Missing required helper');
});

test('rejects an application changed after qualification', async () => {
  const { root, files } = await createReceipt();
  await writeFile(join(root, files.application), 'tampered');

  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '12345',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('Qualified file digest differs');
});

test('can validate an installer-only artifact before extraction', async () => {
  const { root, files } = await createReceipt();
  await rm(join(root, files.application));
  await rm(join(root, files.helper));

  const result = run(
    'verify',
    '--root',
    root,
    '--receipt',
    files.receipt,
    '--expected-head-sha',
    headSha,
    '--expected-run-id',
    '12345',
    '--required-helper',
    'helper',
    '--file-scope',
    'installers',
  );
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  expect(JSON.parse(result.stdout.toString())).toMatchObject({
    headSha,
    installers: [files.installer],
    verified: true,
  });
});
