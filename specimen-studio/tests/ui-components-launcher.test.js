// Probe the actual component e2e entry point with owned launcher fixtures.
// Bun builds the real component fixture; each launcher records admission and
// stops before a browser or production route is opened.
import { expect, test } from 'bun:test';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const entry = resolve(import.meta.dir, 'ui-components.e2e.mjs');
const studio = resolve(import.meta.dir, '..');
const stop = 'STUDIO_LAUNCH_PROBE_STOP';
const driver = `
import { mock, test } from 'bun:test';
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const launcher = (engine) => ({
  async launch(options) {
    writeFileSync(process.env.STUDIO_LAUNCH_RECORD, JSON.stringify({engine, options}));
    throw new Error('STUDIO_LAUNCH_PROBE_STOP');
  },
});
mock.module('@playwright/test', () => ({
  chromium: launcher('chrome'),
  firefox: launcher('firefox'),
  webkit: launcher('webkit'),
  expect: () => { throw new Error('No page assertion is admitted by this launcher probe'); },
}));
test('observe actual launcher admission', async () => {
  await import(pathToFileURL(process.env.STUDIO_LAUNCH_ENTRY).href);
});
`;

async function probe(engine) {
  const root = await mkdtemp(join(tmpdir(), 'studio-launch-probe-'));
  let child;
  let timer;
  try {
    const script = join(root, 'driver.test.mjs');
    const record = join(root, 'launcher.json');
    await writeFile(script, driver);
    const env = {
      ...process.env,
      STUDIO_LAUNCH_RECORD: record,
      STUDIO_LAUNCH_ENTRY: entry,
    };
    if (engine === undefined) delete env.STUDIO_BROWSER;
    else env.STUDIO_BROWSER = engine;
    child = Bun.spawn([process.execPath, 'test', script], {
      cwd: studio,
      env,
      stdin: 'ignore',
      stdout: 'pipe',
      stderr: 'pipe',
    });
    // This owns only the test child. Await its exit before removing scratch data.
    timer = setTimeout(() => child.kill('SIGKILL'), 20_000);
    const [exitCode, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    clearTimeout(timer);
    timer = undefined;
    let launch = null;
    try {
      launch = JSON.parse(await readFile(record, 'utf8'));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    return { exitCode, stdout, stderr, launch };
  } finally {
    if (timer !== undefined) clearTimeout(timer);
    if (child && child.exitCode === null) {
      child.kill('SIGKILL');
      await child.exited;
    }
    await rm(root, { recursive: true, force: true });
  }
}

for (const engine of ['chrome', 'firefox', 'webkit']) {
  test(`component e2e admits the requested ${engine} launcher`, async () => {
    const result = await probe(engine);
    // A fixture build/import failure is not an attributable launcher failure.
    expect(result.stderr).toContain(stop);
    expect(result.exitCode).not.toBe(0);
    expect(result.launch).toEqual({
      engine,
      options:
        engine === 'chrome'
          ? { channel: 'chrome', headless: true }
          : { headless: true },
    });
  }, 30_000);
}

test('component e2e defaults to the installed Chrome launcher', async () => {
  const result = await probe(undefined);
  expect(result.stderr).toContain(stop);
  expect(result.launch).toEqual({
    engine: 'chrome',
    options: { channel: 'chrome', headless: true },
  });
}, 30_000);

test('component e2e refuses an unsupported engine before any launch', async () => {
  const result = await probe('unsupported-engine');
  expect(result.exitCode).not.toBe(0);
  expect(result.launch).toBeNull();
  expect(result.stderr).toContain(
    'Unsupported STUDIO_BROWSER: unsupported-engine',
  );
  expect(result.stderr).not.toContain(stop);
}, 30_000);
