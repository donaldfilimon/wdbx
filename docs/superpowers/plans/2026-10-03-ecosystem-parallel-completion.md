# Ecosystem Parallel Completion Implementation Plan

> For agentic workers: use `superpowers:subagent-driven-development` for owned
> implementation slices and `superpowers:executing-plans` for coordinator work.
> Independent repository work proceeds concurrently. Steps use checkboxes.

**Goal:** Complete WDBX, ABI, Abbey, Abbey Bot, MLAI/Quesar and WebPress's
release requirements, including the WebPress refactor, measured demonstrations,
full brand assets, and rendered animated trailers.

**Architecture:** Preserve the existing canonical repositories and public
contracts. WDBX owns durable episodic semantics; ABI and Abbey consume sibling
crates, while Abbey Bot retains its independent contract implementations.
Qualify independent changes concurrently, then verify dependent integration.

**Tech stack:** Repository-pinned Rust, Zig, Bun/TypeScript, React, native Apple
tooling, and existing animation/rendering tools. Each repository's instructions
and executable gate take precedence over copied commands.

**Spec:** Donald's approved in-thread ecosystem scope and immediate multi-repo,
multi-agent direction on 2026-10-03, together with each repository's approved
designs. This plan does not turn reference subsystems into production claims.

## Global constraints

- Four agent slots total, including the coordinator. Delegation must remain
  within that limit. Assign disjoint file ownership and preserve others' work.
- Work in canonical checkouts; do not switch a shared checkout's branch.
- Do not commit, push, publish, deploy, or restart live services without the
  corresponding authorization. Completion of local work does not imply rollout.
- Use scratch data for tests. Never use private live stores as test fixtures.
- No new production dependency without approval. Use relevant installed skills;
  discover additional skills only for a concrete capability gap.
- Keep WDBX's frozen v2 and v3 commitment contracts compatible. Specimen Studio
  remains a separate workspace with a pinned git dependency.
- Keep heavy gates resource-bounded. Do not run Abbey's Rust and Zig gates
  simultaneously or build against actively changing shared dependency files.
- Existing dirty work is a baseline, not this team's accomplishment.
- No calendar delay is built into this plan. Start available independent work
  immediately; synchronize on tests and interfaces, not estimated weeks.

## Review focus

1. Recovered state after interrupted writes must remain valid and attributable.
2. Consent, deletion, quarantine, and contradictory evidence must retain their
   documented meaning across consumers, persistence and exported projections.
3. Cancellation, unavailable providers, and partial output must terminate or
   recover explicitly without silent success.
4. Saved projects, accepted proposals, undo and publication must preserve
   identity and revision authority across reopen and export.
5. Public numerical and visual claims must match reproducible evidence, while
   private data and restricted assets remain excluded from launch artifacts.

## Task 1: Establish and preserve current state

- [ ] Record each repository's HEAD, branch, dirty paths, active owners, current
      spec/plan/ledger and authoritative gate.
- [ ] Map every explicit release requirement to source, test and runtime
      acceptance evidence; classify Current, Partial, Proposed or Blocked.
- [ ] Verify MLAI's recorded integration into Quesar; avoid duplicating a
      retired standalone application.
- [ ] Select concrete independent slices; record ownership and test commands
      before editing. Inspect current files rather than trusting old task ticks.

## Task 2: WDBX and Specimen Studio

**Owner:** coordinator initially. **Interfaces:** existing canonical episode
encoder/store/receipts and unchanged consumer contracts.

- [x] Qualify the current cross-language test changes and run
      `bash tools/check.sh` on the pinned toolchain.
- [x] Reconcile stale contract documentation with implemented signatures,
      memory edges and gateway boundaries using source/test citations.
- [ ] Close remaining approved episode/retrieval/recovery requirements with
      regression-first slices, preserving existing golden commitments.
- [ ] Qualify Studio separately with `bun run check`, `bun run lint`,
      `bun run test:native`, `bun run check:native` and applicable browser/native
      journeys. Resume the P2 shell plan from live evidence.
- [ ] Run optional FHE checks for affected feature work; record their separate
      outcome without claiming production cryptography or multi-host evidence.

## Task 3: ABI and Abbey

