# Private complete-episode Rust decoder receipt

2026-10-20. The bounded private typed decoder slice passes independent specification/quality review and source tests. This closes this implementation slice only; full completion task02 remains open.

The decoder consumes the existing provisional canonical envelope, retains its typed input, reuses existing constructor and relational validation, honors stricter caller limits while clamping input to1MiB, and rejects any input whose exact bytes differ after canonical reconstruction. It does not sort malformed received collections into acceptance. Diagnostics are static and the retained wrapper's Debug is redacted. Existing encoders, model validation, provisional pins, legacy domains and signatures remain unchanged.

Evidence:

- Meaningful absent-API RED, then53 focused tests passed.
- 64 unchanged selected integration tests passed, including legacy commitments, Python witnesses, signing and store replay.
- Independent review passed without actionable findings; all13 owned source hashes matched.
- Full `bash tools/check.sh` passed774 tests, zero failed/ignored,32 summaries on the immutable ABI `cohort-1/wdbx` capture. All13 decoder hashes were checked against that passing cohort. This uses the refreshed root lock, pinned Rust1.99.1 and offline dependencies.

The [source inventory](../../completion/complete-episode-decoder-v1-source.json) binds the owned files, root lock and passing cohort. Detailed implementation and independent review records remain in `.superpowers/sdd/2026-10-20-completion-remaining/decoder-report.md` and `decoder-independent-review.md`.

An earlier canonical gate passed770 tests but the unrelated filesystem source changed during execution, so that run cannot qualify latest canonical bytes. Subsequent writer-lock source changes are being qualified separately on the shared newer consumer cohort. The774-test receipt above remains bound to its captured source.

Python encoding plus Rust typed decoding does not establish an independent Python typed decoder. Its independently reviewed ingress layer currently passes10 focused tests; the inverse, relational walk and final differential integration remain separate tasks. ABI named-owner ratification, public wire freeze, COSE/trust lifecycle, independent cryptographic review and production qualification remain open. No full constitutional admission or task02 completion is claimed. No commits or pushes by this lane.
