# Portable archive compatibility witnesses

`legacy-zip4.wdbxspecimen` was produced by the **actual** predecessor
`specimen_core::persistence::Store::export`, using ZIP 4.6.1 at source commit
`81ae29a7c472d42765fd1f09f75c1288b0252d30`. It is isolated test data, not a
user store or evidence of trained-model qualification. `provenance.json`
pins that source tree, the relevant source/lock/starter bytes, the producer,
the helper lock and the captured fixture bytes. The actual command/environment
receipt is retained privately by its SHA-256 reference.

`producer.rs` is the qualification harness that was run. It opens a fresh
scratch store, uses the predecessor's starter specimen, sets the name to
`Café 東京 — portable legacy`, retains the exact fractional jitter
`1.8688716782771921_f64`, stores `asset.bin`, and references its content digest
from node provenance. It calls the predecessor's `edit` and `export`, then
uses ZIP 4 to capture the uncompressed `manifest.json`. The manifest remains
`wdbx.native.v2`; the archive extension remains `.wdbxspecimen`.

To reproduce, supply a read-only checkout of the pinned predecessor as an
explicit qualification input. In an ignored, disposable helper workspace,
copy `producer.rs` to `src/main.rs`; declare `specimen-core` as a path dependency
on that checkout, plus `serde_json = { version = "1", features =
["float_roundtrip"] }` and `zip = { version = "4", default-features = false,
features = ["deflate"] }`. Seed the helper lock from the predecessor's Studio
lock, resolve that helper once, then run it with locked Rust 1.99.1:

```sh
cargo run --locked --manifest-path "$HELPER/Cargo.toml" -- \
  "$FIXTURE_OUTPUT" "$FRESH_SCRATCH_STORE" \
  "$LEGACY_SOURCE/specimen-studio/conformance/starter.json"
```

Keep compilation output and the helper outside the read-only source. The helper
is qualification tooling, never a Studio runtime dependency. ZIP timestamps and
compression bytes may differ on reproduction; semantic manifest/asset bytes are
the compatibility oracle. The committed captures themselves are hash pinned,
and tests never regenerate them.

The current reader imports that capture and compares every typed snapshot field,
exact fractional bits, non-ASCII text and asset bytes, including after reopen.
Its new export is also read by Python's independent standard-library `zipfile`
reader: member names, CRCs and uncompressed manifest/asset bytes must match.
Python 3 is required for these witnesses; absence fails the test.

`streamed-overrun.zip` is an independent raw ZIP fixture with one deflated
asset. Both headers declare one uncompressed byte; the compressed payload
actually expands to 32 MiB + 1 bytes of `Z`. Its headers agree, so this tests
streamed-size refusal rather than local/central disagreement. The test verifies
its header fields and independently decompresses it with Python `zlib` before
checking that Studio refuses it without durable writes. The small compressed
fixture avoids allocating a 512 MiB archive. It was generated with raw DEFLATE
(`compressobj(6, DEFLATED, -15)`), 32 one-MiB `Z` chunks plus one byte, true CRC
and compressed size, and uncompressed size 1 in both ZIP headers.

Other hostile archives are built in memory by `tests/support/portable_archive.rs`,
a small stored-method byte writer with its own CRC implementation. It does not
call the production ZIP writer. Tests assert relevant byte structure for
central duplicates, mismatched header names/sizes, nested overlapping members,
invalid UTF-8 flags, truncated payload/directory and corrupt CRCs. Count/size
fixtures use controlled metadata; the input-file limit uses a sparse file.

Each refusal checks the entire snapshot, assets, recovery copies and durable
store files before/after, then reopens the store and checks again. Ephemeral
writer-lease files are excluded because reopening creates a fresh lease; durable
journals/heads remain included. Parent traversal and absolute-name tests retain
sentinels outside the workspace. Accepted symlink/FIFO mode metadata remains
inert byte content: no symlink or special file is created, external content is
not read, and Unix executable bits are not applied. Unix mode assertions are
conditional; native Windows/Linux execution is still required for those hosts.
