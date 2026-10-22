//! The vector-index adapter and the substrate-side vector-id binding.
//!
//! These tests pin the slice that closes the two documented gaps of the
//! dimension-separated ranking: the substrate-side mapping between a vector
//! index's identifiers and the episode digests the constitutional query
//! answers, and the production vector-index adapter that uses it.
//!
//! What they hold:
//!
//! * a bound digest comes back from a search, and the mapping is two-way;
//! * a binding never crosses a scope, and a cross-scope attempt is refused;
//! * a digest the scope's query did not admit is never bound, and the
//!   admission set is **sealed**: it can only be minted by the store's own
//!   constitutional query, so a hand-built answer cannot become one;
//! * the binding index survives a close and a reopen, and a truncated or a
//!   half-written file is corruption rather than a shorter index;
//! * the index takes a writer lock, two snapshots of one file do not lose
//!   each other's bindings, and a leftover temporary file is inert;
//! * the loader refuses a file the writer itself could never produce;
//! * opening the index beside a store never changes the store directory's
//!   mode;
//! * the approximate HNSW source agrees with the exact oracle on a trivially
//!   separable corpus, and a pinned seed makes it reproducible;
//! * the merged ranking carries the digest of a vector hit, keeps the eight
//!   dimensions separately inspectable, and exposes no combined scalar;
//! * a vector hit with no binding is counted, never invented into the
//!   ranking;
//! * the whole path works against a real episode store.

#[path = "support/dimension_ranking.rs"]
#[allow(dead_code)]
mod support;

use abi_wdbx::v3::episode::{ConstitutionalQuery, EvidenceDimension, MemoryResults};
use abi_wdbx::v3::ranking::{
    ExactVectorSource, HnswVectorSource, MergedRanking, VectorArm, VectorBindingError,
    VectorBindingIndex, VectorHit, VectorSource, admitted_scope, rank_with_vector_source,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use support::{GUILD, build_fixture, public_fields};

/// The pinned seed of the HNSW graph's level sampling.
const PINNED_SEED: u64 = 0x00c0_ffee_1234_5678;

/// The ranking module's own sources, for the structural claim that no public
/// ranking type carries an opaque scalar.
const RANKING_SOURCES: &[&str] = &[
    include_str!("../src/v3/ranking/mod.rs"),
    include_str!("../src/v3/ranking/ordering.rs"),
    include_str!("../src/v3/ranking/signals.rs"),
    include_str!("../src/v3/ranking/merge.rs"),
    include_str!("../src/v3/ranking/vector.rs"),
    include_str!("../src/v3/ranking/vector/format.rs"),
    include_str!("../src/v3/ranking/vector/lock.rs"),
    include_str!("../src/v3/ranking/adapter.rs"),
];

/// The corpus the oracle comparison uses: two trivially separable clusters.
const CORPUS: [[f32; 4]; 8] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.99, 0.05, 0.0, 0.0],
    [0.98, 0.0, 0.1, 0.0],
    [0.97, 0.0, 0.0, 0.1],
    [0.0, 1.0, 0.0, 0.0],
    [0.05, 0.99, 0.0, 0.0],
    [0.0, 0.98, 0.1, 0.0],
    [0.0, 0.97, 0.0, 0.1],
];

/// The query the corpus is searched with.
const CORPUS_QUERY: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

/// The body of one `impl` block, by brace counting.
fn impl_block(source: &str, header: &str) -> String {
    let start = source.find(header).expect("the impl block exists");
    let body = &source[start + header.len()..];
    let mut depth = 1;
    for (index, byte) in body.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return body[..index].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("the impl block is not closed");
}

