# Independent review: T02 and T03

Reviewed the two supplied scoped patches, actual corresponding sources, briefs/reports, raw T02 green and T03 red/green logs, the zero-count validator test, v6 sections 12–14, and v5 section 5.2. Review only: no tests rerun, production edits, live stores, commits, pushes, or CI. The earlier 647-test default gate predates these changes and is not final evidence.

## Verdicts

| Packet | Spec compliance | Code quality |
| --- | --- | --- |
| T02 | Accept scoped characterization, with explicitly retained qualification gaps below. Does not qualify the complete dependent profile. | Accept; no blocking defect found. |
| T03 | Accept scoped finite numeric primitives; satisfies the brief and applicable numeric requirements. | Accept; no blocking defect found. |

These are independent packet verdicts, not approval of T00/T01, the complete profile, or a final workspace gate.

## Findings and qualification limits

**Medium, dependent-profile evidence gap (not a T03 implementation defect): v2 authenticated bytes are narrower than v2 spatial/commitment compatibility.** `crates/abi-wdbx/tests/wdbx_3d_compatibility.rs:250` embeds `parents` and `point` as arbitrary JSON bytes inside an Artifact, then uses `seal_object`/`open_object` at lines 252/265. This meaningfully pins envelope encoding, signature authentication and opaque payload preservation, including literal `-0.0` and parent order. It does not instantiate `V2SpatialRecord`, derive the frozen JSON commitment, recover a journal or read a signed segment. A regression in typed spatial serialization or parent commitment could leave this test green. The limitation is accurately declared in `tests/fixtures/wdbx_3d/compatibility-manifest.json:34` and T02-report; retaining that scope is acceptable for this additive constructor packet. Before claiming full compatibility/profile qualification, provide stronger pinned signed v2 journal/segment evidence containing typed f32 spatial values (including negative zero) and ordered parents, or explicitly retain this as unqualified. Existing relevant tests include `src/v2/tests.rs:235`, `:346`, `:382`, `src/v2/segment/tests.rs:188`, and `src/v2/replication.rs:723`; their existence is not evidence of a post-change run or of byte pins for this precise corpus.

**Low, future evidence-maintenance risk:** `tests/fixtures/wdbx_3d/compatibility-manifest.json:42` duplicates the hard-coded legacy ordering characterization in `wdbx_3d_compatibility.rs:120`. The digest pins are executable, but descriptive ordering/report fields are not consumed as expectations. A future change can leave prose and assertions inconsistent. This does not invalidate current evidence; keep both synchronized or derive those descriptive fields in later maintenance.

Attachment-index operations remain blocked because no such API exists. T02 cannot demonstrate preservation across those operations yet. Full legacy reader/writer matrices, encrypted migration/recovery, browser/desktop and optional FHE qualification remain outside these packets. Neither report incorrectly claims those results.

## T02 evidence assessment

The three green cases are meaningful characterization, so an initially green baseline is appropriate; no manufactured behavioral red is required. The v1 case copies repository fixtures into fresh scratch directories, snapshots relative filenames and every byte, verifies the chain, and checks both the empty highest checkpoint and populated monolithic layout. The spatial case exercises 63/64/65 records, actual duplicate ID 7 payloads, Euclidean/Manhattan all-hit and tie-cutoff nearest selection, and cosine radius order. Duplicate u32 identity stays inside the legacy local index.

The signed case pins deterministic v2 envelope and v3 ledger bytes plus the v3 episode digest, reopens and authenticates, and checks unchanged scratch files. Its synthetic seed is visibly test-only. The zero-count case is appropriately owned by `tools/tests/test_wdbx_3d_evidence.py:55`: changing only test_count from zero to four changes validation rejection to acceptance. It tests metadata qualification, not automatic Cargo log ingestion, as the report accurately states. T02 green log contains three executed, zero filtered tests. Initial fixture-authoring corrections are disclosed and do not masquerade as production regressions.

## T03 implementation and evidence assessment

`src/spatial3d/numeric.rs:28` rejects every nonfinite component before checking the closed magnitude domain. `:41` uses exact f32-to-f64 widening and routes through the same constructor. `:71` rejects nonfinite/negative/out-of-domain radii and accepts negative zero, normalizing only the newly created value. Private components, by-value read accessors, and absence of Default/deserialization/unchecked construction preserve validation. No legacy API/body, durable identity, persistence, signing, gateway or hardware implementation changes occur; lib.rs adds only the public module. No unsafe code is added. Rust files in these scoped patches are 5/92/115/290 lines, below 1000.

The four red failures are executable assertion failures against accepted NaN, accepted negative radius, and unnormalized signed-zero bits, not compile/environment failures. The subsequent green log executes exactly four cases. The tests cover each coordinate axis, both out-of-domain signs, the immediate representable value above the limit, f64 extremes/subnormals, representative f32 endpoints and normal/subnormal transitions, legacy nonfinite rejection, zero bits and radius endpoints. The name `widens_all_finite_f32` denotes the contract rather than exhaustive enumeration; implementation uses standard exact widening, so the representative coverage is appropriate. No redundant rerun is needed for this review.

Shapes, frames, predicate arithmetic, exact ordering, validation of already indexed legacy points, allocation/cancellation and index qualification belong to later packets. Their absence is not a defect in the explicitly scoped T03 constructors. v6 section 12.1 still requires later frame binding to leave these legacy points explicitly unitless; this patch infers no units.

## Remaining completion evidence

Focused runs support the stated packet behavior. A final post-change supported workspace gate remains required by the parent workflow before overall completion. The default pre-change gate cannot be presented as that gate, and no optional feature/runtime claims follow from these focused logs.
