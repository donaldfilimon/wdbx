//! Cross-implementation conformance: can `abi-wdbx` read what `abbey-bot` writes?
//!
//! `abbey-bot/src/wdbx.rs` is a deliberately small WDBX-v1 *projection*: it
//! speaks this on-disk format but implements no segments, manifest, or audit
//! chain, and it takes no dependency on this crate (see the Abbey System
//! Constitution, invariant A2 and rule I5). Its doc comment claims a file
//! written there loads here and vice versa. Until this file existed, that claim
//! was asserted in prose and gated by nothing.
//!
//! Why a checked-in fixture rather than a dependency in either direction:
//! `abbey-bot` pins **stable 1.97.1** in its `rust-toolchain.toml`, while
//! `abi-compute` — which this crate depends on — requires
//! `#![feature(portable_simd)]` on nightly. No single toolchain compiles both
//! crates, so neither can link the other even as a dev-dependency. The fixture
//! is therefore duplicated in the two repositories on purpose, and each side
//! pins its own copy:
//!
//! - `abbey-bot/tests/fixtures/wdbx_v1_conformance.seg.jsonl` is asserted
//!   byte-identical to its writer's output.
//! - `tests/golden/abbey-bot-projection.seg.jsonl` (here) is asserted parseable
//!   by this reader.
//!
//! If the two copies diverge, one of these two tests fails rather than the
//! incompatibility being discovered in production.

use abi_wdbx::format::{Record, parse_segment};
use std::path::{Path, PathBuf};

fn fixture() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/abbey-bot-projection.seg.jsonl");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing fixture {}: {e}", path.display()))
}

#[test]
fn abi_wdbx_parses_a_segment_written_by_the_abbey_bot_projection() {
    let content = fixture();
    let segment = parse_segment(0, Path::new("abbey-bot-projection.seg.jsonl"), &content)
        .expect("the abbey-bot projection's output must parse here");

    assert!(
        !segment.truncated_tail,
        "the projection must not emit a torn tail"
    );

    let kinds: Vec<&str> = segment.records.iter().map(Record::type_name).collect();
    assert_eq!(
        kinds,
        ["vector", "vector", "kv", "kv", "block"],
        "record kinds and their order must survive the crossing"
    );
}

#[test]
fn the_projections_vectors_and_memory_facts_survive_the_crossing() {
    let content = fixture();
    let segment =
        parse_segment(0, Path::new("abbey-bot-projection.seg.jsonl"), &content).expect("parses");

    let vectors: Vec<_> = segment
        .records
        .iter()
        .filter_map(|record| match record {
            Record::Vector(vector) => Some(vector),
            _ => None,
        })
        .collect();
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].values, vec![0.5, -0.25, 0.125]);
    assert_eq!(vectors[1].values, vec![-1.0, 0.0, 1.0]);

    // abbey-bot namespaces every memory fact by scoped guild id, which is the
    // tenant boundary the constitution lifts to a requirement. The key must
    // arrive intact or guild isolation cannot be reasoned about from this side.
    let keys: Vec<&str> = segment
        .records
        .iter()
        .filter_map(|record| match record {
            Record::Kv(kv) => Some(kv.key.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        keys.contains(&"mem:guild-1:1"),
        "guild-scoped memory key must survive, got {keys:?}"
    );
}