/// The checksum the index computes over one binding line.
fn line_checksum(scope: &str, vector_id: u64, digest: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(u64::try_from(scope.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(scope.as_bytes());
    hasher.update(vector_id.to_le_bytes());
    hasher.update(digest);
    hasher.finalize().into()
}

/// The checksum the index computes over a whole record set.
fn record_set_checksum(records: &[(String, u64, [u8; 32])]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for (scope, vector_id, digest) in records {
        hasher.update(line_checksum(scope, *vector_id, digest));
    }
    hasher.finalize().into()
}

/// One binding line, as the writer writes it.
fn binding_line(scope: &str, vector_id: u64, digest: &[u8; 32]) -> String {
    serde_json::json!({
        "scope": scope,
        "vector_id": vector_id,
        "episode_digest": digest,
        "checksum": line_checksum(scope, vector_id, digest),
    })
    .to_string()
}

/// The header line, as the writer writes it.
fn header_line(records: &[(String, u64, [u8; 32])]) -> String {
    serde_json::json!({
        "header": {
            "version": 1,
            "records": records.len(),
            "checksum": record_set_checksum(records),
        }
    })
    .to_string()
}

#[test]
fn v3_vector_adapter_round_trips_a_digest_through_the_vector_index() {
    let scratch = support::Scratch::new();
    let scope = "guild_round_trip";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[1])]);
    let digest = digests[scope][0];
    let admitted = support::admitted_for(&store, scope);

    let bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    let mut source =
        HnswVectorSource::new(3, PINNED_SEED, bindings, scope).expect("the source opens");

    let vector_id = source
        .insert_bound(digest, &[1.0, 0.0, 0.0], &admitted)
        .expect("the vector binds");
    assert_eq!(vector_id, 1, "the source assigns dense identifiers");

    // The search answers with the digest the vector was bound to.
    let arm = source
        .search(&[1.0, 0.0, 0.0], 4)
        .expect("the search answers");
    assert_eq!(arm.hits.len(), 1);
    assert_eq!(arm.hits[0].episode_digest, digest);
    assert_eq!(arm.hits_without_binding, 0);

    // The mapping is two-way.
    assert_eq!(source.bindings().digest_of(scope, vector_id), Some(digest));
    assert_eq!(
        source.bindings().vector_id_of(scope, &digest),
        Some(vector_id)
    );

    // The exact source answers the same way over the same binding.
    let mut exact = ExactVectorSource::new(source.bindings().clone(), scope);
    exact
        .insert_bound(digest, &[1.0, 0.0, 0.0], &admitted)
        .expect("the vector binds");
    let arm = exact
        .search(&[1.0, 0.0, 0.0], 4)
        .expect("the search answers");
    assert_eq!(arm.hits[0].episode_digest, digest);
}

#[test]
fn v3_vector_adapter_isolates_scopes() {
    let scratch = support::Scratch::new();
    let scope_a = "guild_scope_a";
    let scope_b = "guild_scope_b";
    let (store, digests) = support::guilds(&scratch, &[(scope_a, &[1]), (scope_b, &[2])]);
    let digest_a = digests[scope_a][0];
    let digest_b = digests[scope_b][0];
    let admitted_a = support::admitted_for(&store, scope_a);
    let admitted_b = support::admitted_for(&store, scope_b);

    let mut bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    bindings
        .bind(scope_a, 1, digest_a, &admitted_a)
        .expect("the binding commits");

    // A lookup in one scope never answers with another scope's digest.
    assert_eq!(bindings.digest_of(scope_a, 1), Some(digest_a));
    assert_eq!(bindings.digest_of(scope_b, 1), None);
    assert_eq!(bindings.vector_id_of(scope_b, &digest_a), None);
    assert_eq!(bindings.vector_id_of(scope_a, &digest_b), None);

    // A cross-scope binding attempt is refused twice over: the admission set
    // belongs to another scope, and the digest is not admitted in this one.
    assert_eq!(
        bindings.bind(scope_b, 1, digest_a, &admitted_a),
        Err(VectorBindingError::ScopeMismatch)
    );
    assert_eq!(
        bindings.bind(scope_b, 2, digest_a, &admitted_b),
        Err(VectorBindingError::NotAdmitted)
    );
    assert!(
        bindings.bindings_in(scope_b).next().is_none(),
        "a refused cross-scope binding writes nothing"
    );

    // The other scope's own digest binds in the other scope.
    bindings
        .bind(scope_b, 3, digest_b, &admitted_b)
        .expect("the second scope binds its own digest");
    assert_eq!(bindings.digest_of(scope_b, 3), Some(digest_b));

    // A source scoped to one guild never returns another guild's digest,
    // even when its graph holds the very vector the other scope bound.
    let mut source =
        HnswVectorSource::new(3, PINNED_SEED, bindings, scope_b).expect("the source opens");
    source
        .insert_unbound(&[1.0, 0.0, 0.0])
        .expect("the vector indexes");
    let arm = source
        .search(&[1.0, 0.0, 0.0], 4)
        .expect("the search answers");
    assert!(
        arm.hits.is_empty(),
        "another scope's digest is never returned"
    );
    assert_eq!(arm.hits_without_binding, 1);
}

