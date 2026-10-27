# Task 1 implementation report

Status: implementation and assigned checks complete; independent review/integration owned by controller.
Commit: `a2e2f26887ccdcf6cce493f3a507ed532193fe9d`.

Owned changes: shared checker/tests, both parity scripts, additive transition JSON,
and Studio successor verification document. No original snapshot, substrate source,
execution register or parent plan edits. No branches, installs, live stores or push.

Scoring baseline also covers the language parity script. Controller approved an
explicit fourth scoring transition (fifth physical file), with original hash
retained. Its provenance is the preimplementation `ad92e81` base; this report
binds final commit above. Current transition membership is otherwise exact.

## RED / GREEN and checks

Commands run from repository root unless specified. Logs retained in
`.work/preservation-task1/`.

- RED: `python3 -m unittest discover -s specimen-studio/scripts/tests -p test_check_preservation.py` before checker creation: 8 errors, missing checker module (`red.log`).
- GREEN: same command, 11 tests pass (`green.log`). `/usr/bin/python3` 3.9.6 same suite: 11 pass (`python39.log`). Isolated synthetic fixtures, no live stores.
- `python3 specimen-studio/scripts/check_preservation.py --profile language --mode current`: exit 0; scoring same: exit 0. Historical language: exit 1 with actual two changed paths; historical scoring: exit 1 with actual four changed paths (`checker-modes.log`).
- Both shell entrypoints reject unknown arguments and extra arguments with exit 2 (`checker-modes.log`).
- `bash -n specimen-studio/scripts/check-language-parity.sh specimen-studio/scripts/check-scoring-parity.sh`: pass.
- `git diff --check`: pass.
- `python3 -m unittest discover -s specimen-studio/scripts/tests -p '*gtk*.py'`: 12 pass (`gtk-tests.log`).
- `python3 specimen-studio/scripts/check_gtk_vendor.py --archives /Users/donald/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f --metadata`: source/license/archive identities and both locked resolution graphs pass (`gtk-check.log`). An initial invocation omitted the required archive directory and failed argument parsing; corrected invocation above passed.
- From `specimen-studio/`: `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$PWD/../.work/preservation-task1-gtk" sh scripts/pinned-cargo.sh test --locked --manifest-path native/vendor/Cargo.toml -p gtk-compat-tests --no-default-features --test diagnostics < /dev/null`: 1 passed (`gtk-diagnostics.log`). Existing rust-objcopy libLLVM loader warnings were emitted during compilation; intended macro diagnostics harness passed.
- From `specimen-studio/`: `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$PWD/../.work/preservation-task1-native" sh scripts/pinned-cargo.sh test --locked -p specimen-core --test portable_archive < /dev/null` 12 passed (`archive-tests.log`, isolated retained build target).

Native/WASM comparators and negative controls unchanged; artifact hashing now
precedes final preservation, with checker/tests/baselines/transition source hashes
retained. Controller runs full parity gates after independent review. No platform,
production, migration, efficacy or C0 promotion claim. HawkScan tools/executables
unavailable; no scan, installation or DAST claim.

## Completed log SHA-256

- `red.log`: `fdfc1fcf955435672ac590af78f8a0225963c4dbda1a0e2a6e5abfd20df16d3a`
- `green.log`: `1c4b83773f5fd296518181cb8267dd589ab12a3b1f8e3f5036458eb9bc854f0e`
- `python39.log`: `a4dc62e635c962210ee88ba70305a9199d885cc3f380ce10212ec43d17bacabd`
- `checker-modes.log`: `6b7842d7a6d2b6043fc8fb9e5dc994da78c355697b9251152a84ad5b0c794780`
- `gtk-tests.log`: `3bf975f2f7d16a6354a6c8c88599076bab9c38c1d1ba1b1149d10f1d4a8dfb4a`
- `gtk-check.log`: `0ae7797990e060ddeb12819d25bdc84f1fb1a3755c75caf673fa297ccb4c1207`
- `gtk-diagnostics.log`: `8d52619f10d2bcb1876725243fa581046adc8779d47594b778a0d6187019bc00`
- `archive-tests.log`: `b59d08d345cb1b6c945a9f604e30f012b362d4a99966b22d1a19fabf13457b24`
