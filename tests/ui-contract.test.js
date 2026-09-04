import { readFile } from 'node:fs/promises';
import { test, expect } from 'bun:test';

const studio = await readFile(
  new URL('../app/studio.tsx', import.meta.url),
  'utf8',
);
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
});

test('instrument console retains responsive access to inspector detail', () => {
  expect(styles).toContain('.console-telemetry');
  expect(styles).toContain('.workflow-step');
  expect(styles).not.toContain(
    '.inspector-section:nth-last-child(2) {\n    display: none;',
  );
});
