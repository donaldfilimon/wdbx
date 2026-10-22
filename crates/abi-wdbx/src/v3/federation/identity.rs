//! Peer identities, organizations, residency classes, and the peer directory.
//!
//! A peer is an *organization's* participant, not an organization. The
//! organization is the unit of authority: an imported record keeps the
//! authority of the organization that authored it, and every exchange names
//! one origin organization.
//!
//! # Identity is derived, not chosen
//!
//! A peer's identifier is derived from its verifying key, exactly as the
//! episode store's [`SignerKeyId`](crate::v3::episode::SignerKeyId) is. A
//! peer therefore cannot claim an identity its key does not match, and a
//! rotated key is a *new* peer identity: the old trust record does not follow
//! the new key, because trust is bound to one peer identity and is never
//! transitive.
//!
//! # Shared tenancy shares nothing
//!
//! Two organizations hosted on the same machine hold two peer identities, two
//! keys, and two trust records. Nothing in this module lets one organization
//! act for another; a shared host is not a shared authority.
//!
//! # No key custody
//!
//! This module holds verifying keys in memory and forgets them at exit. It
//! stores, protects, rotates, and audits nothing: there is no certificate
//! authority, no key store, and no revocation service here. A host that
//! enrolls a peer is responsible for that peer's key custody.

use crate::v3::federation::canonical::hex_digit;
use crate::v3::federation::error::FederationError;
use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::fmt;

/// Prefix of every derived peer identifier.
pub const PEER_ID_PREFIX: &str = "peer:";

/// Maximum length of a bounded identifier.
const MAX_IDENTIFIER: usize = 64;

/// The identity of one peer: `peer:` plus the lowercase hex of SHA-256 over
/// the 32 verifying-key bytes.
///
/// The identifier is derived from the key rather than chosen by an operator,
/// so a bundle cannot claim a peer identity its signature was not made under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(String);

impl PeerId {
    /// Derive the identifier of a verifying key.
    #[must_use]
    pub fn for_key(key: &PeerVerifyingKey) -> Self {
        let fingerprint = Sha256::digest(key.0.as_bytes());
        let mut id = String::with_capacity(PEER_ID_PREFIX.len() + 64);
        id.push_str(PEER_ID_PREFIX);
        for byte in fingerprint {
            id.push(hex_digit(byte >> 4));
            id.push(hex_digit(byte & 0x0f));
        }
        Self(id)
    }

    /// Parse a stored identifier, accepting only the exact derived shape.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix(PEER_ID_PREFIX)?;
        (hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .then(|| Self(value.to_owned()))
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for PeerId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PeerId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("invalid peer id"))
    }
}

/// A peer's public key.
///
/// The key is opaque: it can be compared, and it can derive a peer identity,
/// but its bytes are not part of this crate's public surface.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PeerVerifyingKey(VerifyingKey);

impl PeerVerifyingKey {
    /// The peer identity this key derives.
    #[must_use]
    pub fn peer_id(&self) -> PeerId {
        PeerId::for_key(self)
    }

    /// The raw verifying key, for crate-internal verification.
    pub(crate) fn as_ed25519(&self) -> &VerifyingKey {
        &self.0
    }
}

impl fmt::Debug for PeerVerifyingKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PeerVerifyingKey")
    }
}

/// A peer's signing key.
///
/// The secret never leaves this type, and the `Debug` rendering never prints
/// it. This is a prototype convenience, not a key store: production key
/// custody, hardware protection, and rotation ceremonies are not implemented
/// here.
pub struct PeerKey {
    signing: SigningKey,
    verifying: PeerVerifyingKey,
}

impl PeerKey {
    /// Build a signing key from a 32-byte seed.
    ///
    /// This constructor exists so prototypes and tests are deterministic. A
    /// production host supplies its own key material; this crate does not
    /// store, protect, or rotate keys.
    #[must_use]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let signing = SigningKey::from_bytes(&seed);
        let verifying = PeerVerifyingKey(signing.verifying_key());
        Self { signing, verifying }
    }

    /// The peer identity this key derives.
    #[must_use]
    pub fn peer_id(&self) -> PeerId {
        self.verifying.peer_id()
    }

    /// The public half of this key.
    #[must_use]
    pub fn verifying(&self) -> PeerVerifyingKey {
        self.verifying
    }

    /// Sign a message with this key.
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }
}

impl fmt::Debug for PeerKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PeerKey")
            .field("peer_id", &self.peer_id())
            .finish_non_exhaustive()
    }
}

