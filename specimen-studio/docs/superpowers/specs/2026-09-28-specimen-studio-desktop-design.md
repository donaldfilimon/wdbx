# Specimen Studio desktop program: design

Date: 2026-09-28. Status: design approved in conversation through section 2;
sections 3-5 written for review. Scope: `wdbx/specimen-studio/` only.

## Intent

Donald wants Specimen Studio to feel like a standalone desktop product
(Tauri 2, Rust, bun, TSX, React 19) with a modern Next.js-style shell,
open-source styling, Obsidian-equivalent knowledge features, keyboard and
terminal control of its WDBX store, and in-app construction and customization
of neural networks in 3D.

Decisions taken (Donald, 2026-09-28):

- It stays at `wdbx/specimen-studio/`. The same-day fold into wdbx is not
  reversed; `donaldfilimon/wdbx-specimen-studio` stays archived.
- "TUI controls" means both an in-app keyboard console and a real terminal
  binary, sharing one command layer.
- "3D neural network" means both the specimen topology (nodes and
  attachments) as an editable 3D graph, and a layer builder over the existing
  `neural::Network`. Topology first.
- Full parity: every feature works in the browser (Worker) edition and the
  desktop edition.
- Parity is achieved by one Rust kernel compiled to WebAssembly for the
  browser (approach B), not by maintaining TypeScript and Rust twins.

Assumptions (correct these if wrong):

- The existing white/teal visual identity remains the default theme.
- WDBX ownership rules hold: durable semantics live in the store; the studio
  never invents a second canonical record.
- Vision and model tooling (`models.rs`, `vision.rs`) remain native-only, as
  today. This is the single named parity exception.

## Program decomposition

Each phase gets its own implementation plan and ends with both gates green.

| Phase | Deliverable | Depends on |
|---|---|---|
| P0 | Kernel extraction and WASM feasibility (sections 1, 5) | none |
| P1 | Command layer, text grammar, browser engine on WASM (section 2) | P0 |
| P2 | App shell and UX foundation (section 4) | P1 |
| P3 | In-app console and terminal TUI (section 2) | P1, P2 |
| P4 | Knowledge layer: notes, links, backlinks, graph (section 3) | P1, P2 |
| P5a | 3D topology editor (section 4) | P2, P4 graph layout |
| P5b | 3D layer builder over `neural::Network` (section 4) | P5a |

If the P0 feasibility spike fails (the kernel cannot build for
`wasm32-unknown-unknown` with acceptable size), the program falls back to
approach A: TypeScript and Rust twins held together by the shared
conformance corpus from section 5. Nothing after P0 is started until that
verdict is recorded.

## Section 1: crate architecture

Today `native/specimen-core` is one crate whose dependencies include
`abi-wdbx`, blocking `reqwest`, `sysinfo`, `ocrs`/`rten`, `wgpu`/`pollster`,
`zip` and `image`, none of which belong in a browser build. It splits into
three crates inside the existing `specimen-studio` Cargo workspace:

- `native/specimen-kernel`: pure rules, no I/O. Takes `engine`, `language`,
  the CPU path of `neural`, `protocol`, and every new module (commands,
  notes, links, graph index, layout, topology and layer-builder operations).
  Dependencies limited to `serde`, `serde_json`, `thiserror`, `sha2`,
  `regex`, `pulldown-cmark`. Builds for the host and
  `wasm32-unknown-unknown`.
- `native/specimen-core` (kept name, now the native host): `persistence`
  over `abi-wdbx`, `models`, `vision`, `scheduler`, the `wgpu` path of
  `neural`, `.wdbxspecimen` export and import. Implements the kernel's
  `Store` trait over WDBX. Tauri and `specimen-tui` depend on it.
- `native/specimen-wasm`: `wasm-bindgen` wrapper exposing the command layer
  to TypeScript, with a `Store` over IndexedDB. Replaces
  `lib/specimen/engine.ts` once conformance passes.

Keeping the `specimen-core` name avoids churning `src-tauri`, `package.json`
scripts, CI workflow paths and the AGENTS.md gate text in P0.

