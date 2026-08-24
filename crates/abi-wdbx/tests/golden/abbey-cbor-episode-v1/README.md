# `abbey-cbor-episode-v1` positive vectors

These synthetic fixtures pin the exact deterministic-CBOR bytes and SHA-256
digest produced by the v3 commitment module. Both use schema version 1, an
empty header map, and payload `{1: "synthetic"}`. `empty-parents` has no causal
parents. `two-parents` is deliberately supplied to the encoder in reverse order
and commits the lexicographically sorted `0x11…` and `0xee…` digests.

The `.hex` files are the canonical CBOR bytes represented as lowercase hex;
the corresponding `.sha256` files are the lowercase digest of those decoded
bytes. They contain no user, provider, Discord, credential, or production data.
