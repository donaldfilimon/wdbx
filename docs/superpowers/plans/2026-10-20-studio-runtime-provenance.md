# Studio runtime build provenance correction

Approved full-completion qualification task; user authorized building pinned diffusion/ggml and needed llama components. Current direct user prohibits commits/pushes and requires preserving Studio path dependencies/lock and Grok’s three files. This changes only the runtime helper and focused tests; not the model runtime, inference semantics, source pins or Cargo manifests.

## Problem and owned paths

The existing helper downloads tar archives inside the WDBX Git tree. Upstream CMake discovers the parent repository and embeds a WDBX commit in sd-cli and llama-server instead of the source pin. Existing builds/loaders work but their embedded identity is wrong. Own specimen-studio/scripts/build-runtimes.py plus focused standard-library tests under specimen-studio/scripts/tests/ (new file). Read Studio AGENTS, RUNTIME-PROFILE.md and NATIVE-RUNTIME-PROFILE.md. No changes to downloaded upstream source, source pins, Cargo/JS locks, consumer files or Grok paths. All builds/models/services/listeners outside the explicit local helper remain out of scope.

## Required behavior

Preserve existing selected targets, CPU/static configure flags, platform output paths, parallelism and license handling. Isolate Git metadata on configure AND build: GIT_EXE:FILEPATH=OFF, GIT_EXECUTABLE:FILEPATH=OFF, CMAKE_DISABLE_FIND_PACKAGE_Git:BOOL=TRUE. These suppress both upstream Git discovery mechanisms, including stale cached real paths. Clear stale GGML_GIT_DIRTY cache state with -UGGML_GIT_DIRTY. Use only supported cache overrides, as verified in runtime-provenance-preflight.md:

- llama: LLAMA_BUILD_COMMIT full llama pin; LLAMA_BUILD_NUMBER=0 (unknown archive count). Its bundled ggml belongs to the llama tree; do NOT assign the separate external ggml revision there.
- diffusion: SDCPP_BUILD_COMMIT full diffusion pin; SDCPP_BUILD_VERSION=unknown (do not invent an upstream tag); GGML_BUILD_COMMIT full separately pinned ggml revision and GGML_BUILD_NUMBER=0.

Use a copied subprocess environment that removes inherited GIT_DIR, GIT_WORK_TREE and GIT_COMMON_DIR and adds the absolute work/runtimes path to Git ceiling directories as defense in depth. Do not mutate process-global os.environ. Do not disable Git globally or change user Git config.

Before accepting/copying each built executable, run its non-serving --version with a finite timeout and require success plus the expected full source commit in reported identity (if upstream genuinely truncates, report and settle precise formatting before accepting; do not silently accept unrelated/unknown identity). Failed verification must leave an existing destination executable unchanged. Never run a model or start a listener. Preserve successful --help/version outputs and source/binary hashes in controller qualification, not fabricated test output. Minimal refactoring into callable orchestration functions is allowed to make tests meaningful; no broad packaging redesign.

## Red-first verification

Write failing standard-library tests using temporary fake source trees, independent fixture pins, a fake CMake executable and fake non-serving outputs; no downloads or real models. Cover each --only path and both targets, supported pin metadata, no invented count/tag, clean child Git env and original global env unchanged, stale real Git cache override, source/target scope for ggml, and rejection of wrong version metadata with prior destination preserved. Avoid tests that only compare a hardcoded list to itself: fake configure/build should derive its executable metadata from actual supplied arguments and fail if the fix is removed.

Also use a tiny real CMake fixture with instrumented fake Git to demonstrate defined OFF blocks find_program and FindGit; verify emitted commit fields match independent pins and Git is never called. This probe configures a synthetic fixture, not product inference. CMake is a real required local prerequisite; no missing-tool skip masquerading as passing coverage. No Git commits in fixtures.

After tests pass and independent spec/quality review, controller rebuilds both pinned components, verifies version/help/loader and hashes, then runs Studio native/browser/package gates on a frozen source/lock snapshot. Builds alone are not inference, hardware or installed-platform qualification. Do not hide upstream warnings or weaken gates. No staging/commits/pushes/subagents. Report exact RED/GREEN commands, changed-file hashes and scope limits in runtime-provenance-report.md in this scratch workspace. Only implement after decoder source freeze clears the sole product-writer slot.
