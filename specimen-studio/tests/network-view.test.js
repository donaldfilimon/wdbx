import { expect, test } from 'bun:test';
import {
  describeRewiring,
  layerRows,
  neuronPositions,
  sampledEdges,
  topologyLayout,
  traceStats,
} from '../lib/specimen/network-view';

const layer = (inputs, outputs, fan) => ({
  inputs,
  outputs,
  offsets: Array.from({ length: outputs + 1 }, (_, i) => i * fan),
  columns: Array.from({ length: outputs * fan }, (_, i) => i % inputs),
  weights: Array.from({ length: outputs * fan }, (_, i) =>
    i % 2 ? 0.2 : -0.1,
  ),
  biases: Array(outputs).fill(0.01),
});
const net = { version: 1, layers: [layer(128, 64, 8), layer(64, 32, 8)] };

test('layerRows describe each layer and mark the decoder', () => {
  expect(layerRows(net)).toEqual([
    {
      index: 0,
      inputs: 128,
      outputs: 64,
      fanIn: 8,
      weights: 512,
      decoder: false,
    },
    {
      index: 1,
      inputs: 64,
      outputs: 32,
      fanIn: 8,
      weights: 256,
      decoder: true,
    },
  ]);
});

test('neuronPositions places an input column and one column per layer', () => {
  const cols = neuronPositions(net, 64);
  expect(cols.map((c) => c.length)).toEqual([64, 64, 32]); // inputs capped at 64
  expect(new Set(cols[0].map((p) => p[0])).size).toBe(1);
  expect(cols[0][0][0]).toBeLessThan(cols[1][0][0]);
});

test('sampledEdges caps the edge count and maps to displayed neurons', () => {
  const edges = sampledEdges(net.layers[0], 64, 100);
  expect(edges.length).toBeLessThanOrEqual(100);
  expect(edges.every(([from, to]) => from < 64 && to < 64)).toBe(true);
  expect(sampledEdges(net.layers[1], 64, 1000).length).toBe(256);
});

test('traceStats summarizes each layer output', () => {
  expect(traceStats([[0, 1, 3], [0.5]])).toEqual([
    { layer: 0, min: 0, mean: 4 / 3, max: 3, active: 2 },
    { layer: 1, min: 0.5, mean: 0.5, max: 0.5, active: 1 },
  ]);
  expect(traceStats([])).toEqual([]);
});

test('topologyLayout is deterministic and spreads nodes on a sphere', () => {
  const nodes = ['a', 'b', 'c', 'd'].map((ref) => ({ ref }));
  const a = topologyLayout(nodes);
  expect(topologyLayout(nodes)).toEqual(a);
  expect(Object.keys(a)).toEqual(['a', 'b', 'c', 'd']);
  for (const [x, y, z] of Object.values(a))
    expect(Math.hypot(x, y, z)).toBeCloseTo(3, 5);
  expect(topologyLayout([{ ref: 'solo' }]).solo).toEqual([0, 0, 0]);
});

test('describeRewiring names layers an edit rewired as a side effect', () => {
  const after = {
    version: 1,
    layers: [layer(128, 4, 8), layer(4, 32, 4)],
  };
  // Layer 1 was the edit's target; layer 2 lost inputs because of it.
  expect(describeRewiring(net, after, [0])).toBe(
    'Layer 2 was rewired to its new inputs: 4 per neuron (was 8), new random weights.',
  );
  expect(describeRewiring(net, net, [0])).toBe('');
  // Adding a layer shifts indexes; the displaced layer is still reported.
  const added = {
    version: 1,
    layers: [layer(128, 64, 8), layer(64, 48, 6), layer(48, 32, 8)],
  };
  expect(describeRewiring(net, added, [1])).toBe(
    'Layer 3 was rewired to its new inputs: 8 per neuron, new random weights.',
  );
});