#[test]
fn v3_vector_adapter_refuses_a_digest_that_is_not_admitted() {
    let scratch = support::Scratch::new();
    let scope = "guild_admission";
    let elsewhere = "guild_elsewhere";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[3, 5]), (elsewhere, &[4])]);
    let admitted_digest = digests[scope][0];
    let second_digest = digests[scope][1];
    // A real digest of the ledger, but of another guild: the scope's own
    // constitutional query did not admit it.
    let stranger = digests[elsewhere][0];
    let admitted = support::admitted_for(&store, scope);

    let mut bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");

    // A digest the scope's constitutional query did not answer is refused.
    assert_eq!(
        bindings.bind(scope, 1, stranger, &admitted),
        Err(VectorBindingError::NotAdmitted)
    );
    assert!(
        bindings.is_empty(),
        "a refused binding writes nothing at all"
    );

    // The admitted digest binds.
    let receipt = bindings
        .bind(scope, 1, admitted_digest, &admitted)
        .expect("the binding commits");
    assert_eq!(receipt.vector_id, 1);
    assert_eq!(receipt.episode_digest, admitted_digest);
    assert!(receipt.written);
    assert_eq!(bindings.len(), 1);

    // Re-binding the identical pair writes nothing.
    let again = bindings
        .bind(scope, 1, admitted_digest, &admitted)
        .expect("the binding is idempotent");
    assert!(!again.written);
    assert_eq!(bindings.len(), 1);

    // A second identifier for the same digest, and a second digest for the
    // same identifier, are both refused: the mapping is two-way.
    assert_eq!(
        bindings.bind(scope, 2, admitted_digest, &admitted),
        Err(VectorBindingError::DigestConflict)
    );
    assert_eq!(
        bindings.bind(scope, 1, second_digest, &admitted),
        Err(VectorBindingError::IdConflict)
    );

    // The admission set belongs to its scope.
    let other = support::admitted_for(&store, elsewhere);
    assert_eq!(
        bindings.bind(scope, 3, admitted_digest, &other),
        Err(VectorBindingError::ScopeMismatch)
    );
}

#[test]
fn v3_vector_adapter_builds_the_admission_set_from_the_canonical_answer() {
    let fixture = build_fixture();
    let store = fixture.store();
    let query = ConstitutionalQuery::new(GUILD, 64);

    // The set is minted by the store's own query, and it carries the scope
    // the query was asked for.
    let admitted = store
        .admitted_scope(&query)
        .expect("the canonical query answers");
    assert_eq!(admitted.scope(), GUILD);
    assert!(admitted.contains(&fixture.digest("signed")));
    assert!(
        !admitted.contains(&fixture.digest("erased")),
        "a forgotten record is not in the default answer, so it is not bindable"
    );

    // The adapter's helper goes through the same path, and mints the same
    // set.
    let through_adapter = admitted_scope(&store, &query).expect("the canonical query answers");
    assert_eq!(through_adapter, admitted);
}

/// The admission set is sealed: it has no public constructor, so a hand-built
/// answer cannot become one by any public path.
#[test]
fn v3_vector_adapter_seals_the_admission_set() {
    let source = include_str!("../src/v3/ranking/vector.rs");
    let block = impl_block(source, "impl AdmittedScope {");
    let public: Vec<&str> = block
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub fn "))
        .filter_map(|rest| rest.split('(').next())
        .collect();
    assert_eq!(
        public,
        vec!["scope", "len", "is_empty", "contains"],
        "the admission set exposes accessors and no constructor"
    );
    assert!(
        !source.contains("pub fn from_results"),
        "the constructor that reads an answer is not public"
    );
    assert!(
        !source.contains("impl Default for AdmittedScope"),
        "the admission set has no public default"
    );
    assert!(
        source.contains("pub fn admitted_scope"),
        "the store mints the set itself, from its own query"
    );
}

