import { afterEach, expect, test } from 'bun:test';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const repository = resolve(import.meta.dir, '..');
const script = join(repository, 'scripts/qualification-summary.py');
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

async function fixture({ artifact = true, conclusion = 'success' } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'wdbx-qualification-summary-'));
  temporary.push(root);
  await mkdir(join(root, 'desktop'), { recursive: true });
  const metadata = {
    id: 101,
    html_url: 'https://github.com/studio/repo/actions/runs/101',
    head_sha: sourceSha,
    status: 'completed',
    conclusion,
    path: '.github/workflows/desktop.yml',
    repository: { full_name: 'studio/repo' },
  };
  await writeFile(join(root, 'desktop-run.json'), JSON.stringify(metadata));
  await writeFile(
    join(root, 'desktop-jobs.json'),
    JSON.stringify({ jobs: [{ name: 'desktop (ubuntu-24.04)', conclusion }] }),
  );
  if (artifact) {
    const application = 'qualified application';
    const helper = 'qualified helper';
    const installer = 'qualified installer';
    await writeFile(join(root, 'desktop/app.bin'), application);
    await writeFile(join(root, 'desktop/helper.bin'), helper);
    await writeFile(join(root, 'desktop/app.AppImage'), installer);
    await writeFile(
      join(root, 'desktop/package-qualification.json'),
      JSON.stringify({
        schema: 1,
        headSha: sourceSha,
        workflowRunId: '101',
        runner: { label: 'ubuntu-24.04', os: 'Linux', arch: 'X64' },
        application: { path: 'target/app.bin', sha256: sha256(application) },
        helpers: [
          {
            name: 'helper.bin',
            path: 'target/helper.bin',
            sha256: sha256(helper),
          },
        ],
        installers: [
          {
            path: 'target/app.AppImage',
            sha256: sha256(installer),
          },
        ],
      }),
    );
  }
  const manifest = {
    schema: 1,
    repository: 'studio/repo',
    sourceSha,
    outputJson: join(root, 'summary.json'),
    outputMarkdown: join(root, 'summary.md'),
    runs: [
      {
        kind: 'desktop',
        runId: '101',
        metadata: join(root, 'desktop-run.json'),
        artifacts: join(root, 'desktop'),
        jobs: join(root, 'desktop-jobs.json'),
      },
      { kind: 'macos-signing', required: false },
    ],
    localManualEvidence: [
      {
        label: 'local browser acceptance',
        status: 'passed',
        sourceSha,
        url: 'https://example.invalid/local-evidence.json',
        sha256: 'a'.repeat(64),
      },
    ],
  };
  await writeFile(join(root, 'manifest.json'), JSON.stringify(manifest));
  return { root, manifest: join(root, 'manifest.json') };
}

function run(manifest) {
  return Bun.spawnSync(['python3', script, '--manifest', manifest], {
    cwd: repository,
    stderr: 'pipe',
    stdout: 'pipe',
  });
}

test('consolidates exact run, artifact hash, local evidence, and signing blocker', async () => {
  const { root, manifest } = await fixture();
  const result = run(manifest);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.overall).toBe('blocked');
  expect(summary.runs[0]).toMatchObject({
    kind: 'desktop',
    status: 'passed',
    runId: '101',
    url: 'https://github.com/studio/repo/actions/runs/101',
  });
  expect(summary.runs[0].artifacts[0]).toMatchObject({
    identity: 'app.bin',
    recordedSha256: sha256('qualified application'),
    actualSha256: sha256('qualified application'),
    verification: 'passed',
  });
  expect(summary.runs[0].jobs).toEqual([
    { name: 'desktop (ubuntu-24.04)', conclusion: 'success' },
  ]);
  expect(summary.runs[1]).toMatchObject({
    kind: 'macos-signing',
    status: 'blocked',
  });
  expect(summary.localManualEvidence[0].status).toBe('passed');
  expect(await readFile(join(root, 'summary.md'), 'utf8')).toContain(
    'https://github.com/studio/repo/actions/runs/101',
  );
});