Injected seams: `lib.rs` calls `chrono::Utc::now()` and `uuid` v4 directly.
The kernel instead receives an `Env { clock: &dyn Clock, ids: &mut dyn
IdSource }` from its host. Native hosts supply wall clock and v4 IDs; the
WASM host supplies `Date.now()` and `crypto.randomUUID()`; tests supply a
fixed clock and seeded IDs so every conformance case is byte-deterministic.

GPU: `neural::Network::run` already falls back from `wgpu` to CPU and
reports `backend`. The kernel is CPU-only; the native host adds the GPU path.
Outputs must match within the tolerance already implied by the fallback
(conformance compares CPU results exactly; GPU results are checked only on
native, against CPU, with an explicit epsilon).

The `abi-wdbx` git pin (`3ac03f0`) does not move as part of this program
unless a phase needs a newer store API; that would be its own recorded
decision.

## Section 2: command layer and data flow

The kernel defines serde-tagged `Command` and `Query` enums and:

```rust
fn apply(state: &Workspace, cmd: Command, env: &mut Env) -> Result<Applied>;
struct Applied { state: Workspace, events: Vec<Event>, revision: u64 }
fn query(state: &Workspace, q: Query) -> Result<Answer>;
fn parse(line: &str) -> Result<Parsed>; // Parsed::Command | Parsed::Query
```

Every write from every surface (buttons, 3D drag, note edits, console, TUI)
is a `Command`. Consequences: the activity journal is the event stream;
undo and redo are inverse commands recorded in `Applied`; every feature is
keyboard-reachable. The composer's slash commands (`/prompt`, `/right`,
`/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`,
`/addPattern`, `/learn`) move from `app/studio.tsx` into `parse`, keeping
their current spellings, including the `/wrng` alias. `/help` text is
generated from the enum.

Concurrency: commands carry `expected_revision`, preserving the optimistic
check in `lib/specimen/native.ts`. A stale revision returns `Conflict`; the
UI shows it and offers reload. Nothing overwrites silently.

Terminal TUI: new binary crate `native/specimen-tui` (ratatui) over
`specimen-core`. It opens the same app-data store. `abi-wdbx`'s
`DurableStore` holds `<base>.writer.lock` for its lifetime, so when the
desktop app is running the TUI receives `WriterBusy`, opens read-only,
shows a status-bar banner, and refuses write commands with `ReadOnly`.
Driving a running app from the terminal is out of scope.

Errors keep `Error { code, message }`. New codes: `Conflict`, `ReadOnly`,
`UnknownCommand`, `ParseError`, `NativeOnly`, `NotFound`, `InvalidNetwork`.
Every surface displays code and message verbatim.

## Section 3: knowledge layer (Obsidian-equivalent)

Data (part of `Workspace`, persisted through the same `Store`):

```rust
struct Note { id, title, body: String /* Markdown */, tags: Vec<String>,
              created_at, updated_at }
```

Links are parsed from bodies, never stored separately, so they cannot
disagree with the text:

- `[[Title]]` and `[[Title|label]]` link notes;
- `[[node:REF]]` links a specimen node, `[[pattern:ID]]` a pattern,
  `[[memory:REF]]` a memory resource. This joins the knowledge layer to the
  network: a node's inspector shows the notes that mention it.

Kernel queries: `Backlinks(target)`, `OutgoingLinks(note)`,
`UnresolvedLinks`, `Search(text | regex, scope)`, `Graph { focus, depth }`.
The backlink index is rebuilt incrementally from the events of each command,
and fully on load; a debug assertion in tests compares both.

Commands: `CreateNote`, `EditNote`, `RenameNote` (rewrites every link to the
old title in the same command, so rename is atomic and undoable),
`DeleteNote` (refused while backlinks exist unless `force`), `TagNote`.

Rendering: Markdown is rendered by `pulldown-cmark` inside the kernel with raw
HTML events dropped, so both editions render identically and no untrusted HTML
reaches the DOM. Links render as internal anchors; unresolved links are
styled distinctly and offer "create note".

