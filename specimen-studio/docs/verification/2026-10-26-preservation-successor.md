# Reviewed Studio preservation successor

The parity scripts now default to a reviewed **current** identity. This is a
successor claim, distinct from historical unchanged bytes. The original language
23-file and scoring 80-file JSON baselines remain byte-identical, independently
SHA-256 pinned by `scripts/check_preservation.py`. Their pinned complete bytes
bind membership as well as hashes. All unchanged original rows remain enforced.

The additive transition manifest at
`../../../docs/completion/execution/evidence/2026-10-26-studio-preservation-transition.json`
records prior/current hashes, reviewed provenance and rationale. Its entire digest
is independently pinned in the checker. Language transitions only `Cargo.lock`
and native `persistence.rs`; scoring transitions Studio `AGENTS.md`, native core
`Cargo.toml`, native `persistence.rs`, and `scripts/check-language-parity.sh`.
The fifth physical file is necessary because the historical scoring inventory
itself covers the language script now wired to this checker. Its provenance
names the continuation base; the task report binds the resulting implementation
commit rather than inventing a future commit hash. Scoring's prior instruction
identity corresponds to `651d178`, rather than the common `d2d0b61` baseline.

The ZIP9 persistence transition includes fallible member-name handling and
stricter malformed archive header/overlap refusal. It preserves valid portable
save behavior, not acceptance of ambiguous malformed archive bytes. Dependency
and GTK changes are reviewed compatibility modernization, not unchanged source.

Both parity scripts accept only no arguments or `--historical-preservation`.
Historical mode executes the original hash checks and the language profile's
original exact, exactly-once immutable `entries` accessor normalization; it
currently refuses the reviewed source evolution. Scoring applies no accessor
normalization. No unknown, duplicate, missing, extra or malformed transition
entry is accepted, even if someone updates the manifest digest in isolation.
A changed or missing current file still fails.

The existing native/WASM comparisons, corrupt-digest/label and
arithmetic/quarantine/boundary negative controls are retained. Source receipts
include the checker, its tests, both baselines and the transition. Output artifact
hashes are written before the final preservation check, so its refusal retains
the native/WASM outputs and receipts. All build directories remain retained.

Focused synthetic Python regression tests cover both modes, changed and untouched
source, every transition's mutation/removal, malformed/duplicate/extra/missing
manifest entries, baseline/manifest digest tampering and accessor exactness.
Run from the repository root:

```sh
python3 -m unittest discover -s specimen-studio/scripts/tests -p test_check_preservation.py
bash -n specimen-studio/scripts/check-language-parity.sh specimen-studio/scripts/check-scoring-parity.sh
```

These checks do not promote any completion requirement, prove platform or
accelerator acceptance, migrate live stores, or establish production qualification.
HawkScan is unavailable here; no DAST claim is made.