/// A forged admission set is not accepted through any public path.
///
/// The read model's answer type stays public, so a caller can hand-build an
/// answer naming any digest in any scope. It cannot become an admission set:
/// the set is sealed, and the only public way to have one is the store's own
/// constitutional query.
#[test]
fn v3_vector_adapter_refuses_an_externally_forged_admission_set() {
    let scratch = support::Scratch::new();
    let scope = "guild_forged";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[7])]);
    let digest = digests[scope][0];
    let stranger = [0xAB_u8; 32];

    // A hand-built answer: any digest, in any scope.
    let mut answer = store
        .query(&ConstitutionalQuery::new(scope, 64))
        .expect("the canonical query answers");
    let template = answer.results.remove(0);
    let mut forged = template.clone();
    forged.episode_digest = stranger;
    forged.receipt.guild_ref = "guild_elsewhere".to_string();
    forged.receipt.episode_digest = stranger;
    let forged_answer = MemoryResults {
        scope: "guild_elsewhere".to_string(),
        results: vec![forged],
        disposition_counts: BTreeMap::new(),
    };
    assert_eq!(forged_answer.scope, "guild_elsewhere");
    assert_eq!(forged_answer.results[0].episode_digest, stranger);

    // The set the store mints holds what the store's own answer holds, and
    // nothing else: the hand-built answer is never consulted.
    let admitted = support::admitted_for(&store, scope);
    assert!(admitted.contains(&digest));
    assert!(
        !admitted.contains(&stranger),
        "a hand-built digest is never admitted"
    );

    // The forged digest is refused by every public path.
    let mut bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    assert_eq!(
        bindings.bind(scope, 1, stranger, &admitted),
        Err(VectorBindingError::NotAdmitted),
        "a digest the store never admitted is refused"
    );
    assert!(
        bindings.is_empty(),
        "a refused binding writes nothing at all"
    );

    // And the set belongs to the scope it was minted for: it cannot be
    // presented as another scope's answer.
    assert_eq!(
        bindings.bind("guild_elsewhere", 1, digest, &admitted),
        Err(VectorBindingError::ScopeMismatch)
    );
    assert!(
        bindings.is_empty(),
        "a refused binding writes nothing at all"
    );
}

#[test]
fn v3_vector_adapter_binding_survives_reopen_and_refuses_corruption() {
    let scratch = support::Scratch::new();
    let scope = "guild_durability";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[5, 6])]);
    let first = digests[scope][0];
    let second = digests[scope][1];
    let admitted = support::admitted_for(&store, scope);

    let path = {
        let mut bindings =
            VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
        bindings
            .bind(scope, 1, first, &admitted)
            .expect("the binding commits");
        bindings
            .bind(scope, 2, second, &admitted)
            .expect("the binding commits");
        bindings.path().to_path_buf()
    };

    // The binding survives a close and a reopen.
    {
        let reopened = VectorBindingIndex::open(&path).expect("the index reopens");
        assert_eq!(reopened.len(), 2);
        assert_eq!(reopened.digest_of(scope, 1), Some(first));
        assert_eq!(reopened.digest_of(scope, 2), Some(second));
        assert_eq!(reopened.vector_id_of(scope, &second), Some(2));
    }

    // An incomplete final line is damage, not a crash tail: the index is
    // replaced whole, so a fragment is corruption rather than a trim.
    let valid = std::fs::read(&path).expect("the index file reads");
    let mut fragment = valid.clone();
    fragment.extend_from_slice(b"{\"scope\":\"guild_durability\",\"vector_id\":3");
    std::fs::write(&path, &fragment).expect("the truncated line is written");
    assert_eq!(
        VectorBindingIndex::open(&path).err(),
        Some(VectorBindingError::Corrupt),
        "a trailing fragment is corruption"
    );

    // A truncation at a line boundary removes whole records with no visible
    // trace, so the header's record count is what detects it.
    let mut lines = valid.split_inclusive(|byte| *byte == b'\n');
    let header: Vec<u8> = lines.next().expect("the header line").to_vec();
    let first_record: Vec<u8> = lines.next().expect("a record line").to_vec();
    let truncated: Vec<u8> = [header, first_record].concat();
    std::fs::write(&path, &truncated).expect("the truncated index is written");
    assert_eq!(
        VectorBindingIndex::open(&path).err(),
        Some(VectorBindingError::Corrupt),
        "a truncation at a line boundary is corruption"
    );

    // A corrupted checksum is corruption: the index refuses to open rather
    // than believe a mutated line.
    let mut corrupted = String::new();
    for line in valid.split_inclusive(|byte| *byte == b'\n').skip(1) {
        let mut value: serde_json::Value = serde_json::from_slice(line).expect("the line parses");
        value["checksum"] = serde_json::json!(vec![0_u8; 32]);
        corrupted.push_str(&value.to_string());
        corrupted.push('\n');
    }
    std::fs::write(&path, corrupted).expect("the corrupted index is written");
    assert_eq!(
        VectorBindingIndex::open(&path).err(),
        Some(VectorBindingError::Corrupt)
    );
}

