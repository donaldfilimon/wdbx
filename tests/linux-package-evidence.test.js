import { afterEach, expect, test } from 'bun:test';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repository = resolve(import.meta.dir, '..');
const script = join(repository, 'scripts/linux_package_evidence.py');
const temporary = [];
const sourceSha = '0123456789abcdef0123456789abcdef01234567';

afterEach(async () => {
  await Promise.all(
    temporary.splice(0).map((path) => rm(path, { recursive: true })),
  );
});

function sha256(value) {
  return new Bun.CryptoHasher('sha256').update(value).digest('hex');
}

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'wdbx-linux-stage-'));
  temporary.push(root);
  const appDir = join(root, 'WDBX Specimen Studio.AppDir');
  const extracted = join(root, 'extracted');
  const retained = join(root, 'retained');
  await mkdir(join(appDir, 'usr/bin'), { recursive: true });
  await mkdir(join(appDir, 'usr/lib/WDBX Specimen Studio/binaries'), {
    recursive: true,
  });
  await mkdir(extracted);
  const raw = { app: 'raw-app', llama: 'raw-llama', sd: 'raw-sd' };
  const staged = { app: 'staged-app', llama: raw.llama, sd: 'staged-sd' };
  const paths = {
    app: join(appDir, 'usr/bin/wdbx-studio-desktop'),
    llama: join(appDir, 'usr/lib/WDBX Specimen Studio/binaries/llama-server'),
    sd: join(appDir, 'usr/lib/WDBX Specimen Studio/binaries/sd-cli'),
  };
  for (const key of Object.keys(paths))
    await writeFile(paths[key], staged[key]);
  const installer = join(root, 'studio.AppImage');
  await writeFile(installer, 'appimage');
  const debInstaller = join(root, 'studio.deb');
  await writeFile(debInstaller, 'debian installer');
  const base = join(root, 'package-qualification.json');
  await writeFile(
    base,
    JSON.stringify({
      schema: 1,
      headSha: sourceSha,
      workflowRunId: '101',
      runner: { label: 'ubuntu-24.04', os: 'Linux', arch: 'X64' },
      application: {
        path: 'target/wdbx-studio-desktop',
        sha256: sha256(raw.app),
      },
      helpers: [
        {
          name: 'llama-server',
          path: 'binaries/llama-server',
          sha256: sha256(raw.llama),
        },
        { name: 'sd-cli', path: 'binaries/sd-cli', sha256: sha256(raw.sd) },
      ],
      installers: [
        { path: 'studio.AppImage', sha256: sha256('appimage') },
        { path: 'studio.deb', sha256: sha256('debian installer') },
      ],
    }),
  );
  return {
    root,
    appDir,
    extracted,
    retained,
    installer,
    debInstaller,
    base,
    raw,
    staged,
  };
}

async function debStage(value) {
  const data = join(value.root, 'bundle/deb/studio_0.2.0_amd64/data');
  const staged = join(value.root, 'deb-stage');
  const retained = join(value.root, 'deb-retained');
  const receipt = join(value.root, 'linux-deb-qualification.json');
  await mkdir(join(data, 'usr/bin'), { recursive: true });
  await mkdir(join(data, 'usr/lib/WDBX Specimen Studio/binaries'), {
    recursive: true,
  });
  await writeFile(join(data, 'usr/bin/wdbx-studio-desktop'), 'deb-patched-app');
  await writeFile(
    join(data, 'usr/lib/WDBX Specimen Studio/binaries/llama-server'),
    value.raw.llama,
  );
  await writeFile(
    join(data, 'usr/lib/WDBX Specimen Studio/binaries/sd-cli'),
    value.raw.sd,
  );
  const snapshot = run(
    'snapshot-deb',
    '--root',
    value.root,
    '--bundle-deb-root',
    join(value.root, 'bundle/deb'),
    '--output',
    staged,
  );
  expect(snapshot.exitCode, snapshot.stderr.toString()).toBe(0);
  const result = run(
    'create-deb',
    '--root',
    value.root,
    '--base-receipt',
    value.base,
    '--staged-payload',
    staged,
    '--installer',
    value.debInstaller,
    '--output',
    receipt,
    '--retained-payload',
    retained,
    '--expected-source-sha',
    sourceSha,
    '--expected-run-id',
    '101',
  );
  return { data, staged, retained, receipt, result };
}

