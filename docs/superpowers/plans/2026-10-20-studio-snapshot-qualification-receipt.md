# Studio captured-source qualification

2026-10-20. Local macOS ARM64 checks used an isolated source copy at `/Users/donald/.codex/verification/wdbx-studio-20261020-snapshot/wdbx`. Its656 source/build inputs were copied and rechecked before execution; dependencies retain the user's intentional local paths and unchanged lockfiles. Installed JavaScript dependencies came from the completed frozen install. No branch, canonical source reset, commit or push was made by this lane.

| Check | Result |
| --- | --- |
| Native tests |90 passed,0 failed,1 GPU parity test ignored |
| Native Clippy (`-D warnings`) | exit0 |
| Full browser `check` | exit0;195 tests,0 failures,853 assertions across29 files; typecheck, lint, WASM release/golden probes and production Worker build included |
| Release desktop compile | completed |
| `.app` bundle creation | completed |
| Full `desktop:package` | exit1 during DMG bundling |
| Strict deep app signature verification | exit1; not distribution-ready |

Commands used pinned Rust1.99.1, offline Cargo and two build jobs. Browser and package commands used Bun's explicit `--shell=bun` option. Native tests/check used their named package scripts. The actual logs retain `rust-objcopy` failures loading `libLLVM.dylib` and an upstream `block` future-compatibility warning. A direct probe succeeds when pointed at the pinned toolchain's own library directory, but that experiment did not remove the warnings from these actual package logs. No gate, deadline or assertion was weakened.

Verbose bundling isolated the DMG failure: image creation, mounting and Finder layout completed, then normal detach failed three times with Resource busy. A WDBX Studio executable was running from that temporary packaging image. This lane did not launch the application; its origin remains unknown. A later normal detach also returned16. No process was killed and no forced unmount was attempted. The owned temporary image remains pending safe cleanup after that application releases it. Exact process/mount diagnostics remain in the private verification directory.

The original built executable carries a linker ad hoc signature. `codesign --verify --deep --strict` reports that resources required by the signature are absent; that bundle has no sealed resources or TeamIdentifier. A separate local profile was subsequently signed inside-out using only identity `-` and no timestamp. Its helpers and app pass strict verification, including deep verification and independent review. Original bytes/modes remain unchanged; non-signature Mach-O payload equality and unchanged non-Mach-O files were checked. The [local profile recipe and receipt](../../completion/studio-snapshot-evidence/2026-10-20/local-adhoc/receipt.json) preserve exact hashes and verification. This local ad hoc profile does not establish Developer ID signing, notarization or installed-byte qualification. No credentials were requested or fabricated, and neither artifact was published.

[Qualification inventory](../../completion/studio-snapshot-evidence/2026-10-20/qualification.json) records source-manifest/log hashes, exact commands, signature/loader inspection and artifact hashes. Both packaged runtime sidecar hashes match the separately qualified pinned-source builds. No model weights, inference, app launch, installed UI, OCR accuracy, acceleration, browser interaction server, upgrade/uninstall, other platform or hosted recovery qualification follows from these checks.

Canonical WDBX continued changing in parallel. These results qualify the captured Studio/path-dependency cohort only; the final shared producer gate and consumer gates have separate source manifests. Full Studio and platform completion remain open, including the failed package/signing gates above.

## Separate portable image profile

The existing `scripts/seal-macos-package.py` workflow supplied a nonmounting route: create a compressed UDZO image directly from the separately verified local ad hoc app and an Applications symlink, then verify the image. The new portable profile completed and passed independent review. Image SHA256 is `caff5ebc8f6f892f473888ea9b3efe413ec6fb488d430523fecc29aa29f56492`; size is46,367,385 bytes. `hdiutil create`, `verify` and `imageinfo` exited zero; staged app strict/deep signature checks passed and source/staged bytes and modes remained unchanged.

The [portable image recipe and receipt](../../completion/studio-snapshot-evidence/2026-10-20/portable-dmg/receipt.json) retain exact commands, input hashes, output hash and limits. The image lives outside the repository in the private verification directory. Neither this route nor its independent review mounted or extracted the image, ran the GUI, closed the unknown application or changed the busy original volume. Image checksum validity is not proof of extracted/installed-byte identity. Developer ID, notarization, Gatekeeper, installed UI and the original default `desktop:package` gate remain separately open.
