# Compatible WDBX dependency refresh receipt

2026-10-20. Qualified source base `8c5ed7faf412d9098ecacad5ca2c10a68c935279`, before typed-decoder implementation. User authorized compatible Cargo updates after recoverable backups. Current direct user instruction keeps all changes uncommitted on main and prohibits pushes.

The previously unknown root-lock candidate was preserved byte-for-byte, along with its baseline patch, hash/provenance metadata and reconstruction check. Root, standalone fuzz and observed Studio locks were backed up and read back before any mutation. Only root/fuzz source locks were removed and regenerated, each once. No registry/cache/vendor/target operational lock, manifest pin or unrelated source file was removed or changed. Studio’s intentional path dependencies and existing Cargo.lock remain untouched under direct user coordination; Grok’s three documentation/comment paths remain separately owned.

Pinned Rust1.99.1 and Cargo fallback resolution produced compatible locks. Root remained209package records and exactly reproduced the preserved candidate: SHA-256 `e2fac4f3f125cecc771a0dc7d6f33eb1629972b3979ac0771849fdf61368680e`. Fuzz changed153 to125package records: `b02d27422795c1785c9f4119f5e6a1b57e296bb74d543c908f7f285928a2aa83`. Locked fetching and root workspace/all-targets checking passed; fuzz’s locked binary check passed.

Controller verification on this dependency cohort, pinned compiler, offline Cargo and serialized builds:

| Command | Passed | Failed / ignored |
|---|---:|---:|
| `bash tools/check.sh` |735|0 /0|
| `cargo test --locked -p abi-wdbx --features full-fhe` |467|0 /0|
| `cargo test --locked -p abi-wdbx --features experimental-dghv-bootstrap` |467|0 /0|

The full-FHE suite includes execution with ephemeral test keys. These are local functional tests, not independent cryptographic review, security assessment, production or performance qualification. Fuzz evidence is compilation only, not sanitizer fuzz execution. No Studio/browser/platform or research-efficacy claim follows from root gates. Later decoder/source changes require their own receipts.

Independent specification/quality review passed after correcting stale archive status text: the original candidate’s unknown authorship is preserved as history, while the later authorized regeneration explains its current active use. Durable [backups and resolution evidence](../../completion/dependency-candidates/2026-10-20-compatible-refresh/metadata.json), [semantic resolution changes](../../completion/dependency-candidates/2026-10-20-compatible-refresh/results.json), and [controller gate hashes](../../completion/dependency-candidates/2026-10-20-compatible-refresh/controller-gates.json) accompany the receipt. Raw logs are stored beside those manifests; original candidate and patch are in the adjacent unqualified-cargo-upgrade archive. No commit or push occurred in this continuation.
