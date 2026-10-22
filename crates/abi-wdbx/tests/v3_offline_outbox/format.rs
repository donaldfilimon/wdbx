//! The outbox's durable file, its status surface, and its lack of expiry.

use super::*;
use abi_wdbx::v3::episode::EpisodeStore;
use abi_wdbx::v3::offline::{
    MAX_OPERATIONS, OUTBOX_FILE, OfflineOutbox, OperationKind, OperationStatus, OutboxError,
    OutboxOperation, chunk_digest,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// One operation line of the outbox file, as this test writes it.
///
/// The test writes the file itself so that a full outbox can be built
/// without queueing thousands of operations one at a time. The shape is the
/// file's shape: a checksummed line per operation.
#[derive(Serialize)]
struct FileLine {
    /// The operation the line holds.
    record: OutboxOperation,
    /// SHA-256 over the record's own canonical JSON bytes.
    checksum: [u8; 32],
}

/// The header of the outbox file, as this test writes it.
#[derive(Serialize)]
struct FileHeader {
    /// The format tag.
    format: String,
    /// The number of operation lines the file holds.
    count: u64,
    /// SHA-256 over the record checksums, in file order.
    records_checksum: [u8; 32],
}

/// The header line of the outbox file, as this test writes it.
#[derive(Serialize)]
struct FileHeaderLine {
    /// The header.
    header: FileHeader,
}

/// SHA-256 over some bytes.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    <[u8; 32]>::from(Sha256::digest(bytes))
}

/// Write an outbox file that holds `count` queued operations.
///
/// The file is written in the outbox's own format: a header line, then one
/// checksummed line per operation.
fn write_full_outbox(path: &std::path::Path, count: usize) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the outbox directory");
    }
    let mut body: Vec<u8> = Vec::new();
    let mut checksums: Vec<[u8; 32]> = Vec::with_capacity(count);
    for index in 0..count {
        let identifier = format!("op_{index}");
        let record = OutboxOperation {
            id: op_id(&identifier),
            depends_on: None,
            binding: binding(),
            kind: OperationKind::Write,
            payload: OperationPayload::Write(Box::new(write(
                &format!("req_{index}"),
                &identifier,
                1,
            ))),
            status: OperationStatus::Pending,
            reason: None,
            conflict: None,
            asset: None,
            enqueued_at_ms: 0,
            sequence: u64::try_from(index).expect("a sequence number"),
        };
        let checksum = sha256(&serde_json::to_vec(&record).expect("serialize the record"));
        let mut bytes =
            serde_json::to_vec(&FileLine { record, checksum }).expect("serialize the line");
        bytes.push(b'\n');
        body.extend_from_slice(&bytes);
        checksums.push(checksum);
    }
    let header = FileHeaderLine {
        header: FileHeader {
            format: "outbox.v1".into(),
            count: u64::try_from(count).expect("a count"),
            records_checksum: sha256(&checksums.concat()),
        },
    };
    let mut whole = serde_json::to_vec(&header).expect("serialize the header");
    whole.push(b'\n');
    whole.extend_from_slice(&body);
    fs::write(path, whole).expect("write the outbox file");
}

/// An operation survives an arbitrary elapsed time: there is no fixed
/// offline expiry.
#[test]
fn an_operation_survives_an_arbitrary_elapsed_time() {
    let scratch = Scratch::new("no-expiry");
    let directory = scratch.child("outbox");
    {
        let mut outbox =
            OfflineOutbox::open_with_clock(&directory, Box::new(|| 1_000)).expect("open outbox");
        outbox
            .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
            .expect("enqueue");
    }
    // Four billion milliseconds past the epoch is the year 2100.
    let far_future = 4_102_444_800_000_u64;
    let outbox =
        OfflineOutbox::open_with_clock(&directory, Box::new(move || far_future)).expect("reopen");
    let status = outbox.status();
    assert_eq!(status.pending, 1);
    assert_eq!(status.total(), 1);
    let view = outbox.operation(&op_id("op_1")).expect("operation view");
    assert_eq!(view.status, OperationStatus::Pending);
    assert_eq!(view.reason, None);
}

