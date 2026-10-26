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