Graph view: one force-directed layout computed in the kernel with a seeded
RNG (`neural::Rng` already exists), so positions are deterministic and
testable. The same layout feeds the 2D graph view (P4) and the initial
positions of the 3D topology editor (P5a).

Save format: `Workspace` gains `notes` and per-node `position`. The save
version increments; loading an older save migrates with empty notes and
unpinned positions, and the original is retained as a recovery copy, as
native import already does. `.wdbxspecimen` export includes notes.

Out of scope: daily notes, templates, plugins, canvas, sync.

## Section 4: shell, 3D editors and styling

Shell (P2): `app/studio.tsx` (3,238 lines) is split into
`app/shell/` (layout, sidebar, tabs, status bar) and `app/panels/` (one
module per panel: Studio, Network 3D, Layers, Notes, Graph, Memory,
Specification, Models). Layout: left sidebar of panels, tabbed center,
right inspector, bottom console dock toggled with the backtick key, command palette on
Cmd/Ctrl-K (existing `cmdk`), resizable panes (existing
`react-resizable-panels`). Styling stays on the existing shadcn and
Base UI components and Tailwind 4 tokens, with light and dark themes; the
white/teal palette is the default. `prefers-reduced-motion` disables camera
animation and auto-rotation.

3D rendering: `three` with `@react-three/fiber`. One scene component serves
both editors. The 3D canvas is not accessible on its own, so every 3D view
has a synchronized table view of the same data, and all edits are also
available as commands.

P5a topology editor. Encodings: node `type` to shape, `strength` to size,
`tone` to color, attachment `affinity` to edge width, `hard` to solid versus
dashed, `bidirectional` to arrowheads on both ends. Interactions: select
(inspector opens), drag (issues `MoveNode`, pins position), connect by
dragging from a node handle (`Connect`), delete, add node at cursor.
Commands: `AddNode`, `UpdateNode`, `MoveNode`, `UnpinNode`, `Connect`,
`UpdateAttachment`, `Disconnect`.

P5b layer builder over `neural::Network { version, layers: Vec<Layer> }`,
where `Layer` has `inputs`, `outputs`, `offsets`, `columns`, `weights`,
`biases` (sparse). Commands: `AddLayer`, `RemoveLayer`, `ResizeLayer`,
`SetConnectivity`, `InitWeights { seed }`, `RunNetwork { input }`. Every
network command runs `Network::validate()` before commit and returns
`InvalidNetwork` on failure. The view shows layers as columns of neurons
with sparse connections; `RunNetwork` animates activations (static under
reduced motion) and reports which backend produced the result.

## Section 5: testing, parity and gates

Conformance corpus: `specimen-studio/conformance/` holds cases
`{ initial workspace, env seed, commands, expected events, expected state
digest, expected query answers }`. The kernel runs them natively in
`cargo test`; the WASM build runs the same files under `bun test`. The
existing `native/specimen-core/tests/fixtures/starter.json` becomes the
first case. Before `engine.ts` is retired, the corpus also runs against it,
and any disagreement is recorded as a finding, not silently resolved.

Store parity: a round-trip test exports `.wdbxspecimen` from each edition
and imports it into the other, comparing state digests.

Kernel tests: unit tests per module; property tests for `parse` (round-trip
of `Display`) and for `RenameNote` (no link to the old title survives).

UI: existing `tests/ui-contract.test.js`, `browser.e2e.mjs` and
`accessibility.e2e.mjs` extend to the new panels; axe must stay clean,
including the table views that stand in for the 3D canvas.

TUI: snapshot tests of rendered frames using ratatui's test backend, plus an
integration test that a second process gets `ReadOnly` while a writer holds
the lock.

Gates, each phase: `bun run check` and `cargo test --workspace --locked`
from `specimen-studio/`, plus `bun run check:native` (Clippy, warnings
denied). P0 adds `cargo build -p specimen-kernel --target
wasm32-unknown-unknown` to the gate. Reports name both halves; one green
half is not a green phase.

P0 feasibility verdict is recorded in `VALIDATION.md` with the measured
WASM size (raw and gzip) and build time.
