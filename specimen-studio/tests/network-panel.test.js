import { beforeAll, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { LayersView, TopologyView } from '../app/network/network-panel';
import {
  initKernel,
  networkDefault,
  seedSpecimen,
  useKernelWorker,
} from '../lib/specimen/kernel';

beforeAll(async () => {
  useKernelWorker(false);
  await initKernel(
    readFileSync(
      new URL('../lib/specimen/wasm/specimen_wasm.wasm', import.meta.url),
    ),
  );
});

test('the layer builder lists every layer and protects the decoder', () => {
  const out = renderToStaticMarkup(
    h(LayersView, {
      network: networkDefault(),
      onCommand: () => {},
      busy: false,
    }),
  );
  expect(out).toContain('aria-label="Layer 1 neurons"');
  expect(out).toContain('Decoder');
  expect(out).toContain('Remove layer 1');
  expect(out).not.toContain('Remove layer 2');
  // The 3D scene is one named image that points at the table.
  expect(out).toMatch(
    /role="img" aria-label="Network layers in 3D[^"]*The Layers table lists every layer\."/,
  );
  // Server render has no WebGL: the notice stands in for the canvas.
  expect(out).toContain('3D view unavailable in this browser');
  // The trace says what it leaves out rather than claiming engine parity.
  expect(out).toContain('text-only encoding');
  expect(out).not.toContain('as the engine does');
});

test('the topology tab pairs the 3D view with node and attachment tables', () => {
  const specimen = seedSpecimen();
  const out = renderToStaticMarkup(h(TopologyView, { specimen }));
  for (const n of specimen.nodes) expect(out).toContain(n.name);
  expect(out).toContain('aria-pressed="false"');
  expect(out).toMatch(/role="img" aria-label="Specimen topology in 3D/);
  const empty = renderToStaticMarkup(
    h(TopologyView, { specimen: { ...specimen, nodes: [], attachments: [] } }),
  );
  expect(empty).toContain('No nodes');
});
