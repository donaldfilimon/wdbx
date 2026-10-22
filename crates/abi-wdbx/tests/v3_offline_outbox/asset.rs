//! The outbox's resumable asset transfer.

use super::*;
use abi_wdbx::v3::offline::{OfflineOutbox, OutboxError, asset_digest_of};

/// The first chunk of the test asset.
const FIRST: &[u8] = b"chunkzer";
/// The second chunk of the test asset.
const SECOND: &[u8] = b"chunkone";

/// An asset transfer resumes, and a corrupted chunk is detected.
#[test]
fn an_asset_transfer_resumes_and_verifies_its_digest() {
    let scratch = Scratch::new("asset");
    let directory = scratch.child("outbox");
    let proposal = AssetProposal {
        digest: asset_digest_of(&[(0, FIRST), (1, SECOND)]),
        length: 16,
        chunk_size: 8,
    };
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(asset_operation("op_asset", proposal))
        .expect("enqueue");
    assert_eq!(
        outbox
            .pending_asset_chunks(&op_id("op_asset"))
            .expect("pending"),
        vec![0, 1]
    );

    let outcome = outbox
        .supply_asset_chunk(&op_id("op_asset"), 0, FIRST)
        .expect("supply chunk 0");
    assert!(!outcome.already_transferred);
    assert_eq!(outcome.bytes, 8);
    assert_eq!(
        outbox
            .pending_asset_chunks(&op_id("op_asset"))
            .expect("pending"),
        vec![1]
    );

    // A resumed transfer does not re-request the completed chunk.
    drop(outbox);
    let mut outbox = OfflineOutbox::open(&directory).expect("reopen outbox");
    assert_eq!(
        outbox
            .pending_asset_chunks(&op_id("op_asset"))
            .expect("pending"),
        vec![1]
    );
    let progress = outbox.asset_state(&op_id("op_asset")).expect("asset state");
    assert_eq!(progress.completed_chunks, 1);
    assert_eq!(progress.total_chunks, 2);
    assert_eq!(progress.completed_bytes, 8);
    assert_eq!(progress.total_bytes, 16);

    outbox
        .supply_asset_chunk(&op_id("op_asset"), 1, SECOND)
        .expect("supply chunk 1");
    let verification = outbox.verify_asset(&op_id("op_asset")).expect("verify");
    assert!(verification.complete);
    assert!(verification.matches_declared);

    // The same chunk again is idempotent; a different one is refused.
    let repeat = outbox
        .supply_asset_chunk(&op_id("op_asset"), 0, FIRST)
        .expect("repeat chunk 0");
    assert!(repeat.already_transferred);
    assert!(matches!(
        outbox.supply_asset_chunk(&op_id("op_asset"), 0, b"corruptd"),
        Err(OutboxError::ChunkMismatch)
    ));
}

/// A corrupted chunk makes the completed asset fail its digest.
#[test]
fn a_corrupted_chunk_is_detected_by_the_asset_digest() {
    let scratch = Scratch::new("corrupt-chunk");
    let directory = scratch.child("outbox");
    let proposal = AssetProposal {
        digest: asset_digest_of(&[(0, FIRST), (1, SECOND)]),
        length: 16,
        chunk_size: 8,
    };
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(asset_operation("op_asset", proposal))
        .expect("enqueue");
    outbox
        .supply_asset_chunk(&op_id("op_asset"), 0, FIRST)
        .expect("supply chunk 0");
    outbox
        .supply_asset_chunk(&op_id("op_asset"), 1, b"corruptd")
        .expect("supply chunk 1");
    let verification = outbox.verify_asset(&op_id("op_asset")).expect("verify");
    assert!(verification.complete);
    assert!(!verification.matches_declared);
}

/// The order a caller holds its chunks in does not change the asset digest.
///
/// A caller that numbered its chunks by the order it happened to hold them
/// could compute a digest that disagrees with the outbox's, and a correctly
/// transferred asset would be reported as corrupted.
#[test]
fn the_chunk_order_does_not_change_the_asset_digest() {
    let scratch = Scratch::new("chunk-order");
    let directory = scratch.child("outbox");
    let proposal = AssetProposal {
        digest: asset_digest_of(&[(0, FIRST), (1, SECOND)]),
        length: 16,
        chunk_size: 8,
    };
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(asset_operation("op_asset", proposal))
        .expect("enqueue");

    // The chunks are transferred out of index order.
    outbox
        .supply_asset_chunk(&op_id("op_asset"), 1, SECOND)
        .expect("supply chunk 1");
    outbox
        .supply_asset_chunk(&op_id("op_asset"), 0, FIRST)
        .expect("supply chunk 0");
    let verification = outbox.verify_asset(&op_id("op_asset")).expect("verify");
    assert!(verification.complete);
    assert!(verification.matches_declared);

    // The same chunk set, handed over in another order, is the same digest.
    assert_eq!(
        asset_digest_of(&[(1, SECOND), (0, FIRST)]),
        asset_digest_of(&[(0, FIRST), (1, SECOND)])
    );
}
