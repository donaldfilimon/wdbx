# B4 · Interactive specification diagrams

Phase B4 of the program in `2026-09-29` planning (Bun, component system, WASM,
live visualization). Depends on B1 (components) and B3 (live trace data).

## Goal

The specification reader shows its 29 fenced figures as first-class figures.
Six become interactive diagrams tied to the running specimen where that is
honest; the rest become labelled code figures. The authoritative Markdown stays
the only source: every diagram keeps the original figure text one click away.

## Data

`scripts/sync-specification.py` splits each chapter body at top-level fenced
blocks and emits `segments` in place of `html`:

```ts
type Segment =
  | { kind: 'html'; html: string }
  | { kind: 'figure'; id: string; lang: string; code: string };
```

Figure ids are `ch<N>-<k>` (1-based within the chapter), so the six diagram ids
are stable while prose around them changes. The script asserts 29 figures, and
that joining the segments' source reproduces the chapter body.

## Figures

| Id | Diagram | Live link |
|---|---|---|
| `ch2-1` | Architecture pipeline: stages, phase boundary, JIT fan-out/fan-in, idle band | Each stage links to the chapter that specifies it |
| `ch5-1`, `ch5-4`, `ch5-5` | Entity model explorer: one card per entity, field types that name another entity are links to its card | Counts beside `nodes`, `attachments`, `history` from the specimen |
| `ch11-1` | Cycle lifecycle: the kernel's six phases over the pseudocode | Highlights the phases the last or running cycle reached |
| `ch19-1` | Idle maintenance coin flip: heads to mutation, tails to PHAGY | Mutation and PHAGY event counts |
| `ch20-2` | Index Rafts planner: candidates split into checkpoint chunks, each split across rafts | Uses the specimen's `chunkSize`, `maxRafts`, `scanLimit`; N is adjustable |
| `ch24-1` | Save/restore flow: specimen, structural checks, migration, kernel validation, storage | Shows which storage this edition uses |

Everything else renders through `CodeFigure` (labelled, focusable, language
badge). Pseudocode and ASCII keep monospace and horizontal scroll.

The raft plan comes from the kernel, not a TypeScript copy:
`specimen_kernel::search::raft_plan(len, chunk, workers)` returns the chunk and
raft ranges exactly as `RaftCursor::advance` splits them (chunk capped at 4096,
raft size `ceil(chunk_len / workers)`), exposed as the WASM op `raftPlan`. A
specimen-core test pins it against `RaftCursor`'s partition.

## Reader restyle

The reader chrome (layout, chapter nav, meta line, pagination) moves to
Tailwind tokens and B1 components; the prose rules for generated HTML stay in
`studio.css` because they style Markdown output. Kept hooks: `.chapter-count`,
`.reference-content`, `nav[aria-label="Specification chapters"]`, the article.

## Accessibility

Each diagram is a `<figure>` with a `<figcaption>`; SVGs are `role="img"` with
`<title>`/`<desc>`, and every interactive element is a real link or button with
a 44px target. Each figure has "Specification text" disclosure with the source.
Diagrams use token colors only and respect reduced motion. Axe runs on a
diagram chapter in both themes.

## Testing

- Python: the sync script's own assertions.
- Rust: `raft_plan` unit tests (empty, exact multiple, remainder, workers >
  items, chunk > 4096) and the specimen-core parity test.
- Bun: `parseEntityOutline`, figure registry coverage (every id in the JSON
  that the registry names exists), `raftPlan` through the WASM facade,
  server-rendered diagrams (roles, names, links).
- e2e: open chapter 20, change N, see the raft count; open chapter 11 after a
  cycle and see the reached phases; axe state `spec-diagrams`.
