# GLib compatibility backport

Tauri 2's GTK 3 dependency chain requires GLib 0.18. This source copy retains the original 0.18.5 crate, license and attribution, with the two-line upstream fix for RUSTSEC-2024-0429: the C out-pointer is mutable and passed as `&mut p`.

- Original crate: https://static.crates.io/crates/glib/glib-0.18.5.crate
- Original archive SHA-256: `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`
- Original upstream revision: `42b9caf98e03ded086362d9653ca58fe94dc8658`
- Upstream fix: https://github.com/gtk-rs/gtk-rs-core/pull/1343
- Advisory: https://rustsec.org/advisories/RUSTSEC-2024-0429.html
- Changed source: `glib-0.18.5/src/variant_iter.rs`, `VariantStrIter::impl_get` only.

The root Cargo patch selects this copy. Its API/version remains compatible with GTK 3; this is not a claim that unpatched registry version 0.18.5 is safe. The Linux conformance test exercises forward, reverse, nth and last string iteration, with GLib optimized in both dev and release profiles. Remove this backport when Tauri's dependency chain can consume a fixed upstream release directly.
