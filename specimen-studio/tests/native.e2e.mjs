import { browser, $, $$, expect } from '@wdio/globals';
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { deepStrictEqual } from 'node:assert';
const evidence = {
  schema: 1,
  headSha: process.env.GITHUB_SHA ?? null,
  runner: {
    os: process.env.RUNNER_OS ?? process.platform,
    arch: process.env.RUNNER_ARCH ?? process.arch,
  },
  scenarios: [],
};
describe('Native specimen desktop', () => {
  let fixtureDirectory;
  before(async () => {
    if (!process.env.WDBX_STUDIO_TEST_DATA)
      throw Error(
        'Native qualification requires an isolated WDBX_STUDIO_TEST_DATA directory',
      );
    await mkdir(resolve('work'), { recursive: true });
    fixtureDirectory = await mkdtemp(resolve('work/native-fixtures-'));
    await browser.saveScreenshot(resolve('work/native-first-screen.png'));
    console.log('Native page title', await browser.getTitle());
  });
  after(async () => {
    await writeFile(
      resolve('work/native-ui-evidence.json'),
      `${JSON.stringify(evidence, null, 2)}\n`,
    );
  });
  it('runs the Rust engine and renders a scoped response', async () => {
    await browser.waitUntil(
      async () => !(await $('body').getText()).includes('Opening workspace'),
      { timeout: 30000 },
    );
    await $(
      'nav[aria-label="Main navigation"] a[href="?view=studio"]',
    ).waitForDisplayed();
    await $('nav[aria-label="Main navigation"] a[href="?view=studio"]').click();
    await $('button=What is 2 + 2?').waitForDisplayed();
    await $('button=What is 2 + 2?').click();
    await expect($('.answer-text')).toHaveText('4');
    await $('button[aria-label="Run dossier"]').click();
    await expect($('.provenance-detail')).toHaveText(
      expect.stringContaining('Calculate'),
    );
    const right = $('button[aria-label^="Right "][aria-label$=" segment"]');
    await right.click();
    await expect(right).toBeDisabled();
    await browser.waitUntil(
      async () =>
        browser.tauri.execute(async () => {
          const snapshot = await window.__wdbxTestCall({ op: 'snapshot' });
          const cycle = snapshot.specimen.history.at(-1);
          return (
            cycle.feedback.length === 1 &&
            cycle.segments[0].contributors.length > 0
          );
        }),
      { timeout: 30000 },
    );
    evidence.scenarios.push('native-feedback-provenance-persistence');
    await $('input#prompt').setValue("don't calculate 2 + 2; say hello");
    await $('button=Run cycle').click();
    await browser.waitUntil(
      async () =>
        /\b(Hello|Hi|Hey|Greetings)\b/.test(
          (await $$('.answer-text').map((element) => element.getText())).join(
            ' ',
          ),
        ),
      { timeout: 30000 },
    );
    if (
      (await $$('.answer-text').map((element) => element.getText())).some(
        (text) => text.trim() === '4',
      )
    )
      throw Error('Negated arithmetic escaped its clause');
    evidence.scenarios.push('rust-engine-scoped-response');
  });
  it('rejects stale writes and malformed imports while restoring portable saves', async () => {
    const archive = resolve(fixtureDirectory, 'roundtrip.wdbxspecimen');
    const malformed = resolve(fixtureDirectory, 'malformed.json');
    await writeFile(malformed, '{broken');
    const result = await browser.tauri.execute(
      async (tauri, paths) => {
        const call = (request) => window.__wdbxTestCall(request);
        const original = await call({ op: 'snapshot' });
        await call({ op: 'export', path: paths.archive });
        const changed = structuredClone(original.specimen);
        changed.name = 'Native persistence qualification';
        const edited = await call({
          op: 'edit',
          revision: original.revision,
          specimen: changed,
        });
        let staleCode;
        try {
          await call({
            op: 'edit',
            revision: original.revision,
            specimen: original.specimen,
          });
        } catch (error) {
          staleCode = error.code;
        }
        const afterStale = await call({ op: 'snapshot' });
        let malformedCode;
        try {
          await call({
            op: 'import',
            revision: edited.revision,
            path: paths.malformed,
          });
        } catch (error) {
          malformedCode = error.code;
        }
        const afterMalformed = await call({ op: 'snapshot' });
        const restored = await call({
          op: 'import',
          revision: edited.revision,
          path: paths.archive,
        });
        return {
          staleCode,
          malformedCode,
          stalePreserved: JSON.stringify(afterStale) === JSON.stringify(edited),
          malformedPreserved:
            JSON.stringify(afterMalformed) === JSON.stringify(edited),
          originalSpecimen: original.specimen,
          revision: restored.revision,
          expectedRevision: edited.revision + 1,
          specimen: restored.specimen,
        };
      },
      { archive, malformed },
    );
    if (result.staleCode !== 'StaleRevision' || !result.stalePreserved)
      throw Error('Stale native edit replaced committed data');
    if (!result.malformedCode || !result.malformedPreserved)
      throw Error('Malformed import replaced committed data');
    deepStrictEqual(
      result.specimen,
      result.originalSpecimen,
      'Portable save changed the original specimen',
    );
    if (result.revision !== result.expectedRevision)
      throw Error(
        'Portable save did not restore the exact specimen at a new revision',
      );
    await browser.refresh();
    await browser.waitUntil(
      async () => !(await $('body').getText()).includes('Opening workspace'),
      { timeout: 30000 },
    );
    const afterReload = await browser.tauri.execute(async () =>
      window.__wdbxTestCall({ op: 'snapshot' }),
    );
    deepStrictEqual(
      afterReload.specimen,
      result.specimen,
      'Native bridge reload changed the restored specimen',
    );
    evidence.scenarios.push('durable-conflict-import-export-reload');
  });
  it('shows installed model metadata and performs native image analysis', async () => {
    await $('nav[aria-label="Main navigation"] a[href="?view=lab"]').click();
    await expect($('.native-lab')).toBeDisplayed();
    await expect($('.runtime-pills')).toHaveText(
      expect.stringContaining('Native Rust engine'),
    );
    const missing = await browser.tauri.execute(async () => {
      try {
        await window.__wdbxTestCall({
          op: 'generate',
          jobId: crypto.randomUUID(),
          modelId: 'qwen3-4b',
          prompt: 'hello',
          seed: 1,
        });
        return 'unexpected success';
      } catch (e) {
        return e.code;
      }
    });
    if (missing !== 'ModelMissing')
      throw Error('Missing model must produce a structured error: ' + missing);
    console.log('Native model workspace ready; missing model handled locally');
    const bytes = Array.from(
      await readFile(resolve('work/acceptance/ocr-fixture.png')),
    );
    const result = await browser.tauri.execute(async (tauri, bytes) => {
      return window.__wdbxTestCall({
        op: 'analyze',
        jobId: crypto.randomUUID(),
        bytes,
        focus: { x: 0, y: 0, width: 1, height: 1 },
      });
    }, bytes);
    if (!result.analysis.patternId.startsWith('visual-v2:'))
      throw Error('Missing native visual descriptor');
    if (result.analysis.sdf.values.length !== 4096)
      throw Error('Invalid signed distance field');
    console.log('Native analysis ready');
    const visualCycle = await browser.tauri.execute(async (tauri, result) => {
      const call = (request) => window.__wdbxTestCall(request);
      const current = await call({ op: 'snapshot' });
      const learned = await call({
        op: 'learnVisual',
        revision: current.revision,
        asset: result.asset,
        analysis: result.analysis,
        name: 'Native visual fixture',
        ocrCorrection: 'HELLO WORLD',
      });
      return call({
        op: 'runVisual',
        revision: learned.revision,
        jobId: crypto.randomUUID(),
        asset: result.asset,
        focus: result.analysis.focus,
      });
    }, result);
    if (visualCycle.cycle.votes.length !== 1)
      throw Error('Expected one learned visual vote');
    if (visualCycle.cycle.votes[0].evidence.similarity !== 100)
      throw Error('Visual score not preserved');
    if (visualCycle.cycle.imageEvidence.asset !== result.asset)
      throw Error('Missing image provenance');

    await browser.saveScreenshot(resolve('work/native-desktop.png'));
    evidence.scenarios.push('native-image-analysis');
  });
  it('reports the store, commits network edits at their revision, and shows the Lab tabs', async () => {
    const result = await browser.tauri.execute(async () => {
      const call = (request) => window.__wdbxTestCall(request);
      const info = await call({ op: 'storeInfo' });
      const snapshot = await call({ op: 'snapshot' });
      const saved = await call({
        op: 'network',
        revision: snapshot.revision,
        network: snapshot.network,
      });
      let stale = 'accepted';
      try {
        await call({ op: 'network', revision: snapshot.revision, network: snapshot.network });
      } catch (e) {
        stale = e.code;
      }
      return { info, before: snapshot.revision, after: saved.revision, stale };
    });
    if (typeof result.info.revision !== 'number' || !result.info.auditDag.ok)
      throw Error('storeInfo must report a revision and a verified audit DAG: ' + JSON.stringify(result.info));
    if (result.info.snapshotKey && result.info.snapshotKey.key !== 'studio/snapshot')
      throw Error('storeInfo must describe the studio snapshot key');
    if (result.after !== result.before + 1) throw Error('network edit must advance the revision');
    if (result.stale !== 'StaleRevision')
      throw Error('a network edit at an old revision must be StaleRevision, got ' + result.stale);
    await $('nav[aria-label="Main navigation"] a[href="?view=store"]').click();
    await expect($('main')).toHaveText(expect.stringContaining('Desktop · WDBX v2 journal'));
    await $('nav[aria-label="Main navigation"] a[href="?view=lab"]').click();
    for (const tab of ['Image', 'Generate', 'Models'])
      await expect($(`button=${tab}`)).toBeDisplayed();
    await $('button=Models').click();
    await expect($('.model-card')).toBeDisplayed();
    evidence.scenarios.push('store-info-network-revision-lab-tabs');
  });
  it('searches a virtualized 100,000-record history', async () => {
    // Embedded direct-eval has its own short timeout. Poll the asynchronous
    // native commit so a large fixture does not outlive a single evaluation.
    await browser.tauri.execute(() => {
      window.__nativeScaleResult = { pending: true };
      void (async () => {
        const call = (request) => window.__wdbxTestCall(request);
        const snapshot = await call({ op: 'snapshot' });
        const specimen = snapshot.specimen;
        specimen.settings.historyLimit = 100000;
        specimen.history = Array.from({ length: 100000 }, (_, i) => ({
          id: 'scale-' + i,
          input: 'Scale record ' + i,
          createdAt: '2026-09-04T12:00:00Z',
          segments: [
            { id: 'segment-' + i, text: 'Evidence ' + i, contributors: [] },
          ],
          votes: [],
          trace: [],
          feedback: [],
          status: 'matched',
          pinned: false,
        }));
        await call({ op: 'edit', revision: snapshot.revision, specimen });
        return specimen.history.length;
      })().then(
        (count) => {
          window.__nativeScaleResult = { count };
        },
        (error) => {
          window.__nativeScaleResult = { error: String(error) };
        },
      );
      return true;
    });
    await browser.waitUntil(
      async () =>
        browser.tauri.execute(() => !window.__nativeScaleResult.pending),
      { timeout: 90000, interval: 500 },
    );
    const result = await browser.tauri.execute(
      () => window.__nativeScaleResult,
    );
    if (result.error) throw Error(result.error);
    if (result.count !== 100000) throw Error('Scale fixture incomplete');
    await browser.refresh();
    await browser.waitUntil(
      async () => !(await $('body').getText()).includes('Opening workspace'),
      { timeout: 30000 },
    );
    await $('nav[aria-label="Main navigation"] a[href="?view=memory"]').click();
    await $('button=Conversation').click();
    try {
      await $('.history-record').waitForDisplayed({ timeout: 30000 });
    } catch (error) {
      console.log(
        'History qualification failure',
        await browser.getUrl(),
        await $('body').getText(),
      );
      await browser.saveScreenshot(resolve('work/native-history-failure.png'));
      throw error;
    }
    const rendered = await $$('.history-record');
    if (rendered.length < 1 || rendered.length > 30)
      throw Error('History virtualization failed: ' + rendered.length);
    const start = Date.now();
    await $('input[placeholder="Search your memory…"]').setValue(
      'Scale record 43210',
    );
    await expect($('.history-heading')).toHaveText(
      expect.stringContaining('Scale record 43210'),
    );
    const elapsed = Date.now() - start;
    console.log(
      '100,000-record history search rendered in ' +
        elapsed +
        ' ms; initial DOM rows ' +
        rendered.length,
    );
    if (elapsed > 5000)
      throw Error('History search exceeded five-second acceptance budget');
    evidence.historySearchMilliseconds = elapsed;
    evidence.scenarios.push('virtualized-history-search');
  });
});