for (const [label, mutate, message] of [
  ['malformed metadata', (value) => '{', 'not valid JSON'],
  [
    'stale source',
    (value) => ({ ...value, head_sha: 'f'.repeat(40) }),
    'source SHA differs',
  ],
  [
    'unrelated repository',
    (value) => ({ ...value, repository: { full_name: 'other/repo' } }),
    'repository differs',
  ],
  [
    'unrelated workflow',
    (value) => ({ ...value, path: '.github/workflows/ocr.yml' }),
    'workflow differs',
  ],
]) {
  test(`rejects ${label}`, async () => {
    const { root, manifest } = await fixture();
    const path = join(root, 'desktop-run.json');
    const current = JSON.parse(await readFile(path, 'utf8'));
    const changed = mutate(current);
    await writeFile(
      path,
      typeof changed === 'string' ? changed : JSON.stringify(changed),
    );
    const result = run(manifest);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
  });
}

test('marks successful workflow with missing receipt as unverified', async () => {
  const { root, manifest } = await fixture({ artifact: false });
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({ status: 'unverified' });
  expect(summary.overall).toBe('unverified');
});

test('rejects a tampered artifact whose recorded digest still names the original', async () => {
  const { root, manifest } = await fixture();
  await writeFile(join(root, 'desktop/app.bin'), 'tampered');
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('digest differs');
});

test('records a completed failed job as failed without treating it as proof', async () => {
  const { root, manifest } = await fixture({
    artifact: false,
    conclusion: 'failure',
  });
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({
    status: 'failed',
    conclusion: 'failure',
  });
  expect(summary.overall).toBe('failed');
});

test('accepts the real browser-checks receipt shape without treating it as inference', async () => {
  const { root, manifest } = await fixture({ artifact: false });
  const metadataPath = join(root, 'desktop-run.json');
  const metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  metadata.path = '.github/workflows/browser.yml';
  await writeFile(metadataPath, JSON.stringify(metadata));
  await writeFile(
    join(root, 'desktop/browser-checks-chrome.json'),
    JSON.stringify({ passed: true, headSha: sourceSha, engine: 'chromium' }),
  );
  const manifestValue = JSON.parse(await readFile(manifest, 'utf8'));
  manifestValue.runs[0].kind = 'browser';
  await writeFile(manifest, JSON.stringify(manifestValue));
  const result = run(manifest);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({ kind: 'browser', status: 'passed' });
  expect(summary.runs[0].artifacts).toEqual([
    {
      identity: 'browser-checks-chrome.json',
      path: 'browser-checks-chrome.json',
      recordedSha256: null,
      actualSha256: sha256(
        JSON.stringify({
          passed: true,
          headSha: sourceSha,
          engine: 'chromium',
        }),
      ),
      verification: 'receipt-identity-only',
    },
  ]);
});

test('downgrades manual evidence without frozen-source provenance to unverified', async () => {
  const { root, manifest } = await fixture();
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.localManualEvidence[0].sourceSha = null;
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.localManualEvidence[0]).toMatchObject({
    status: 'unverified',
    blocker: 'evidence source SHA differs from frozen source',
  });
  expect(summary.overall).toBe('unverified');
});

test('rejects receipt paths that escape the downloaded artifact root', async () => {
  const { root, manifest } = await fixture();
  const receiptPath = join(root, 'desktop/package-qualification.json');
  const receipt = JSON.parse(await readFile(receiptPath, 'utf8'));
  receipt.application.path = '../outside.bin';
  await writeFile(receiptPath, JSON.stringify(receipt));
  await writeFile(join(root, 'outside.bin'), 'qualified application');
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain('unsafe recorded artifact path');
});

test('does not copy undeclared local evidence fields into outputs or logs', async () => {
  const { root, manifest } = await fixture();
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.localManualEvidence[0].credential = 'must-not-escape';
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode).toBe(0);
  expect(result.stdout.toString()).not.toContain('must-not-escape');
  expect(await readFile(join(root, 'summary.json'), 'utf8')).not.toContain(
    'must-not-escape',
  );
});

test('rejects a schema-only desktop receipt and still writes inspectable outputs', async () => {
  const { root, manifest } = await fixture();
  await writeFile(
    join(root, 'desktop/package-qualification.json'),
    JSON.stringify({ schema: 1 }),
  );
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({
    status: 'failed',
    errors: ['desktop receipt must identify the frozen source SHA'],
  });
  expect(await readFile(join(root, 'summary.md'), 'utf8')).toContain(
    'desktop receipt must identify the frozen source SHA',
  );
});