**Owner:** runtime lane. **Interfaces:** existing sibling WDBX crate APIs,
gateway/CLI/MCP contracts, provider and assistant interfaces.

- [ ] Recover the live training/runtime and assistant plans, preserving their
      currently dirty implementation and instruction files.
- [ ] Complete the first unqualified approved slice in each repo with explicit
      source ownership and regression evidence.
- [ ] Exercise orchestration, memory provenance, cancellation, provider failure,
      restart and streaming acceptance required by the release checklist.
- [x] Run ABI's `./tools/check.sh`, Abbey's `./check.sh`, and retained Zig
      qualification separately as required by their current instructions.
- [x] Recheck consumers after the current additive WDBX interface change: ABI
      full gate and Abbey's 12 focused WDBX consumer tests passed.

## Task 4: Abbey Bot and MLAI/Quesar

**Owner:** application lane. **Interfaces:** existing adapter and learning
contracts; Quesar authentication, publication and sidecar boundaries.

- [ ] Recover Learning Tasks 4–5 without overwriting existing erasure/reset work.
- [ ] Close remaining required consent, routing, memory, voice, reconnect and
      shutdown behavior; qualify the bot's source and live service separately.
- [ ] Resume Quesar's current approved plans from actual source/review evidence.
- [ ] Verify public and authenticated journeys, persistence, responsive states,
      provider configuration, server build and static publication.
- [ ] Run each repo's authoritative gate and relevant runtime acceptance tests.

## Task 5: WebPress refactor and complete authoring journey

**Owner:** WebPress lane. **Interfaces:** existing project model, proposal host,
accepted checkpoints, rendering, publication and runnable export contracts.

- [ ] Preserve and qualify the current refactor before rebuilding any completed
      task. Resolve the first remaining runtime or source failure from evidence.
- [ ] Complete create/edit/preview/save/reopen/publish/export acceptance,
      including undo, revision safety, malformed input and recovery.
- [ ] Verify remaining required CMS, templates, offline, collaboration and
      commerce behavior against approved scope and fixtures.
- [ ] Run WebPress's current full gate and browser/native acceptance separately.

## Task 6: Full brand system and rendered trailers

**Owner:** release/media lane, scheduled in a free slot alongside implementation.

- [ ] Inventory existing brand assets, typography, licenses, motion components,
      cinematic work and `webpress/examples/abbey-trailer`.
- [ ] Complete ecosystem and individual product identities: vector assets,
      colors, typography, icons, motion rules, accessible usage and export kit.
- [ ] Build on existing launch material; produce editable timelines, source
      assets and reproducible render commands, not only HTML storyboards.
- [ ] Render one 90–120 second ecosystem trailer and 45–60 second product cuts;
      deliver 15/30 second derivatives, thumbnails, captions and audio.
- [ ] Export 4K masters and 1080p web versions, with deliberate 16:9 and 9:16
      compositions. Inspect actual rendered frames, pacing, audio and captions.
- [ ] Use real product behavior and reproducible measurements. Label conceptual
      illustrations accurately and exclude private or unlicensed material.

## Task 7: Recursive review, integration and completion

- [ ] For each slice: reproduce failure, implement, focused test, independent
      standards review and independent spec review, repair, then repo gate.
- [ ] Pin the review baseline and source diff; never review an empty range as
      evidence for uncommitted work. Report standards/spec findings separately.
- [ ] Retest cross-repo contracts after integration, then run essential journeys.
- [ ] Record benchmark hardware, revisions, data, configuration, repetitions,
      correctness, latency distribution, throughput and resource use.
- [ ] Audit every release requirement and launch artifact against direct
      evidence. Keep unresolved requirements open; do not equate a local gate
      with full product, provider, hardware or deployment acceptance.

## Execution record

The companion progress file records evidence, task ownership, commands, tool
handles, decisions and remaining work. It is updated as results arrive. The
program is complete only when all tasks above satisfy their acceptance scope.


## Qualified slices and immediate continuation

These bounded deliveries do not close the wider unchecked release requirements.
The evidence index is `../../verification/2026-10-03-delivery-index.md`.

