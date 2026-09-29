/**
 * Pure derivations behind the Network view: table rows, 3D positions,
 * sampled connections and trace statistics. The 3D scene and the tables both
 * read from these, so the canvas never shows data the tables do not.
 */
import type { Network, NetworkLayer } from './kernel';

export interface LayerRow {
  index: number;
  inputs: number;
  outputs: number;
  fanIn: number;
  weights: number;
  /** The last layer: fixed at 32 outputs for the composition decoder. */
  decoder: boolean;
}

export function layerRows(net: Network): LayerRow[] {
  return net.layers.map((l, index) => ({
    index,
    inputs: l.inputs,
    outputs: l.outputs,
    fanIn: l.outputs ? Math.round(l.weights.length / l.outputs) : 0,
    weights: l.weights.length,
    decoder: index === net.layers.length - 1,
  }));
}

type Point = [number, number, number];

/** Neurons drawn per column; wider layers show their first `cap`. */
function column(count: number, x: number, cap: number): Point[] {
  const shown = Math.min(count, cap);
  const side = Math.ceil(Math.sqrt(shown));
  const gap = 0.35;
  return Array.from({ length: shown }, (_, i) => [
    x,
    ((i % side) - (side - 1) / 2) * gap,
    (Math.floor(i / side) - (side - 1) / 2) * gap,
  ]);
}

/** One column for the 128 inputs, then one per layer, left to right. */
export function neuronPositions(net: Network, cap: number): Point[][] {
  const widths = [
    net.layers[0]?.inputs ?? 0,
    ...net.layers.map((l) => l.outputs),
  ];
  const spacing = 3;
  const start = (-(widths.length - 1) * spacing) / 2;
  return widths.map((w, i) => column(w, start + i * spacing, cap));
}

/**
 * Up to `max` connections of a layer as [input, output] pairs, keeping only
 * neurons that are drawn (index below `cap`), sampled evenly across rows.
 */
export function sampledEdges(
  layer: NetworkLayer,
  cap: number,
  max: number,
): [number, number][] {
  const all: [number, number][] = [];
  for (let row = 0; row < Math.min(layer.outputs, cap); row++)
    for (let k = layer.offsets[row]; k < layer.offsets[row + 1]; k++)
      if (layer.columns[k] < cap) all.push([layer.columns[k], row]);
  if (all.length <= max) return all;
  const step = all.length / max;
  return Array.from({ length: max }, (_, i) => all[Math.floor(i * step)]);
}

export interface TraceStat {
  layer: number;
  min: number;
  mean: number;
  max: number;
  /** Neurons with a positive output. */
  active: number;
}

export function traceStats(layers: number[][]): TraceStat[] {
  return layers.map((values, layer) => ({
    layer,
    min: Math.min(...values),
    mean: values.reduce((a, b) => a + b, 0) / values.length,
    max: Math.max(...values),
    active: values.filter((v) => v > 0).length,
  }));
}

/** Specimen nodes on a Fibonacci sphere of radius 3, in list order. */
export function topologyLayout(
  nodes: { ref: string }[],
): Record<string, Point> {
  const n = nodes.length;
  const golden = Math.PI * (3 - Math.sqrt(5));
  return Object.fromEntries(
    nodes.map((node, i) => {
      if (n === 1) return [node.ref, [0, 0, 0] as Point];
      const y = 1 - (2 * i) / (n - 1);
      const r = Math.sqrt(1 - y * y);
      const t = golden * i;
      return [
        node.ref,
        [3 * r * Math.cos(t), 3 * y, 3 * r * Math.sin(t)] as Point,
      ];
    }),
  );
}
