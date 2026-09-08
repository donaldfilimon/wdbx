# Release candidate execution record

Approved target: browser and native 0.2 profiles; macOS ARM64/Intel,
Windows x64 and Linux x64 candidates. No publication or deployment.
Reference macOS runners qualify real text/image inference; other platforms
qualify helper startup and portability. Explicit architecture extensions remain
outside this release.

## Progress

- Baseline: canonical main at ee9973520b957c64bd1084045078655dfe655088,
  with the existing engine/test negation patch preserved.
- Browser scope and regression matrix: in progress, browser agent owns engine,
  Bun discovery config, runtime profile and specification synchronization.
- Browser CI and final acceptance ledger: in progress, root owns workflows
  except platform packaging, README and validation/release documentation.
- Packaging receipt/signing/install audit: in progress, packaging agent owns
  packaging scripts/tests and Windows/macOS packaging workflows.
- Native boundaries and local qualification: in progress, native agent owns
  Rust/desktop code and native interface tests.
- Baseline locked Rust tests: 11 unit + 17 conformance passed, one GPU ignored.
- Baseline locked workspace all-target Clippy: passed; dependency future
  incompatibility notices remain separate from warnings-denied gate result.
- Hosted run 33878636044 at b2c382d qualified text-model only; desktop skipped.
- Credential inventory: GitHub repository secret list empty; no local Developer
  ID Application identity. Signed release candidates require credentials.

## Decisions

Use the canonical main checkout as explicitly approved. Agents own disjoint
paths and must preserve concurrent work. Do not change ABI/Abbey/WDBX siblings.
Historical hosted successes do not qualify a new candidate source commit.
Manual physical touch, VoiceOver and unavailable platform acceptance remain
unresolved until actually exercised. No fabricated acceptance or receipts.