| Slice | Exact seam / deliverable | Gate / next acceptance | State |
| --- | --- | --- | --- |
| Core ML fixture portability | `abi/crates/abi-gpu/native/metal_dot.swift` | 21 focused tests plus ABI full gate; independent review | Current local |
| Studio component behavior | `wdbx/specimen-studio/components/ui/`, `hooks/use-mobile.ts`, component tests and browser command | web/native/lint/Clippy/browser plus five instrumented native tests passed; packaging/signing remains | Current local / Partial runtime |
| Bot offline quality | `abbey-bot/src/brain/quality_evaluation.rs`, DQN import/replay, CLI, 100-case fixture | fix3 regressions, independent reviews and source-stable strict gate pass (2,087 Rust tests) | Current local / Partial release |
| WebPress saved publication | `webpress/src/export/publication.ts` and tests | full gate, actual browser-saved ZIP and independent review passed | Current local |
| Quesar films | `quesar.cloud/scripts/export-films.ts`, static byte validation, narrated-film player | six complete movies; source/static/browser gates and review passed | Current local |
| Bot yanked-package repair | `abbey-bot/Cargo.lock`, yoke-derive 0.8.4 | strict 2,087 Rust / 339 Python / Swift 12+16 gate; five accepted vulnerabilities and three unmaintained warnings remain | Current local / Partial release |
| Quesar claims correction | `src/routes/quesar.tsx`, OG source/card and versioned brand kit | 744 unit / 129 static pages / 226 browser; independent Standards+Spec PASS | Current local |
| Quesar brand distribution | `quesar.cloud/scripts/package-brand.ts`, pinned brand inputs and local archive | reproducibility,71 payload hashes, 54 references, full 744-test gate and review passed | Current local |
| Abbey concept delivery | `webpress/examples/abbey-trailer/render.py`, retained metadata regression | 30/15 second cuts in 1080p, native 4K and recomposed portrait pass | Current concept-format set |
| ABI callback cancellation | `abi-agent-host/src/host.rs` and local contract tests | Six RED regressions; 25 focused pass; independent reviews; fresh ABI 996 Rust +144 Python gate | Current local |
| Ecosystem concept film | `quesar.cloud/notes/launch/ecosystem-trailer-2026-10-03/` | 90/30/15-second 1080p movies, original score, captions, full decode and Chromium playback pass; receipts, checksums and desktop/mobile review-gallery playback pass | Current concept set / Partial format program |
| WDBX/ABI resolver read | Canonical per-edge digest, additive gateway field 9, CLI text/JSON | Independent reviews, WDBX 647 and ABI 988 Rust + 144 Python full gates passed | Current local |
| Release timing observations | Existing 500-vector scalar HNSW harness, five runs | All runs retained; source/binary hashes and resource use; recall unmeasured | Current local measurement |
| Abbey desktop | `abbey/desktop/check.sh` with frozen source and locked dependencies | 49 Rust + 13 Python desktop gate plus seven native Routes UI scenarios and 21 AX snapshots pass; broader desktop journeys separate | Current local / Partial runtime |

The installed Zig compiler used for qualification is
`0.17.0-dev.2338+b46a7f3a2`, newer than the documented minimum
`0.17.0-dev.2320+1e770dbef`; exact historical compiler qualification remains
unverified. Bot's recorded 2026-10-03 07:10 UTC service observation was ready;
that historical observation predates the current candidate. The installed
binary differs from the qualified candidate, so source completion is not rollout proof.

Closed since the prior checkpoint: Bot fix3 strict gate, all six Abbey concept
exports, per-edge WDBX/ABI resolver provenance with fresh full gates, and five
release-profile timing observations. Abbey native Routes UI also passed seven scenarios. Studio instrumented native journeys passed five tests/six scenarios. WebPress native
authored publication reached a verified isolated build but awaits ownership
resolution after external edits to its acceptance window. All implementation and review work
remains capped at four active agents and resource-bounded local commands.


The admitted `abi-agent-host` callback cancellation slice is now locally qualified:
six regressions proved the defect, 25 focused tests pass, both independent reviews
pass, and the root full gate passes 996 Rust plus 144 Python on frozen source.
The wider runtime/training plan is not reopened merely because historical
checklist marks lag verified implementation. WebPress native authored export
remains Partial pending ownership of the externally edited acceptance window.
Remaining product stories, ecosystem 4K/portrait formats, full brand program and
platform/provider/release acceptance remain explicit unfinished work.