test('verifies real macOS absolute producer paths inside the receipt artifact scope', async () => {
  const { root, manifest } = await fixture();
  const artifact = join(root, 'mac-artifact');
  await mkdir(artifact);
  const inputBytes = 'portable dmg';
  const bytes = 'notarized dmg';
  await writeFile(join(artifact, 'input.dmg'), inputBytes);
  await writeFile(join(artifact, 'signed.dmg'), bytes);
  await writeFile(
    join(artifact, 'signed-notarized.json'),
    JSON.stringify({
      schema: 1,
      sourceSha,
      architecture: 'aarch64',
      input: {
        path: '/home/runner/work/repo/input.dmg',
        sha256: sha256(inputBytes),
      },
      output: {
        path: '/home/runner/work/repo/work/notarized/signed.dmg',
        sha256: sha256(bytes),
      },
      qualificationRunId: '101',
      signingIdentityFingerprint: 'A'.repeat(40),
      hardenedRuntime: true,
      secureTimestamps: true,
      signedMachOFiles: ['Contents/MacOS/app'],
      signedPayloadSha256: { 'Contents/MacOS/app': 'c'.repeat(64) },
      finalInstallerPayloadVerification: 'passed',
      notarization: { submissionId: 'submission', status: 'Accepted' },
      stapler: { staple: 'passed', validate: 'passed' },
      gatekeeper: { application: 'accepted', dmg: 'accepted' },
    }),
  );
  const metadata = JSON.parse(
    await readFile(join(root, 'desktop-run.json'), 'utf8'),
  );
  metadata.path = '.github/workflows/macos-package.yml';
  await writeFile(join(root, 'desktop-run.json'), JSON.stringify(metadata));
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.runs[0] = {
    ...value.runs[0],
    kind: 'macos-signing',
    artifacts: artifact,
  };
  value.runs.pop();
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0].domain).toMatchObject({
    notarization: 'Accepted',
    finalInstallerPayloadVerification: 'passed',
  });
  expect(summary.runs[0].artifacts).toContainEqual(
    expect.objectContaining({ identity: 'signed.dmg', verification: 'passed' }),
  );
});

test('verifies a real Windows backslash installer path and signature conclusions', async () => {
  const { root, manifest } = await fixture();
  const bytes = 'signed exe';
  await writeFile(join(root, 'desktop/signed.exe'), bytes);
  await writeFile(
    join(root, 'desktop/windows-package-verification.json'),
    JSON.stringify({
      schema: 1,
      sourceSha,
      qualificationRunId: '99',
      qualificationRunner: 'windows-2022',
      applicationQualifiedBeforeSigning: true,
      finalInstaller: {
        path: 'target\\release\\bundle\\nsis\\signed.exe',
        sha256: sha256(bytes),
      },
      timestampStatus: 'verified',
      verification: [
        {
          path: 'signed.exe',
          signtool: 'passed',
          authenticodeStatus: 'Valid',
          timestampStatus: 'verified',
        },
      ],
      installSmoke: {
        silentInstall: 'passed',
        installedHelperIsolatedStartup: 'passed',
        launch: 'passed',
        cleanShutdown: 'passed',
        silentUninstall: 'passed',
      },
    }),
  );
  const metadata = JSON.parse(
    await readFile(join(root, 'desktop-run.json'), 'utf8'),
  );
  metadata.path = '.github/workflows/windows-package.yml';
  await writeFile(join(root, 'desktop-run.json'), JSON.stringify(metadata));
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.runs[0].kind = 'windows-signing';
  value.runs.pop();
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0].domain).toMatchObject({ timestampStatus: 'verified' });
  expect(summary.runs[0].artifacts[0]).toMatchObject({
    identity: 'signed.exe',
    verification: 'passed',
  });
});

test('preserves valid layers and concrete tampering errors in one summary', async () => {
  const { root, manifest } = await fixture();
  const badRoot = join(root, 'tampered-desktop');
  await mkdir(badRoot);
  await writeFile(join(badRoot, 'app.bin'), 'tampered');
  await writeFile(join(badRoot, 'helper.bin'), 'helper');
  await writeFile(join(badRoot, 'app.AppImage'), 'installer');
  await writeFile(
    join(badRoot, 'package-qualification.json'),
    JSON.stringify({
      schema: 1,
      headSha: sourceSha,
      workflowRunId: '202',
      runner: { label: 'windows-2022', os: 'Windows', arch: 'X64' },
      application: { path: 'app.bin', sha256: sha256('original') },
      helpers: [
        {
          name: 'helper.bin',
          path: 'helper.bin',
          sha256: sha256('helper'),
        },
      ],
      installers: [{ path: 'app.AppImage', sha256: sha256('installer') }],
    }),
  );
  const runMetadata = {
    id: 202,
    html_url: 'https://github.com/studio/repo/actions/runs/202',
    head_sha: sourceSha,
    status: 'completed',
    conclusion: 'success',
    path: '.github/workflows/desktop.yml',
    repository: { full_name: 'studio/repo' },
  };
  await writeFile(join(root, 'bad-run.json'), JSON.stringify(runMetadata));
  await writeFile(
    join(root, 'bad-jobs.json'),
    JSON.stringify({
      jobs: [{ name: 'desktop (windows-2022)', conclusion: 'success' }],
    }),
  );
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.runs.splice(1, 0, {
    kind: 'desktop',
    runId: '202',
    metadata: join(root, 'bad-run.json'),
    artifacts: badRoot,
    jobs: join(root, 'bad-jobs.json'),
  });
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0].status).toBe('passed');
  expect(summary.runs[1]).toMatchObject({
    status: 'failed',
    errors: ['artifact digest differs for app.bin'],
  });
  expect(await readFile(join(root, 'summary.md'), 'utf8')).toContain(
    'artifact digest differs for app.bin',
  );
});