/// The outbox is not a canonical writer: the canonical store holds exactly
/// the writes that were authorized and bound, and the outbox holds no
/// canonical episode digest.
#[test]
fn the_outbox_is_not_a_canonical_writer() {
    let scratch = Scratch::new("not-writer");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = policy();
    let mut store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");
    // A second operation was written under a contract revision the server no
    // longer serves, so the current policy refuses it.
    let mut skewed = binding();
    skewed.contract_revision = REVISION + 1;
    let mut proposal = write("req_2", "op_2", 2);
    proposal.contract_revision = REVISION + 1;
    outbox
        .enqueue(NewOperation {
            id: op_id("op_2"),
            depends_on: None,
            binding: skewed,
            payload: OperationPayload::Write(Box::new(proposal)),
        })
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(report.submitted, 1);
    assert_eq!(report.accepted, 1);

    // The sync agent routes what the sink was given to the canonical writer.
    let mut bound = Vec::new();
    for submission in &sink.submissions {
        if let OperationPayload::Write(proposal) = &submission.payload {
            let receipt = store.propose_write(proposal).expect("canonical append");
            bound.push(receipt.episode_digest);
        }
    }
    assert_eq!(bound.len(), 1);
    let receipts = store.retrieve(SCOPE, 64).expect("receipts");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].episode_digest, bound[0]);

    // The outbox holds no canonical episode digest: not the bound one, and
    // not the one the canonical store would compute for the refused
    // operation either.
    let bytes = fs::read(outbox.path()).expect("outbox file");
    assert!(!contains(&bytes, &bound[0]));
    assert!(!contains(&bytes, hex(&bound[0]).as_bytes()));
    assert!(!contains(&bytes, b"episode_digest"));
    // The refused operation is preserved, and it bound nothing.
    assert_eq!(outbox.status().rejected, 1);
    assert_eq!(outbox.status().acknowledged, 1);
}

/// A corrupted sidecar is refused.
#[test]
fn a_corrupted_sidecar_is_refused() {
    let scratch = Scratch::new("corrupt");
    let directory = scratch.child("outbox");
    {
        let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
        outbox
            .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
            .expect("enqueue");
        outbox
            .enqueue(write_operation("op_2", write("req_2", "op_2", 2)))
            .expect("enqueue");
    }
    let path = directory.join(OUTBOX_FILE);
    let pristine = fs::read(&path).expect("outbox file");

    // A complete line whose checksum does not match is corruption.
    let mut corrupted = pristine.clone();
    let middle = corrupted.len() / 2;
    corrupted[middle] = corrupted[middle].wrapping_add(1);
    fs::write(&path, &corrupted).expect("write corrupted file");
    assert!(matches!(
        OfflineOutbox::open(&directory),
        Err(OutboxError::Corrupt)
    ));
}

/// The file's header names the operations it holds, so a truncation is
/// detectable.
#[test]
fn the_file_carries_a_header_that_names_its_operations() {
    let scratch = Scratch::new("header");
    let directory = scratch.child("outbox");
    {
        let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
        outbox
            .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
            .expect("enqueue");
        outbox
            .enqueue(write_operation("op_2", write("req_2", "op_2", 2)))
            .expect("enqueue");
    }
    let bytes = fs::read(directory.join(OUTBOX_FILE)).expect("outbox file");
    let first_line = bytes.split(|byte| *byte == b'\n').next().expect("a header");
    let header: serde_json::Value = serde_json::from_slice(first_line).expect("a header line");
    assert_eq!(header["header"]["format"], "outbox.v1");
    assert_eq!(header["header"]["count"], 2);
    assert!(header["header"]["records_checksum"].is_array());
}

/// A file truncated at a line boundary is refused as corrupt: the outbox
/// never opens with fewer operations than were queued.
#[test]
fn a_truncated_file_is_refused_as_corrupt() {
    let scratch = Scratch::new("truncated");
    let directory = scratch.child("outbox");
    {
        let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
        outbox
            .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
            .expect("enqueue");
        outbox
            .enqueue(write_operation("op_2", write("req_2", "op_2", 2)))
            .expect("enqueue");
    }
    let path = directory.join(OUTBOX_FILE);
    let pristine = fs::read(&path).expect("outbox file");
    let newlines: Vec<usize> = pristine
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .map(|(index, _)| index)
        .collect();
    // The header, the first record, and the second record.
    assert_eq!(newlines.len(), 3);

    // The last record is cut away at the line boundary. The file is still
    // whole lines, and the header still names two operations.
    fs::write(&path, &pristine[..=newlines[1]]).expect("write truncated file");
    assert!(matches!(
        OfflineOutbox::open(&directory),
        Err(OutboxError::Corrupt)
    ));

    // The header itself is cut away.
    fs::write(&path, &pristine[..newlines[0] / 2]).expect("write truncated file");
    assert!(matches!(
        OfflineOutbox::open(&directory),
        Err(OutboxError::Corrupt)
    ));
}

