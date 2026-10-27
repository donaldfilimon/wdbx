# GTK 3 compatibility backport

Tauri's GTK 3 graph needs GLib 0.18. These complete local crate archives retain
GLib 0.18.5, glib-macros 0.18.5 and gtk3-macros 0.18.2 identities, licenses,
attribution and upstream tests. The original GTK macro description and dirty VCS
marker are retained. This repairs compatibility; it does not establish GTK 3
maintenance or whole-product security clearance.

GLib's existing two-line `VariantStrIter::impl_get` fix for
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) is
unchanged: the C out-pointer is mutable and passed as `&mut p`, following
[upstream commit b5a4071](https://github.com/gtk-rs/gtk-rs-core/commit/b5a4071e439bef2b5eea76c3aa25e5ae84839e34).
The original archive SHA-256 is
`233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`.
Its upstream revision is `42b9caf98e03ded086362d9653ca58fe94dc8658`.

Both macro parents replace `proc_macro_error::` with the real
`proc_macro_error3::` import, keeping the `proc_macro_error` attribute name.
Both active and original manifests record this local modification, pinning
`proc-macro-error3 = "=3.1.1"`, `default-features = false`, `features = ["syn2-error"]`.
Published attr3 3.1.1 generates `::proc_macro_error3::entry_point`; aliasing the
new package to its old name would fail actual expansions. The default Syn 3
feature would mismatch the parents' Syn 2 API. The earlier version-2 proposal is
rejected because [RUSTSEC-2026-0173](https://rustsec.org/advisories/RUSTSEC-2026-0173.html)
reports no patched maintained version.

`source-manifest.json` records original archive URLs/checksums, complete original
and patched file hashes, exact changes, supplied licenses and VCS identities,
including unmodified registry error3/attr3 sources. Original checksums describe
original bytes, never patched bytes. `scripts/check_gtk_vendor.py` pins that
manifest, rejects source/license/metadata/package drift, checks both local Cargo
resolution graphs and the replacement feature graph. Its optional `--archives`
mode checks exact transformations against read-only original archives. This is
provenance/license inventory, not complete distribution-license clearance.

This directory is an independent workspace with its own patches, lock and dev
GLib opt-level 3. Studio excludes it and carries the same three local patches;
profiles and patches do not inherit across these roots. Studio's independent
GLib 0.21 graph and ZIP/Tokio/ctutils resolutions are preserved.

From `specimen-studio/`, using the pinned Rust 1.99.1 Cargo/rustc and an isolated
absolute `CARGO_TARGET_DIR` for the vendor workspace:

```sh
bun run check:gtk
bun run test:gtk
cargo test --locked --manifest-path native/vendor/Cargo.toml -p gtk-compat-tests --features glib-runtime --test glib_runtime < /dev/null
cargo test --locked --manifest-path native/vendor/Cargo.toml -p gtk-compat-tests --features glib-runtime --test glib_runtime --release < /dev/null
cargo test --locked --manifest-path native/vendor/Cargo.toml -p glib --lib variant_iter < /dev/null
cargo test --locked --manifest-path native/vendor/Cargo.toml -p glib --lib variant_iter --release < /dev/null
```

`check:native` runs the provenance/graph/negative-control gate before Clippy.
The diagnostic harness pins compiler identity and requires four intended macro
errors, rejecting panic/unresolved-crate failures. Its subprocess manifest keeps
optional GTK declared without compiling it. Generated diagnostics remain in the
isolated target. GLib runtime fixtures test enum `Value` roundtrips, `clone!` and
optimized empty/single/Unicode/mixed-direction string iteration.

On native Linux with GTK 3 development libraries and a declared display (such as
`xvfb-run -a`), add `--features glib-runtime,gtk-runtime` and run both dev and
release. The GTK fixture uses the real public derive, subclass, inline template
and bound child; it requires successful initialization and asserts label content.
Also run Studio's existing Linux-only
`optimized_glib_variant_iteration_backport` conformance regression in both
profiles, requiring one executed test. A macOS zero-selection result cannot
qualify that test. See `verification.md` for actual outcomes and remaining gates.

Remove these backports only when the consumer can select fixed upstream sources
and the same compatibility checks pass. Reverting the task commit restores the
previous dependency graph; no live-store migration is involved.