#[test]
fn v3_vector_adapter_refuses_a_file_the_writer_cannot_produce() {
    let scratch = support::Scratch::new();
    std::fs::create_dir_all(scratch.path()).expect("the directory is created");
    let digest = [11_u8; 32];

    // Two identifiers bound to one digest: the writer refuses it, so the
    // loader refuses it too.
    let records = vec![
        ("guild_state".to_string(), 1_u64, digest),
        ("guild_state".to_string(), 2_u64, digest),
    ];
    let mut file = header_line(&records);
    for (scope, vector_id, bound) in &records {
        file.push('\n');
        file.push_str(&binding_line(scope, *vector_id, bound));
    }
    file.push('\n');
    let path = scratch.path().join("digest-conflict.jsonl");
    std::fs::write(&path, file).expect("the file is written");
    assert_eq!(
        VectorBindingIndex::open(&path).err(),
        Some(VectorBindingError::Corrupt),
        "the loader accepts exactly the states the writer can produce"
    );

    // A scope that is not a bounded identifier.
    let records = vec![(String::new(), 1_u64, digest)];
    let mut file = header_line(&records);
    for (scope, vector_id, bound) in &records {
        file.push('\n');
        file.push_str(&binding_line(scope, *vector_id, bound));
    }
    file.push('\n');
    let path = scratch.path().join("empty-scope.jsonl");
    std::fs::write(&path, file).expect("the file is written");
    assert_eq!(
        VectorBindingIndex::open(&path).err(),
        Some(VectorBindingError::Corrupt),
        "an unbounded scope is corruption"
    );

    // A scope that carries a control character.
    let records = vec![("guild\u{0007}scope".to_string(), 1_u64, digest)];
    let mut file = header_line(&records);
    for (scope, vector_id, bound) in &records {
        file.push('\n');
        file.push_str(&binding_line(scope, *vector_id, bound));
    }
    file.push('\n');
    let path = scratch.path().join("control-scope.jsonl");
    std::fs::write(&path, file).expect("the file is written");
    assert_eq!(
        VectorBindingIndex::open(&path).err(),
        Some(VectorBindingError::Corrupt),
        "a control character in a scope is corruption"
    );

    // A well-formed file of the writer's own shape opens.
    let records = vec![("guild_state".to_string(), 1_u64, digest)];
    let mut file = header_line(&records);
    for (scope, vector_id, bound) in &records {
        file.push('\n');
        file.push_str(&binding_line(scope, *vector_id, bound));
    }
    file.push('\n');
    let path = scratch.path().join("well-formed.jsonl");
    std::fs::write(&path, file).expect("the file is written");
    let opened = VectorBindingIndex::open(&path).expect("the index opens");
    assert_eq!(opened.digest_of("guild_state", 1), Some(digest));
}

#[test]
fn v3_vector_adapter_refuses_a_second_writer_on_the_same_directory() {
    let scratch = support::Scratch::new();
    let first = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    assert_eq!(
        VectorBindingIndex::open_beside(scratch.path()).err(),
        Some(VectorBindingError::WriterBusy),
        "a second writer on one index is refused, not raced"
    );
    // The first writer still holds the index.
    assert_eq!(first.len(), 0);
    drop(first);
    // Once the first writer is gone, the next writer takes the index.
    let second = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    assert_eq!(second.len(), 0);
}

#[test]
fn v3_vector_adapter_keeps_both_snapshots_bindings_when_both_write() {
    let scratch = support::Scratch::new();
    let scope = "guild_merge";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[21, 22])]);
    let first = digests[scope][0];
    let second = digests[scope][1];
    let admitted = support::admitted_for(&store, scope);

    let path = {
        let bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
        let path = bindings.path().to_path_buf();
        // Two snapshots of one file: each holds its own view of the bindings,
        // and each writes its own.
        let mut one = bindings.clone();
        let mut two = bindings.clone();
        one.bind(scope, 1, first, &admitted)
            .expect("the first binding commits");
        two.bind(scope, 2, second, &admitted)
            .expect("the second binding commits");
        path
    };

    // Neither write lost the other's binding.
    let reopened = VectorBindingIndex::open(&path).expect("the index reopens");
    assert_eq!(reopened.digest_of(scope, 1), Some(first));
    assert_eq!(reopened.digest_of(scope, 2), Some(second));
    assert_eq!(reopened.len(), 2);
}

