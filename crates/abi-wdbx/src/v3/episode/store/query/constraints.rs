//! What a query admits, what it excludes, and how a caller narrows it.

use super::super::super::signing::SignerKeyId;
use super::super::super::types::EvidenceLevel;
use ed25519_dalek::VerifyingKey;
use std::fmt;
use std::sync::Arc;

/// Which member-scope class a query admits.
///
/// The ledger records one bit per candidate: whether the record is isolated
/// to one member of the guild or shared by the guild. A query that names a
/// scope sees only the records of that scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberScope {
    /// Records isolated to one member of the guild.
    Member,
    /// Records shared by the whole guild.
    Guild,
}

/// What a query admits and what it excludes. The defaults keep uncertainty
/// visible: a contradicted, quarantined, held, or revoked record is listed
/// unless the caller says otherwise. A forgotten record is not listed by
/// default, because a tombstone means the memory was erased.
///
/// The inclusion flags are independent toggles over a closed vocabulary of
/// findings, so a bool is the right shape for each of them: there is no
/// state beyond "include" and "exclude" for any one of them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryConstraints {
    /// Include records an open contradiction names.
    pub include_contradicted: bool,
    /// Include records an open quarantine names.
    pub include_quarantined: bool,
    /// Include records a forgetting tombstone erased.
    pub include_forgotten: bool,
    /// Include records a live hold freezes.
    ///
    /// The default is to include them. A hold is evidence the caller must
    /// not silently ignore: it says the record's destruction is frozen, and
    /// hiding the record would hide the freeze. The caller that wants only
    /// freely usable records excludes them here and reads the disposition
    /// instead.
    pub include_held: bool,
    /// Include records an authority revoked.
    ///
    /// The default is to include them, for the same reason as the held
    /// records: a revocation is a withdrawal the caller must see, and a
    /// record the caller never sees is a record the caller cannot account
    /// for.
    pub include_revoked: bool,
    /// Restrict the results to one member-scope class.
    pub member_scope: Option<MemberScope>,
    /// Exclude records whose recorded maturity is below this level.
    pub min_evidence_level: Option<EvidenceLevel>,
}

impl Default for QueryConstraints {
    fn default() -> Self {
        Self {
            include_contradicted: true,
            include_quarantined: true,
            include_forgotten: false,
            include_held: true,
            include_revoked: true,
            member_scope: None,
            min_evidence_level: None,
        }
    }
}

/// The closure a [`KeyResolver`] holds.
type ResolveKey = dyn Fn(&SignerKeyId) -> Option<VerifyingKey> + Send + Sync;

/// Resolves an episode-signing key identifier to the key that verifies it.
/// The caller supplies this because key custody is the caller's business: a
/// resolver that cannot resolve a key yields an unverifiable signature,
/// never an invalid one.
#[derive(Clone)]
pub struct KeyResolver(Arc<ResolveKey>);

impl KeyResolver {
    /// Build a resolver from any closure.
    pub fn new<F>(resolve: F) -> Self
    where
        F: Fn(&SignerKeyId) -> Option<VerifyingKey> + Send + Sync + 'static,
    {
        Self(Arc::new(resolve))
    }

    /// Resolve one key identifier.
    #[must_use]
    pub fn resolve(&self, key_id: &SignerKeyId) -> Option<VerifyingKey> {
        (self.0)(key_id)
    }
}

impl fmt::Debug for KeyResolver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("KeyResolver")
    }
}

/// One constitutional query over one guild's memory candidates.
#[derive(Clone, Debug)]
pub struct ConstitutionalQuery {
    /// The guild the query is scoped to.
    pub guild_ref: String,
    /// The greatest number of results to return.
    pub limit: usize,
    /// What the query admits and excludes.
    pub constraints: QueryConstraints,
    /// The resolver for episode-signing keys.
    ///
    /// Without one, the store resolves only the key it signs with, if it
    /// signs at all. A signature under any other key is then unverifiable,
    /// never invalid.
    pub key_resolver: Option<KeyResolver>,
}

impl ConstitutionalQuery {
    /// A query for one guild with the default constraints.
    #[must_use]
    pub fn new(guild_ref: impl Into<String>, limit: usize) -> Self {
        Self {
            guild_ref: guild_ref.into(),
            limit,
            constraints: QueryConstraints::default(),
            key_resolver: None,
        }
    }

    /// Attach a resolver for episode-signing keys.
    #[must_use]
    pub fn with_key_resolver(mut self, resolver: KeyResolver) -> Self {
        self.key_resolver = Some(resolver);
        self
    }
}