test('keeps portable macOS packaging passed while notarization is blocked', async () => {
  const { root, manifest } = await fixture({ artifact: false });
  const dmg = 'portable dmg';
  await writeFile(join(root, 'desktop/portable.dmg'), dmg);
  await writeFile(
    join(root, 'desktop/macos-package-verification.json'),
    JSON.stringify({
      schema: 1,
      sourceSha,
      qualificationRunId: '99',
      applicationUnchangedBeforeSealing: true,
      runtimeChecks: [{ name: 'llama-server', isolatedHelpExitCode: 0 }],
      adHocSeal: {
        dmgPath: 'target/release/bundle/dmg/portable.dmg',
        dmgSha256: sha256(dmg),
        codesignStrictVerification: 'passed',
        dmgIntegrityVerification: 'passed',
      },
    }),
  );
  const metadataPath = join(root, 'desktop-run.json');
  const metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  metadata.path = '.github/workflows/macos-package.yml';
  await writeFile(metadataPath, JSON.stringify(metadata));
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.runs[0].kind = 'macos-signing';
  value.runs.pop();
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode).toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({
    status: 'blocked',
    blocker:
      'portable packaging passed without Developer ID notarization evidence',
    domain: { portablePackaging: 'passed', signing: 'unverified' },
  });
});

test('rejects an accelerator receipt that substitutes a weaker threshold', async () => {
  const { root, manifest } = await fixture({ artifact: false });
  const log = 'backend=wgpu max_delta=0.001 threshold=0.01';
  await writeFile(join(root, 'desktop/cpu-gpu-parity.log'), log);
  await writeFile(
    join(root, 'desktop/run-receipt.json'),
    JSON.stringify({
      schema: 1,
      headSha: sourceSha,
      workflowRunId: '101',
      runner: 'macOS/ARM64',
      backend: 'wgpu',
      maxDelta: 0.001,
      threshold: 0.01,
      logSha256: sha256(log),
    }),
  );
  const metadataPath = join(root, 'desktop-run.json');
  const metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  metadata.path = '.github/workflows/accelerator.yml';
  await writeFile(metadataPath, JSON.stringify(metadata));
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.runs[0].kind = 'accelerator';
  value.runs.pop();
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({
    status: 'failed',
    errors: [
      'accelerator max delta must be finite, nonnegative, and below 0.0001',
    ],
  });
});

test('rejects hash-valid text inference whose actual generated text is empty', async () => {
  const { root, manifest } = await fixture({ artifact: false });
  const resultJson = JSON.stringify({ text: '   ', model: 'qwen3-4b' });
  await writeFile(join(root, 'desktop/text.json'), resultJson);
  await writeFile(
    join(root, 'desktop/run-receipt.json'),
    JSON.stringify({
      schema: 1,
      headSha: sourceSha,
      workflowRunId: '101',
      runner: 'macOS/X64',
      mode: 'text',
      resultSha256: sha256(resultJson),
    }),
  );
  const value = JSON.parse(await readFile(manifest, 'utf8'));
  value.runs[0].kind = 'text-inference';
  value.runs.pop();
  await writeFile(manifest, JSON.stringify(value));
  const result = run(manifest);
  expect(result.exitCode).not.toBe(0);
  const summary = JSON.parse(
    await readFile(join(root, 'summary.json'), 'utf8'),
  );
  expect(summary.runs[0]).toMatchObject({
    status: 'failed',
    errors: ['text inference result contains no generated text'],
  });
});