#[test]
fn v3_vector_adapter_ignores_a_leftover_temporary_file() {
    let scratch = support::Scratch::new();
    let scope = "guild_temporary";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[31])]);
    let digest = digests[scope][0];
    let admitted = support::admitted_for(&store, scope);

    let path = {
        let mut bindings =
            VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
        bindings
            .bind(scope, 1, digest, &admitted)
            .expect("the binding commits");
        bindings.path().to_path_buf()
    };

    // An interrupted write leaves its own temporary file behind, under a name
    // no other writer can publish under.
    let leftover = scratch
        .path()
        .join(".episodes.v1.vectors.jsonl.999.0.0.tmp");
    std::fs::write(&leftover, b"{\"half\":").expect("the leftover is written");

    // The next open is unaffected: the index itself is whole.
    let reopened = VectorBindingIndex::open(&path).expect("the index reopens");
    assert_eq!(reopened.digest_of(scope, 1), Some(digest));
    assert_eq!(reopened.len(), 1);
}

#[test]
fn v3_vector_adapter_does_not_change_the_directory_mode() {
    let scratch = support::Scratch::new();
    let directory = scratch.path();
    std::fs::create_dir_all(directory).expect("the directory is created");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o755))
            .expect("the mode is set");
    }
    let bindings = VectorBindingIndex::open_beside(directory).expect("the index opens");
    assert!(bindings.is_empty());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(directory)
            .expect("the directory reads")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o755,
            "derived state never changes the mode of the directory it reads"
        );
    }
}

#[test]
fn v3_vector_adapter_agrees_with_the_exact_oracle_and_is_deterministic() {
    let scratch = support::Scratch::new();
    let scope = "guild_oracle";
    let (store, digests) = support::guilds(&scratch, &[(scope, &[1, 2, 3, 4, 5, 6, 7, 8])]);
    let digests = digests[scope].clone();
    let admitted = support::admitted_for(&store, scope);

    let bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    let mut approximate =
        HnswVectorSource::new(4, PINNED_SEED, bindings.clone(), scope).expect("the source opens");
    let mut exact = ExactVectorSource::new(bindings.clone(), scope);
    for (index, vector) in CORPUS.iter().enumerate() {
        approximate
            .insert_bound(digests[index], vector, &admitted)
            .expect("the vector binds");
        exact
            .insert_bound(digests[index], vector, &admitted)
            .expect("the vector binds");
    }

    // The approximate source is declared approximate; the oracle is not.
    assert!(approximate.is_approximate());
    assert!(!exact.is_approximate());

    let approximate_arm = approximate
        .search(&CORPUS_QUERY, CORPUS.len())
        .expect("the search answers");
    let exact_arm = exact
        .search(&CORPUS_QUERY, CORPUS.len())
        .expect("the search answers");
    assert_eq!(approximate_arm.hits.len(), CORPUS.len());
    assert_eq!(exact_arm.hits.len(), CORPUS.len());

    // The oracle agreement: on a trivially separable corpus the approximate
    // source answers the same order, and the same similarities.
    let approximate_order: Vec<[u8; 32]> = approximate_arm
        .hits
        .iter()
        .map(|hit| hit.episode_digest)
        .collect();
    let exact_order: Vec<[u8; 32]> = exact_arm
        .hits
        .iter()
        .map(|hit| hit.episode_digest)
        .collect();
    assert_eq!(
        approximate_order, exact_order,
        "the approximate source must agree with the exact oracle"
    );
    for (approximate_hit, exact_hit) in approximate_arm.hits.iter().zip(exact_arm.hits.iter()) {
        assert!(
            (approximate_hit.similarity - exact_hit.similarity).abs() < 1e-5,
            "the two indexes report the same similarity"
        );
    }

    // A pinned seed makes the graph reproducible: two runs built the same
    // way answer the same search the same way.
    let mut first =
        HnswVectorSource::new(4, PINNED_SEED, bindings.clone(), scope).expect("the source opens");
    let mut second =
        HnswVectorSource::new(4, PINNED_SEED, bindings.clone(), scope).expect("the source opens");
    assert_eq!(first.seed(), PINNED_SEED);
    assert_eq!(second.seed(), PINNED_SEED);
    for (index, vector) in CORPUS.iter().enumerate() {
        first
            .insert_bound(digests[index], vector, &admitted)
            .expect("the vector binds");
        second
            .insert_bound(digests[index], vector, &admitted)
            .expect("the vector binds");
    }
    let one: VectorArm = first
        .search(&CORPUS_QUERY, CORPUS.len())
        .expect("the search answers");
    let two: VectorArm = second
        .search(&CORPUS_QUERY, CORPUS.len())
        .expect("the search answers");
    assert_eq!(one, two, "a pinned seed makes the graph deterministic");
    assert_eq!(one.hits, approximate_arm.hits);
}

