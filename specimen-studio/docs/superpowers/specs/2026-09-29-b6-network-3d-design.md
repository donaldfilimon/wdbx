# B6 · 3D network editor

Phase B6 of the 2026-09-29 program; the P5a/P5b editors from
`2026-09-28-specimen-studio-desktop-design.md` section 4, scoped to what the
kernel can guarantee.

## Constraints (from `specimen_kernel::neural::Network::validate`)

Version 1; 1 to 8 layers; the first layer reads 128 features; the last layer
writes 32 values (the composition decoder); each layer has 1 to 1,024
outputs and at most 131,072 sparse weights; CSR shape (`offsets`, `columns`,
`weights`, `biases`) must be consistent and finite.

## Kernel commands

`specimen_kernel::network_edit::apply(&Network, NetworkCommand) ->
Result<Network>`, deterministic for a seed, validated before returning
(`InvalidNetwork` on failure, original untouched):

| Command | Effect |
|---|---|
| `addLayer { at, outputs, fanIn, seed }` | Insert a hidden layer at `at` (never after the decoder); the layer it displaces is rewired to the new width, keeping its fan-in |
| `removeLayer { index }` | Remove a hidden layer; the next layer is rewired |
| `resizeLayer { index, outputs, seed }` | Change a hidden layer's width; it and the next layer are rewired |
| `setConnectivity { index, fanIn, seed }` | Rewire one layer with `fanIn` distinct inputs per neuron |
| `initWeights { seed }` | Resample every weight, keeping connectivity; biases reset to 0.01 |

`Network::trace(input, familiar, seed)` returns each layer's output on the
CPU with the same activation choices as `run`. WASM ops: `networkDefault`,
`networkEdit`, `networkTrace` (text is encoded with `neural::encode`).

## Persistence

Browser: IndexedDB key `network` beside the workspace; missing or invalid
means the default network. Desktop: the snapshot's `network`, committed with
the existing `network` op at the current revision.

## UI

View `network` ("Network", Build group), shadcn `Tabs`:

- Layers: 3D columns of neurons with sampled sparse connections, coloured
  by the last trace; a builder table (width, fan-in, weights, remove), add
  layer, reinitialise with a seed, and a trace form (text input) whose result
  is a table of per-layer statistics and the output vector.
- Topology: specimen nodes and attachments in 3D (type to shape, strength to
  size, tone to colour, affinity to width, hard to solid versus faint);
  selecting a node shows its details; paired nodes and attachments tables.

The canvas is lazy-loaded, `frameloop="demand"`, static under reduced motion,
named as one image that points at its table, and replaced by a notice when
WebGL is unavailable. Dragging and connect-by-drag are out of scope for B6.

## Testing

Rust unit tests per command (shape, determinism, validation failures, budget
errors, decoder protection) and for `trace` matching `run`; Bun facade
tests through WASM; server-rendered table tests; e2e adds a layer, traces a
prompt, and checks the tables; axe state `network-layers` and
`network-topology`.
