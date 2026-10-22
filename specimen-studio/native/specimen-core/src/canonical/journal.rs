//! The studio's own canonical journal: the map from a canonical episode to
//! the studio's content.
//!
//! # Why the studio needs one
//!
//! The canonical ledger commits to a payload; it never stores one, and the
//! substrate exposes no read API that returns a committed record's payload
//! commitment. The studio therefore keeps its own map: for every episode it
//! recorded, the journal names the episode digest, the ledger sequence, the
//! content commitment the episode committed to, and the studio-side facts the
//! ledger cannot hold.
//!
//! The journal is a *derived* sidecar. It is not canonical authority: the
//! ledger is. Everything in the journal is either a digest the ledger also
//! holds, or a studio-side fact (the node a deletion removed, the instant it
//! happened) that the ledger never held and never could.
//!
//! # What the journal is not
//!
//! * It is not a second episode ledger. It holds no event, no actor, and no
//!   payload; it holds the map from the ledger's digests to the studio's own
//!   content.
//! * It is not a commitment. The studio never predicts an episode digest and
//!   never re-derives one; the substrate computes it.
//! * It is not required for the ledger to be read. A ledger episode the
//!   journal does not know is a *declared gap*, not a silent drop: the
//!   rebuild reports it.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// The key of the studio's canonical journal.
///
/// The journal is small and structured, so it lives in the studio's own v2
/// key/value store. The content the journal points at lives in the studio's
/// own content-addressed blob store, beside the other assets.
pub const JOURNAL_KEY: &str = "studio/canonical/journal";

/// A 32-byte digest, written as hex in the studio's own JSON.
///
/// The studio's JSON is its own; a hex digest is the same shape the studio's
/// assets already use, so a reader can compare a journal digest with an
/// asset digest without a conversion table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The all-zero digest.
    pub const ZERO: Digest = Digest([0; 32]);

    /// Wrap 32 raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Whether this is the all-zero digest.
    pub const fn is_zero(&self) -> bool {
        let mut index = 0;
        while index < 32 {
            if self.0[index] != 0 {
                return false;
            }
            index += 1;
        }
        true
    }

    /// The digest as lowercase hex.
    pub fn to_hex(self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in self.0 {
            hex.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
            hex.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
        }
        hex
    }

    /// Read a digest from lowercase hex.
    ///
    /// `None` when the text is not 64 hexadecimal digits: a digest that
    /// cannot be read is not a digest, and the studio never guesses one.
    pub fn parse_hex(text: &str) -> Option<Self> {
        if text.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (index, pair) in text.as_bytes().chunks(2).enumerate() {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            bytes[index] = ((high << 4) | low) as u8;
        }
        Some(Self(bytes))
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse_hex(&text).ok_or_else(|| D::Error::custom("not a 64-digit hex digest"))
    }
}

/// What kind of canonical episode a journal entry records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalKind {
    /// The durable specimen state.
    Edit,
    /// An erasure request.
    Forget,
    /// A learned proposal.
    Learn,
}

impl JournalKind {
    /// The stable label of the kind.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Edit => "edit",
            Self::Forget => "forget",
            Self::Learn => "learn",
        }
    }
}

/// One studio tombstone row: the node that disappeared, and when.
///
/// The ledger's tombstone names an *episode digest*, never a node and never
/// an instant. These rows are the studio's own facts about the erasure, and
/// the journal holds them beside the digest that caused them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TombstoneRow {
    /// The node as it was, whole.
    pub node: Value,
    /// The instant the studio recorded the erasure.
    pub deleted_at: String,
}

impl TombstoneRow {
    /// The row in the studio snapshot's own shape.
    pub fn to_value(&self) -> Value {
        serde_json::json!({ "node": self.node, "deletedAt": self.deleted_at })
    }
}

/// One canonical episode, as the studio recorded it.
///
/// Every digest here is a digest the substrate itself returned. The studio
/// never derives one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    /// The canonical episode digest the substrate returned.
    pub episode: Digest,
    /// The ledger sequence the substrate assigned.
    pub sequence: u64,
    /// The studio's idempotency key for the operation.
    pub request_id: String,
    /// What kind of episode this is.
    pub kind: JournalKind,
    /// The content commitment the episode commits to, when it commits to one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Digest>,
    /// The candidate this episode supersedes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<Digest>,
    /// The candidate this episode erases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forgets: Option<Digest>,
    /// The proposal this episode records, for a learned proposal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<String>,
    /// The studio tombstone rows this erasure produced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub erases: Vec<TombstoneRow>,
}

/// The studio's canonical journal: every episode the studio recorded.
///
/// The journal is ordered by the ledger sequence, so the studio's own record
/// and the ledger's record can be walked side by side.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    /// The entries, in ledger order.
    #[serde(default)]
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    /// The entry for one episode digest.
    pub fn entry(&self, episode: &Digest) -> Option<&JournalEntry> {
        self.entries.iter().find(|entry| entry.episode == *episode)
    }

    /// The edit entries, in ledger order.
    pub fn edits(&self) -> impl DoubleEndedIterator<Item = &JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == JournalKind::Edit)
    }

    /// The erasure entries, in ledger order.
    pub fn forgets(&self) -> impl Iterator<Item = &JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == JournalKind::Forget)
    }

    /// The learning entries, in ledger order.
    pub fn learns(&self) -> impl Iterator<Item = &JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == JournalKind::Learn)
    }

    /// The newest edit entry, the one that holds the current specimen state.
    pub fn last_edit(&self) -> Option<&JournalEntry> {
        self.edits().next_back()
    }

    /// The learning entry for one proposal, when the studio recorded one.
    pub fn episode_for_proposal(&self, proposal: &str) -> Option<&JournalEntry> {
        self.learns()
            .find(|entry| entry.proposal.as_deref() == Some(proposal))
    }
}