/// A trailing fragment is refused as corrupt. It is damage, not a crash
/// tail: the outbox replaces the whole file atomically, so its own code path
/// can never leave one behind.
#[test]
fn a_trailing_fragment_is_refused_as_corrupt() {
    let scratch = Scratch::new("fragment");
    let directory = scratch.child("outbox");
    {
        let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
        outbox
            .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
            .expect("enqueue");
        outbox
            .enqueue(write_operation("op_2", write("req_2", "op_2", 2)))
            .expect("enqueue");
    }
    let path = directory.join(OUTBOX_FILE);
    let pristine = fs::read(&path).expect("outbox file");

    // A fragment after the last whole line.
    let mut fragmented = pristine.clone();
    fragmented.extend_from_slice(b"{\"record\":{\"id\":\"op_3\"");
    fs::write(&path, &fragmented).expect("write fragmented file");
    assert!(matches!(
        OfflineOutbox::open(&directory),
        Err(OutboxError::Corrupt)
    ));

    // A file that does not end in a newline at all.
    fs::write(&path, &pristine[..pristine.len() - 1]).expect("write fragmented file");
    assert!(matches!(
        OfflineOutbox::open(&directory),
        Err(OutboxError::Corrupt)
    ));
}

/// A full outbox refuses new work with `Capacity`, which is distinct from an
/// invalid proposal.
#[test]
fn a_full_outbox_refuses_new_work_with_capacity() {
    let scratch = Scratch::new("capacity");
    let directory = scratch.child("outbox");
    write_full_outbox(&directory.join(OUTBOX_FILE), MAX_OPERATIONS);
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    assert_eq!(outbox.operations().len(), MAX_OPERATIONS);

    // The queue is full: the refusal says so, and says it is not the
    // proposal's fault.
    assert!(matches!(
        outbox.enqueue(write_operation("op_new", write("req_new", "op_new", 9))),
        Err(OutboxError::Capacity)
    ));

    // An invalid proposal is still an invalid proposal.
    let mut invalid = write("req_bad", "op_bad", 9);
    invalid.request_id = "not a bounded identifier".into();
    assert!(matches!(
        outbox.enqueue(write_operation("op_bad", invalid)),
        Err(OutboxError::InvalidInput)
    ));
}

/// The outbox's file and its status surface carry no episode content.
#[test]
fn the_outbox_carries_no_episode_content() {
    let scratch = Scratch::new("content");
    let directory = scratch.child("outbox");
    let marker = b"episode-content-marker-9f2c";
    let commitment = chunk_digest(marker);
    let mut proposal = write("req_1", "op_1", 1);
    let EpisodeEvent::MemoryCandidate { candidate, .. } = &mut proposal.event else {
        panic!("the proposal is a memory candidate");
    };
    candidate.payload_commitment = commitment;
    candidate.payload_bytes = u64::try_from(marker.len()).expect("length");

    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", proposal))
        .expect("enqueue");

    // The file holds the commitment, never the content.
    let bytes = fs::read(outbox.path()).expect("outbox file");
    assert!(!contains(&bytes, marker));
    assert!(!contains(&bytes, hex(&commitment).as_bytes()));
    assert!(!contains(&bytes, b"episode-content-marker"));

    // The status surface carries no proposal either.
    let view = outbox.operation(&op_id("op_1")).expect("operation view");
    let rendered = format!("{view:?}");
    assert!(!rendered.contains("episode-content-marker"));
    assert!(!rendered.contains("payload"));
    assert!(!rendered.contains(&hex(&commitment)));

    // Free text can never enter the file: the outbox bounds every
    // identifier a proposal carries.
    let mut free_text = write("req_2", "op_2", 2);
    let EpisodeEvent::MemoryCandidate { candidate, .. } = &mut free_text.event else {
        panic!("the proposal is a memory candidate");
    };
    candidate.embedding_version = Some("Episode Content Marker".into());
    assert!(matches!(
        outbox.enqueue(write_operation("op_2", free_text)),
        Err(OutboxError::InvalidInput)
    ));
    let after = fs::read(outbox.path()).expect("outbox file");
    assert!(!contains(&after, b"Episode Content Marker"));
}