#[test]
fn v3_vector_adapter_merged_ranking_carries_the_digest_and_keeps_the_dimensions_apart() {
    let fixture = build_fixture();
    let store = fixture.store();
    let query = ConstitutionalQuery::new(GUILD, 64);
    let admitted = admitted_scope(&store, &query).expect("the canonical query answers");

    let scratch = support::Scratch::new();
    let bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    let mut source =
        HnswVectorSource::new(4, PINNED_SEED, bindings, GUILD).expect("the source opens");
    let signed = fixture.digest("signed");
    let unsigned = fixture.digest("unsigned");
    source
        .insert_bound(signed, &[1.0, 0.0, 0.0, 0.0], &admitted)
        .expect("the vector binds");
    source
        .insert_bound(unsigned, &[0.9, 0.1, 0.0, 0.0], &admitted)
        .expect("the vector binds");

    let merged: MergedRanking =
        rank_with_vector_source(&store, &query, &source, &[1.0, 0.0, 0.0, 0.0], 8)
            .expect("the ranking answers");

    // The merged ranking carries the digest of every vector hit.
    let signed_entry = merged.ranking.result(signed).expect("the record is ranked");
    assert_eq!(signed_entry.result.episode_digest, signed);
    assert_eq!(signed_entry.vector_rank, Some(1));
    assert!(signed_entry.vector_similarity.is_some());
    let unsigned_entry = merged
        .ranking
        .result(unsigned)
        .expect("the record is ranked");
    assert_eq!(unsigned_entry.result.episode_digest, unsigned);
    assert_eq!(unsigned_entry.vector_rank, Some(2));

    // The eight dimensions stay separately inspectable on every entry.
    for entry in merged.ranking.entries() {
        for dimension in EvidenceDimension::all() {
            assert!(
                !entry.dimension_signals.get(dimension).label().is_empty(),
                "{dimension:?} must carry a named value"
            );
        }
    }

    // The structural claim, checked at the source: the ranking's own public
    // surface — the adapter and the binding index included — exposes no
    // number that could stand in for the eight dimensions.
    for source_text in RANKING_SOURCES {
        for forbidden in ["fn score(", "fn combined(", "fn trust(", "fn rank("] {
            assert!(
                !source_text.contains(forbidden),
                "{forbidden} appears in the ranking module"
            );
        }
        for (name, ty) in public_fields(source_text) {
            for forbidden in ["score", "rank", "trust", "combined", "weight", "factor"] {
                assert_ne!(
                    name, forbidden,
                    "the ranking exposes a field named {forbidden}"
                );
            }
            if ty.contains("f32") || ty.contains("f64") {
                assert!(
                    name.contains("similarity"),
                    "{name} is a floating-point field that is not a similarity"
                );
            }
        }
    }
    let mut float_fields = Vec::new();
    for source_text in RANKING_SOURCES {
        for (name, ty) in public_fields(source_text) {
            if ty.contains("f32") || ty.contains("f64") {
                float_fields.push(name);
            }
        }
    }
    float_fields.sort_unstable();
    assert_eq!(
        float_fields,
        vec!["similarity".to_string(), "vector_similarity".to_string()],
        "the ranking must expose no number but the vector arm's own similarity"
    );
}

#[test]
fn v3_vector_adapter_counts_an_unbound_hit_and_never_invents_it() {
    let fixture = build_fixture();
    let store = fixture.store();
    let query = ConstitutionalQuery::new(GUILD, 64);
    let admitted = admitted_scope(&store, &query).expect("the canonical query answers");

    let scratch = support::Scratch::new();
    let bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    let mut source =
        HnswVectorSource::new(4, PINNED_SEED, bindings, GUILD).expect("the source opens");
    let signed = fixture.digest("signed");
    source
        .insert_bound(signed, &[1.0, 0.0, 0.0, 0.0], &admitted)
        .expect("the vector binds");
    // A vector whose episode is not bound: the arm counts the hit and never
    // names an episode for it.
    source
        .insert_unbound(&[0.0, 1.0, 0.0, 0.0])
        .expect("the vector indexes");

    let arm = source
        .search(&[1.0, 0.0, 0.0, 0.0], 8)
        .expect("the search answers");
    assert_eq!(arm.hits.len(), 1, "only the bound hit is resolved");
    assert_eq!(arm.hits_without_binding, 1, "the unbound hit is counted");

    let merged = rank_with_vector_source(&store, &query, &source, &[1.0, 0.0, 0.0, 0.0], 8)
        .expect("the ranking answers");
    assert_eq!(
        merged.vector_hits_without_binding, 1,
        "the merged ranking counts the hit it cannot name"
    );

    // The ranking's result count is unchanged: the unbound hit is never
    // invented into it.
    let canonical = store.query(&query).expect("the canonical query answers");
    assert_eq!(
        merged.ranking.len(),
        canonical.results.len(),
        "an unbound hit never adds a record to the ranking"
    );
    let ranked_by_the_arm = merged
        .ranking
        .entries()
        .iter()
        .filter(|entry| entry.vector_rank.is_some())
        .count();
    assert_eq!(
        ranked_by_the_arm, 1,
        "only the bound hit reaches the ranking"
    );
}