function verifyDeb(
  value,
  receipt,
  installed,
  source = sourceSha,
  runId = '101',
) {
  return run(
    'verify-deb',
    '--root',
    value.root,
    '--receipt',
    receipt,
    '--base-receipt',
    value.base,
    '--installer',
    value.debInstaller,
    '--expected-source-sha',
    source,
    '--expected-run-id',
    runId,
    ...installed.flatMap((path) => ['--installed-payload', path]),
  );
}

test('binds an independent pre-installer Debian stage and verifies installed bytes', async () => {
  const value = await fixture();
  const { staged, receipt, result } = await debStage(value);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const installed = ['wdbx-studio-desktop', 'llama-server', 'sd-cli'].map(
    (name) => join(staged, name),
  );
  expect(verifyDeb(value, receipt, installed).exitCode).toBe(0);
});

test('snapshots only one actual Tauri Debian data tree and refuses replacement', async () => {
  const value = await fixture();
  const created = await debStage(value);
  const replacement = run(
    'snapshot-deb',
    '--root',
    value.root,
    '--bundle-deb-root',
    join(value.root, 'bundle/deb'),
    '--output',
    created.staged,
  );
  expect(replacement.exitCode).not.toBe(0);
  expect(replacement.stderr.toString()).toContain('refusing to replace');
  await mkdir(join(value.root, 'bundle/deb/other/data'), { recursive: true });
  const ambiguous = run(
    'snapshot-deb',
    '--root',
    value.root,
    '--bundle-deb-root',
    join(value.root, 'bundle/deb'),
    '--output',
    join(value.root, 'other-stage'),
  );
  expect(ambiguous.exitCode).not.toBe(0);
  expect(ambiguous.stderr.toString()).toContain(
    'exactly one Tauri Debian data root',
  );
});

test('rejects tampered or missing retained Debian stage payload', async () => {
  const value = await fixture();
  const { staged, retained, receipt } = await debStage(value);
  const installed = ['wdbx-studio-desktop', 'llama-server', 'sd-cli'].map(
    (name) => join(staged, name),
  );
  await writeFile(join(retained, 'wdbx-studio-desktop'), 'tampered');
  expect(verifyDeb(value, receipt, installed).stderr.toString()).toContain(
    'retained staged Debian qualification differs',
  );
  await rm(join(retained, 'wdbx-studio-desktop'));
  expect(verifyDeb(value, receipt, installed).exitCode).not.toBe(0);
});

