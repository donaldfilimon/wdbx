# Package 1.0 framing and identities

The exact byte grammar is [design section 2](native-package-design-v1.md#2-canonical-binary-grammar-and-outer-framing).
`WDBXMOD1` starts a 96-byte header, followed by 8–32 directory entries of exactly
80 bytes each. Integer widths are explicit little-endian; there is no native
padding, varint, compression or executable payload. Components occupy one
contiguous positive-length interval each. All reserved bytes and unknown flag
bits are rejected. Versions must match exactly, including recognized minor
versions. Kinds 1–8 are required at schema 1.0, including explicit absent optimizer
and RNG components. Unknown optional kinds remain opaque and cannot satisfy a
required component or alter semantics.

Each component and the directory has an ordinary SHA-256. The package identity
hashes `wdbx-model-package-v1\0` plus the complete artifact; the artifact SHA-256
is also retained for independent admission. State root hashes the
`wdbx-model-state-v1\0` domain and kind/version/digest triples for kinds 1–7.
Generation hashes `wdbx-model-generation-v1\0`, model ID, partition ID, optional
parent generation, state root and provenance component digest. Exact formulas
and field order remain in section 2. No component includes its own generation.
Optional annotations affect package identity but not state/generation identity.

[Section 11](native-package-design-v1.md#11-aggregate-resource-and-hostile-input-rules)
defines checked arithmetic, deterministic error precedence, the 64 MiB artifact,
2 MiB metadata, 1 MiB tokenizer and 256 MiB loading ceilings. Metadata excludes
only numeric tensor/moment and vocabulary-byte payloads; unknown annotations
count fully. N02 must prove its allocation upper bound before reservation, retain
one immutable snapshot, and implement fallible allocation and EOF handling.
The Python witness is an independent schema oracle, not production allocation
qualification or a secure staged-import implementation.

## Clarification 1.0.2: N02 review precedence and staging notes (2026-10-27)

This clarification changes no wire byte, field, version, accepted package or
error family. It records how the N02 parsers behave where design sections 11
and 14 are silent, following the [N02 independent review](../../completion/execution/evidence/2026-10-26-source-pin-migration-review/n02-independent-review.md).

- **Zero training-split digest (M1).** The tokenizer trainer refuses a zero split
  digest as `InvalidTokenizer`. The package decoder and `ByteBpe::from_contract`
  refuse the same zero digest inside a stored tokenizer as `InvalidProvenance`.
  The witness agrees with each entry point. The two families are intentional:
  one is a trainer input error, the other a stored-provenance error.
- **Within-component precedence (M2).** Design section 11 pins first-error order
  between stages only. Inside one component, an input with more than one defect
  (for example a semantic fault and trailing bytes) may get a different family
  from the Rust parser and the witness. Both always reject it. Single-defect
  vectors are the comparison unit for cross-language agreement.
- **Directory hash before entry canonicality (M5).** `directory_sha256` is
  checked before directory entries are read, in both Rust and the witness, so a
  tampered noncanonical directory reports `IntegrityMismatch`.
- **Staging precondition (M4).** `import_staged` requires a caller-owned private
  directory. The existence check and the verifying open are separate path
  operations; with an attacker-writable directory that would be a TOCTOU window.
  The crate denies `unsafe` code and has no `libc` dependency, so no
  `O_NOFOLLOW` open is used. The precondition is the mitigation.
- **Best-effort temporary unlink (M3).** Once the object is linked and the
  directory synced, a failed removal of this call's temporary name no longer
  reports `Io`. The `.partial` file is retained for explicit cleanup, like other
  incomplete task-owned files.

`n02-matrix-negatives-v1.json` adds single-defect vectors for section 14 matrix
items that previously had no fixture (finding M6); see `n02-fixtures.md`.