/// The identifier of an independent organization.
///
/// An organization is the unit of *authority*: an imported record keeps the
/// authority of the organization that authored it, never the authority of the
/// organization that imported it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrganizationId(String);

impl OrganizationId {
    /// Build a bounded organization identifier.
    pub fn new(value: &str) -> Result<Self, FederationError> {
        if !bounded_identifier(value) {
            return Err(FederationError::InvalidInput);
        }
        Ok(Self(value.to_string()))
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrganizationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for OrganizationId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for OrganizationId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::new(&text).map_err(|_| serde::de::Error::custom("invalid organization id"))
    }
}

/// The deployment class a peer declares.
///
/// The vocabulary is closed: a peer declares exactly one class, and a grant
/// names the class it requires of the other side. This models the *shape* of
/// a residency restriction; it does not enforce one, because nothing in this
/// crate can observe where a peer actually runs.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyClass {
    /// No residency restriction.
    Unrestricted,
    /// The peer is deployed inside one region.
    SameRegion,
    /// The peer is deployed on premises.
    OnPremise,
}

impl ResidencyClass {
    /// The strictness rank of the class.
    ///
    /// The classes form a chain: an on-premise deployment is also inside one
    /// region, and a regional deployment imposes no residency restriction at
    /// all.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Unrestricted => 0,
            Self::SameRegion => 1,
            Self::OnPremise => 2,
        }
    }

    /// Whether a peer of this class satisfies a requirement of `required`.
    #[must_use]
    pub const fn satisfies(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
}

/// One enrolled peer: its derived identity, its organization, its public
/// key, and the residency class it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerIdentity {
    peer: PeerId,
    organization: OrganizationId,
    key: PeerVerifyingKey,
    residency: ResidencyClass,
}

impl PeerIdentity {
    /// Enroll a peer.
    ///
    /// The identity is derived from the key, so the caller cannot name a peer
    /// the key does not match.
    #[must_use]
    pub fn new(
        key: PeerVerifyingKey,
        organization: OrganizationId,
        residency: ResidencyClass,
    ) -> Self {
        Self {
            peer: key.peer_id(),
            organization,
            key,
            residency,
        }
    }

    /// The peer's derived identity.
    #[must_use]
    pub fn peer(&self) -> &PeerId {
        &self.peer
    }

    /// The organization the peer acts for.
    #[must_use]
    pub fn organization(&self) -> &OrganizationId {
        &self.organization
    }

    /// The residency class the peer declares.
    #[must_use]
    pub fn residency(&self) -> ResidencyClass {
        self.residency
    }

    /// The peer's public key.
    #[must_use]
    pub fn key(&self) -> PeerVerifyingKey {
        self.key
    }
}

/// The peers this organization knows.
///
/// The directory is a local, in-memory enrollment list. It is not a public
/// directory, a certificate authority, or a discovery service: a peer that is
/// not enrolled is unknown, and an unknown peer cannot sign a bundle this
/// organization will accept.
#[derive(Clone, Debug, Default)]
pub struct PeerDirectory {
    peers: BTreeMap<PeerId, PeerIdentity>,
}

impl PeerDirectory {
    /// An empty directory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enroll a peer.
    ///
    /// Enrolling the same peer twice is idempotent: the identity is derived
    /// from the key, so the same peer identity always carries the same key.
    pub fn enroll(&mut self, identity: PeerIdentity) -> Result<(), FederationError> {
        self.peers.insert(identity.peer().clone(), identity);
        Ok(())
    }

    /// Withdraw a peer's enrollment.
    ///
    /// A withdrawn peer is unknown from then on: its bundles no longer
    /// resolve, so they are refused rather than silently accepted.
    pub fn withdraw(&mut self, peer: &PeerId) -> Result<(), FederationError> {
        self.peers.remove(peer);
        Ok(())
    }

    /// The enrolled identity of one peer.
    #[must_use]
    pub fn get(&self, peer: &PeerId) -> Option<&PeerIdentity> {
        self.peers.get(peer)
    }

    /// Whether one peer is enrolled.
    #[must_use]
    pub fn contains(&self, peer: &PeerId) -> bool {
        self.peers.contains_key(peer)
    }

    /// How many peers are enrolled.
    #[must_use]
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Whether no peer is enrolled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

/// Whether a value is a bounded, opaque identifier.
///
/// The alphabet is the episode store's: lowercase letters, digits, `_`, `-`,
/// and `.`. An organization identity therefore never carries a path
/// separator, a space, or an uppercase letter.
pub(crate) fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}