test('rejects a missing executable before sealing the Debian stage', async () => {
  const value = await fixture();
  const created = await debStage(value);
  await rm(join(created.data, 'usr/lib/WDBX Specimen Studio/binaries/sd-cli'));
  await rm(created.staged, { recursive: true });
  await rm(created.retained, { recursive: true });
  await rm(created.receipt);
  const result = run(
    'snapshot-deb',
    '--root',
    value.root,
    '--bundle-deb-root',
    join(value.root, 'bundle/deb'),
    '--output',
    created.staged,
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain(
    'exactly one Tauri Debian staged executable',
  );
});

test('rejects stale Debian linkage, replaced installer, and installed mismatch', async () => {
  const value = await fixture();
  const { staged, receipt } = await debStage(value);
  const installed = ['wdbx-studio-desktop', 'llama-server', 'sd-cli'].map(
    (name) => join(staged, name),
  );
  expect(
    verifyDeb(value, receipt, installed, 'f'.repeat(40)).exitCode,
  ).not.toBe(0);
  expect(
    verifyDeb(value, receipt, installed, sourceSha, '202').exitCode,
  ).not.toBe(0);
  await writeFile(value.debInstaller, 'replaced');
  expect(verifyDeb(value, receipt, installed).stderr.toString()).toContain(
    'Debian installer digest differs',
  );
  await writeFile(value.debInstaller, 'debian installer');
  await writeFile(installed[0], 'installed mismatch');
  expect(verifyDeb(value, receipt, installed).stderr.toString()).toContain(
    'installed payload differs from staged Debian qualification',
  );
});

function run(...args) {
  return Bun.spawnSync(['python3', script, ...args], {
    cwd: repository,
    stdout: 'pipe',
    stderr: 'pipe',
  });
}

async function createStage(value) {
  const receipt = join(value.root, 'linux-appdir-qualification.json');
  const result = run(
    'create',
    '--root',
    value.root,
    '--base-receipt',
    value.base,
    '--appdir',
    value.appDir,
    '--installer',
    value.installer,
    '--output',
    receipt,
    '--retained-payload',
    value.retained,
    '--expected-source-sha',
    sourceSha,
    '--expected-run-id',
    '101',
  );
  return { receipt, result };
}

test('records independent final AppDir hashes bound to raw qualification and installer', async () => {
  const value = await fixture();
  const { receipt, result } = await createStage(value);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const evidence = JSON.parse(await readFile(receipt, 'utf8'));
  expect(evidence).toMatchObject({
    schema: 1,
    sourceSha,
    workflowRunId: '101',
    installer: { sha256: sha256('appimage') },
  });
  expect(evidence.payload.map((item) => item.name)).toEqual([
    'wdbx-studio-desktop',
    'llama-server',
    'sd-cli',
  ]);
});

test('rejects extracted AppImage bytes that differ from the independent AppDir stage', async () => {
  const value = await fixture();
  const { receipt } = await createStage(value);
  await cp(value.appDir, value.extracted, { recursive: true });
  await writeFile(
    join(value.extracted, 'usr/bin/wdbx-studio-desktop'),
    'tampered',
  );
  const result = run(
    'verify-appimage',
    '--root',
    value.root,
    '--receipt',
    receipt,
    '--base-receipt',
    value.base,
    '--installer',
    value.installer,
    '--extracted',
    value.extracted,
    '--expected-source-sha',
    sourceSha,
    '--expected-run-id',
    '101',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('staged AppDir qualification');
});

test('rejects an AppImage whose digest no longer matches the bound base receipt', async () => {
  const value = await fixture();
  const { receipt } = await createStage(value);
  await cp(value.appDir, value.extracted, { recursive: true });
  await writeFile(value.installer, 'replaced appimage');
  const result = run(
    'verify-appimage',
    '--root',
    value.root,
    '--receipt',
    receipt,
    '--base-receipt',
    value.base,
    '--installer',
    value.installer,
    '--extracted',
    value.extracted,
    '--expected-source-sha',
    sourceSha,
    '--expected-run-id',
    '101',
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain(
    'AppImage digest differs from base qualification',
  );
});

for (const [label, mutate, error] of [
  [
    'missing executable',
    async (value) => rm(join(value.appDir, 'usr/bin/wdbx-studio-desktop')),
    'exactly one staged',
  ],
  [
    'duplicate executable',
    async (value) =>
      writeFile(join(value.appDir, 'wdbx-studio-desktop'), 'duplicate'),
    'exactly one staged',
  ],
]) {
  test(`rejects ${label} in the final AppDir`, async () => {
    const value = await fixture();
    await mutate(value);
    const { result } = await createStage(value);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
  });
}

test('rejects stale source and run linkage', async () => {
  const value = await fixture();
  const { receipt } = await createStage(value);
  await cp(value.appDir, value.extracted, { recursive: true });
  for (const [sha, runId, message] of [
    ['f'.repeat(40), '101', 'source SHA differs'],
    [sourceSha, '202', 'workflow run differs'],
  ]) {
    const result = run(
      'verify-appimage',
      '--root',
      value.root,
      '--receipt',
      receipt,
      '--base-receipt',
      value.base,
      '--installer',
      value.installer,
      '--extracted',
      value.extracted,
      '--expected-source-sha',
      sha,
      '--expected-run-id',
      runId,
    );
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
  }
});

test('rejects Debian payload bytes that differ from the raw build receipt', async () => {
  const value = await fixture();
  const rawRoot = join(value.root, 'deb-payload');
  await mkdir(rawRoot);
  await writeFile(join(rawRoot, 'wdbx-studio-desktop'), 'changed-by-package');
  await writeFile(join(rawRoot, 'llama-server'), value.raw.llama);
  await writeFile(join(rawRoot, 'sd-cli'), value.raw.sd);
  const result = run(
    'verify-raw',
    '--base-receipt',
    value.base,
    '--payload-root',
    rawRoot,
  );
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('raw build qualification');
});