#[test]
fn v3_vector_adapter_ranks_end_to_end_against_a_real_store() {
    let fixture = build_fixture();
    let store = fixture.store();
    let query = ConstitutionalQuery::new(GUILD, 64);

    // The canonical read model answers first, and its answer is the only
    // thing a binding may name.
    let admitted = admitted_scope(&store, &query).expect("the canonical query answers");
    assert!(!admitted.is_empty());
    assert_eq!(admitted.scope(), GUILD);
    assert!(admitted.contains(&fixture.digest("signed")));
    assert!(
        !admitted.contains(&fixture.digest("erased")),
        "a forgotten record is not in the default answer, so it is not bindable"
    );

    let scratch = support::Scratch::new();
    let bindings = VectorBindingIndex::open_beside(scratch.path()).expect("the index opens");
    let mut source =
        HnswVectorSource::new(4, PINNED_SEED, bindings, GUILD).expect("the source opens");
    let signed = fixture.digest("signed");
    let proposal = fixture.digest("proposal");
    source
        .insert_bound(signed, &[1.0, 0.0, 0.0, 0.0], &admitted)
        .expect("the vector binds");
    source
        .insert_bound(proposal, &[0.0, 1.0, 0.0, 0.0], &admitted)
        .expect("the vector binds");

    let merged = rank_with_vector_source(&store, &query, &source, &[1.0, 0.0, 0.0, 0.0], 8)
        .expect("the ranking answers");

    // The merged ranking is the canonical answer, ranked, with the vector
    // arm's own reasoning beside it and nothing collapsed.
    let canonical = store.query(&query).expect("the canonical query answers");
    assert_eq!(merged.ranking.len(), canonical.results.len());
    for result in &canonical.results {
        let ranked = merged
            .ranking
            .result(result.episode_digest)
            .expect("every canonical result is ranked");
        assert_eq!(ranked.result.disposition, result.disposition);
        assert_eq!(ranked.result.dimensions, result.dimensions);
    }

    // The vector arm's own order survives the merge: sorting the merged
    // ranking by the arm's position reproduces the arm's order exactly.
    let arm_order: Vec<[u8; 32]> = vec![signed, proposal];
    let mut by_vector_rank: Vec<([u8; 32], Option<usize>)> = merged
        .ranking
        .entries()
        .iter()
        .map(|entry| (entry.result.episode_digest, entry.vector_rank))
        .collect();
    by_vector_rank.sort_by_key(|(_, rank)| rank.unwrap_or(usize::MAX));
    let recovered: Vec<[u8; 32]> = by_vector_rank
        .iter()
        .filter(|(_, rank)| rank.is_some())
        .map(|(digest, _)| *digest)
        .collect();
    assert_eq!(recovered, arm_order);

    // The merged ranking holds no record the canonical answer did not
    // answer, and the arm named no digest the answer does not hold.
    for entry in merged.ranking.entries() {
        assert!(
            canonical
                .results
                .iter()
                .any(|result| { result.episode_digest == entry.result.episode_digest })
        );
    }
    assert_eq!(merged.vector_hits_without_binding, 0);
    assert_eq!(merged.vector_hits_without_constitutional_result, 0);

    // The digest the arm carried is the digest the ledger committed: the
    // vector identifier is recoverable from the substrate's own mapping.
    let bindings = source.bindings();
    assert_eq!(bindings.vector_id_of(GUILD, &signed), Some(1));
    assert_eq!(bindings.vector_id_of(GUILD, &proposal), Some(2));
    let hits: Vec<VectorHit> = source
        .search(&[1.0, 0.0, 0.0, 0.0], 8)
        .expect("the search answers")
        .hits;
    assert_eq!(hits[0].episode_digest, signed);
}
