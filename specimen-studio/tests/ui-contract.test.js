import { readFile, readdir } from 'node:fs/promises';
import { test, expect } from 'bun:test';

// The studio markup spans app/studio.tsx, app/panels, app/dialogs and app/shell.
const appDir = new URL('../app/', import.meta.url);
const sources = (await readdir(appDir, { recursive: true })).filter((f) =>
  f.endsWith('.tsx'),
);
const studio = (
  await Promise.all(sources.map((f) => readFile(new URL(f, appDir), 'utf8')))
).join('\n');
const styles = await readFile(
  new URL('../app/studio.css', import.meta.url),
  'utf8',
);

test('instrument console exposes a clear workflow and live telemetry', () => {
  expect(studio).toContain('className="console-telemetry"');
  expect(studio).toContain('aria-label="Specimen workflow"');
  expect(studio).toContain('aria-busy={busy}');
  expect(studio).toContain('Teach pattern');
  expect(studio).toContain('Run dossier');
  expect(studio).toContain('Topology zoom');
  expect(studio).toContain('Zoom in topology');
  expect(studio).toContain("'Deep scan'");
  expect(studio).toContain("'Enrich'");
});

test('instrument console retains responsive access to inspector detail', () => {
  expect(styles).toContain('.console-telemetry');
  expect(styles).toContain('.workflow-step');
  expect(styles).toContain('.topology-stage');
  expect(styles).toContain('repeat(6, minmax(0, 1fr))');
  expect(styles).not.toContain(
    '.inspector-section:nth-last-child(2) {\n    display: none;',
  );
});

test('the studio panel owns the workflow and telemetry markup', async () => {
  const panel = await readFile(
    new URL('../app/panels/studio-panel.tsx', import.meta.url),
    'utf8',
  );
  expect(panel).toContain('className="console-telemetry"');
  expect(panel).toContain('aria-label="Specimen workflow"');
  expect(panel).toContain('aria-busy={busy}');
});

test('the shell exposes its landmarks', () => {
  expect(studio).toContain('aria-label="Search or run a command"');
  expect(studio).toContain('Side pane: ${title}');
  expect(studio).toContain('aria-label="Activity dock"');
  expect(studio).toContain("'Collapse sidebar'");
  expect(studio).toContain('<DialogTitle className="sr-only">Command palette');
});
