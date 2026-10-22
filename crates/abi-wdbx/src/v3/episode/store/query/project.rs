//! The store side of the query: how one record is projected into the eight
//! dimensions, a lineage, an explanation, and a disposition.

use super::super::super::signing::{SignatureStatus, SignerKeyId, check_signature};
use super::super::super::types::{
    ActorKind, EdgeReason, EpisodeEvent, MemoryCandidate, MemoryEdgeKind, OpenContradiction,
};
use super::super::{
    EpisodeStore, EpisodeStoreError, GuildMemories, MAX_RECEIPTS, StoredRecord, bounded_identifier,
    receipt,
};
use super::constraints::{ConstitutionalQuery, KeyResolver, MemberScope};
use super::dimensions::{
    AgeEvidence, CalibrationEvidence, CompatibilityEvidence, ConfidenceEvidence,
    CryptographicEvidence, EvidenceDimensions, HoldEvidence, LifecycleEvidence, OutcomeEvidence,
    RegimeEvidence, ReputationEvidence, RevocationEvidence, SignatureEvidence, SignerKeyStatus,
    SourceEvidence, UncertaintyEvidence, live_hold_kinds,
};
use super::result::{ConstitutionalResult, Lineage, MemoryResults};
use super::verdict::{Explanation, disposition_of, explanations};
use std::collections::BTreeMap;

/// The edges the ledger holds about one candidate.
#[derive(Clone, Debug, Default)]
struct EdgeFacts {
    /// The edge episodes naming the candidate.
    named: Vec<NamedEdge>,
    /// The resolutions closing a named edge.
    resolutions: Vec<Resolution>,
}

/// One edge episode naming a candidate.
#[derive(Clone, Copy, Debug)]
struct NamedEdge {
    /// The edge episode's own commitment.
    digest: [u8; 32],
    /// The kind of edge.
    kind: MemoryEdgeKind,
}

/// One resolution edge closing a named edge.
#[derive(Clone, Copy, Debug)]
struct Resolution {
    /// The edge episode the resolution closed.
    closed: [u8; 32],
    /// The resolution episode's own commitment.
    digest: [u8; 32],
    /// The reviewer's reason.
    reason: EdgeReason,
}

impl EdgeFacts {
    /// The resolution that closed one named edge, if any.
    fn resolution_of(&self, edge: &[u8; 32]) -> Option<&Resolution> {
        self.resolutions
            .iter()
            .find(|resolution| resolution.closed == *edge)
    }
}

impl EpisodeStore {
    /// Answer one constitutional query over a guild's memory candidates.
    ///
    /// The query is guild-scoped: it never returns a record of another
    /// guild, and a member-scope constraint partitions the guild's records by
    /// the isolation the ledger recorded for them. The limit is bounded by
    /// the same convention the rest of the store uses for receipts, and a
    /// limit outside it is refused as invalid input.
    ///
    /// The answer is a projection of what the ledger holds. It hides
    /// nothing: a record that is quarantined, contradicted, superseded,
    /// forgotten, held, or revoked is listed with the disposition that says
    /// why, unless the caller's own constraints exclude it. A record whose
    /// cryptographic evidence fails is listed as rejected, not omitted.
    ///
    /// The projection reads the guild's whole ledger, which the store's
    /// receipt bound keeps finite; it appends nothing and mutates nothing.
    ///
    /// The answer carries the scope it was asked for, so a caller cannot
    /// present one guild's answer as another's.
    pub fn query(&self, query: &ConstitutionalQuery) -> Result<MemoryResults, EpisodeStoreError> {
        if !bounded_identifier(&query.guild_ref, 128) || !(1..=MAX_RECEIPTS).contains(&query.limit)
        {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let resolver = self.resolver_for(query);
        let memories = self.state.memories.get(&query.guild_ref);
        let mut results = Vec::new();
        let mut disposition_counts = BTreeMap::new();
        for record in self
            .records
            .iter()
            .filter(|record| record.guild_ref == query.guild_ref)
        {
            let (candidate, actor_kind) = match &record.event {
                EpisodeEvent::MemoryCandidate {
                    recorded_by,
                    candidate,
                } => (candidate, recorded_by.kind),
                _ => continue,
            };
            if !self.admitted(query, memories, record, candidate) {
                continue;
            }
            let result =
                self.project(record, candidate, actor_kind, memories, resolver.as_ref())?;
            *disposition_counts.entry(result.disposition).or_insert(0) += 1;
            results.push(result);
            if results.len() >= query.limit {
                break;
            }
        }
        Ok(MemoryResults {
            scope: query.guild_ref.clone(),
            results,
            disposition_counts,
        })
    }

    /// The resolver the query uses: the caller's, else the store's own key.
    fn resolver_for(&self, query: &ConstitutionalQuery) -> Option<KeyResolver> {
        if let Some(resolver) = &query.key_resolver {
            return Some(resolver.clone());
        }
        self.signer.as_ref().map(|signer| {
            let key = signer.verifying_key();
            let key_id = signer.key_id().clone();
            KeyResolver::new(move |requested| (requested == &key_id).then_some(key))
        })
    }

    /// Whether one record satisfies the query's constraints.
    fn admitted(
        &self,
        query: &ConstitutionalQuery,
        memories: Option<&GuildMemories>,
        record: &StoredRecord,
        candidate: &MemoryCandidate,
    ) -> bool {
        let constraints = &query.constraints;
        if let Some(floor) = constraints.min_evidence_level
            && record.evidence_level < floor
        {
            return false;
        }
        let member_scoped = memories
            .and_then(|memories| memories.admitted.get(&record.episode_digest).copied())
            .unwrap_or(candidate.member_scoped);
        if let Some(scope) = constraints.member_scope {
            let wanted = match scope {
                MemberScope::Member => true,
                MemberScope::Guild => false,
            };
            if member_scoped != wanted {
                return false;
            }
        }
        let digest = record.episode_digest;
        let forgotten = memories.is_some_and(|memories| memories.forgotten.contains(&digest));
        if forgotten && !constraints.include_forgotten {
            return false;
        }
        let quarantined =
            memories.is_some_and(|memories| memories.quarantined.contains_key(&digest));
        if quarantined && !constraints.include_quarantined {
            return false;
        }
        let contradicted = memories.is_some_and(|memories| {
            memories
                .contradictions
                .keys()
                .any(|(low, high)| *low == digest || *high == digest)
        });
        if contradicted && !constraints.include_contradicted {
            return false;
        }
        // A hold and a revocation are governance decisions about the
        // candidate, and the caller may exclude them the same way it may
        // exclude a contradicted or a quarantined record. The default keeps
        // them listed: the store never hides a record on its own.
        if self.is_held(&record.guild_ref, &digest) && !constraints.include_held {
            return false;
        }
        let revoked = memories.is_some_and(|memories| memories.revoked.contains(&digest));
        if revoked && !constraints.include_revoked {
            return false;
        }
        true
    }

    /// Project one record as eight dimensions, a lineage, and an explanation.
    fn project(
        &self,
        record: &StoredRecord,
        candidate: &MemoryCandidate,
        actor_kind: ActorKind,
        memories: Option<&GuildMemories>,
        resolver: Option<&KeyResolver>,
    ) -> Result<ConstitutionalResult, EpisodeStoreError> {
        let facts = self.edge_facts(&record.guild_ref, &record.episode_digest);
        let supersession = self.supersession_lineage(&record.guild_ref, &record.episode_digest)?;
        let hold = self.hold_evidence(&record.guild_ref, &record.episode_digest)?;
        let revocation =
            if memories.is_some_and(|memories| memories.revoked.contains(&record.episode_digest)) {
                RevocationEvidence::Revoked
            } else {
                RevocationEvidence::NotRevoked
            };
        let dimensions = EvidenceDimensions {
            cryptographic: Self::cryptographic(record, resolver),
            source: SourceEvidence {
                source_type: record.source_type,
                actor_kind,
                reputation: ReputationEvidence::Unknown,
            },
            calibration: CalibrationEvidence::Unknown,
            outcome: Self::outcome(&facts, memories),
            compatibility: self.compatibility(record),
            regime: RegimeEvidence::Unknown,
            lifecycle: LifecycleEvidence {
                retention: candidate.retention,
                superseded_by: supersession.superseded_by,
                supersedes: supersession.superseded,
                forgotten: supersession.forgotten,
                tombstoned: supersession.tombstoned,
                open_quarantine: memories
                    .and_then(|memories| memories.quarantined.get(&record.episode_digest).copied()),
                hold,
                revocation,
                age: AgeEvidence::Unknown,
            },
            confidence: ConfidenceEvidence {
                maturity: record.evidence_level,
                uncertainty: UncertaintyEvidence::Unknown,
            },
        };
        let disposition = disposition_of(&dimensions);
        Ok(ConstitutionalResult {
            episode_digest: record.episode_digest,
            disposition,
            dimensions,
            lineage: Self::lineage(record, memories, &facts),
            explanation: Explanation {
                entries: explanations(&dimensions),
            },
            receipt: receipt(record),
        })
    }

    /// The hold state of one candidate, projected into the lifecycle
    /// dimension.
    ///
    /// The store's own hold index is the authority: a live hold freezes the
    /// candidate, and a hold that was released freezes nothing and reads as
    /// no hold at all. The projection carries the kinds of the live holds
    /// and never the authorities that placed them, so the dimension stays
    /// content-free.
    fn hold_evidence(
        &self,
        guild_ref: &str,
        digest: &[u8; 32],
    ) -> Result<HoldEvidence, EpisodeStoreError> {
        let Some(state) = self.hold_state(guild_ref, digest)? else {
            return Ok(HoldEvidence::NotHeld);
        };
        let kinds = live_hold_kinds(&state.holds);
        if kinds.is_empty() {
            Ok(HoldEvidence::NotHeld)
        } else {
            Ok(HoldEvidence::Held(kinds))
        }
    }

    /// The cryptographic dimension of one record.
    ///
    /// A record whose digest does not reproduce from its own fields is
    /// invalid evidence, whatever its signature says. Otherwise the detached
    /// signature decides, and the four signing states stay apart.
    fn cryptographic(
        record: &StoredRecord,
        resolver: Option<&KeyResolver>,
    ) -> CryptographicEvidence {
        let reproduces = record
            .computed_digest()
            .is_ok_and(|digest| digest == record.episode_digest);
        if !reproduces {
            return CryptographicEvidence {
                signature: SignatureEvidence::Invalid,
                key_status: SignerKeyStatus::Absent,
            };
        }
        let resolve = |key_id: &SignerKeyId| resolver.and_then(|resolver| resolver.resolve(key_id));
        match check_signature(&record.episode_digest, record.signature.as_ref(), resolve) {
            SignatureStatus::Valid(_) => CryptographicEvidence {
                signature: SignatureEvidence::Valid,
                key_status: SignerKeyStatus::Resolved,
            },
            SignatureStatus::Invalid(_) => CryptographicEvidence {
                signature: SignatureEvidence::Invalid,
                key_status: SignerKeyStatus::Resolved,
            },
            SignatureStatus::UnknownKey(_) => CryptographicEvidence {
                signature: SignatureEvidence::Unverifiable,
                key_status: SignerKeyStatus::Unresolved,
            },
            SignatureStatus::Unsigned => CryptographicEvidence {
                signature: SignatureEvidence::Unsigned,
                key_status: SignerKeyStatus::Absent,
            },
        }
    }

    /// The compatibility dimension of one record.
    fn compatibility(&self, record: &StoredRecord) -> CompatibilityEvidence {
        let Some(guild) = self.policy.guilds.get(&record.guild_ref) else {
            return CompatibilityEvidence::Unknown;
        };
        if record.contract_revision != self.policy.contract_revision
            || record.contract_digest != self.policy.contract_digest
        {
            return CompatibilityEvidence::ContractStale;
        }
        if record.policy_version != guild.policy_version {
            return CompatibilityEvidence::PolicyStale;
        }
        CompatibilityEvidence::Compatible
    }

    /// The outcome dimension of one record.
    fn outcome(facts: &EdgeFacts, memories: Option<&GuildMemories>) -> OutcomeEvidence {
        for edge in &facts.named {
            if edge.kind == MemoryEdgeKind::Contradicts && Self::edge_open(memories, &edge.digest) {
                return OutcomeEvidence::Contradicted;
            }
        }
        for edge in &facts.named {
            if edge.kind != MemoryEdgeKind::Contradicts {
                continue;
            }
            if let Some(resolution) = facts.resolution_of(&edge.digest) {
                return match resolution.reason {
                    EdgeReason::ReviewedValid => OutcomeEvidence::Validated,
                    _ => OutcomeEvidence::Contradicted,
                };
            }
        }
        for edge in &facts.named {
            if edge.kind != MemoryEdgeKind::Quarantines {
                continue;
            }
            if let Some(resolution) = facts.resolution_of(&edge.digest)
                && resolution.reason == EdgeReason::ReviewedValid
            {
                return OutcomeEvidence::Validated;
            }
        }
        OutcomeEvidence::Unobserved
    }

    /// Whether an edge episode is still open.
    fn edge_open(memories: Option<&GuildMemories>, edge: &[u8; 32]) -> bool {
        let Some(memories) = memories else {
            return false;
        };
        memories.quarantined.values().any(|open| open == edge)
            || memories.contradictions.values().any(|open| open == edge)
    }

    /// The edges naming one candidate, and the resolutions closing them.
    fn edge_facts(&self, guild_ref: &str, candidate: &[u8; 32]) -> EdgeFacts {
        let mut facts = EdgeFacts::default();
        for record in self
            .records
            .iter()
            .filter(|record| record.guild_ref == guild_ref)
        {
            let EpisodeEvent::MemoryEdge { edge, .. } = &record.event else {
                continue;
            };
            let names_candidate = match edge.kind {
                MemoryEdgeKind::Quarantines => edge.target == *candidate,
                MemoryEdgeKind::Contradicts => {
                    edge.target == *candidate || edge.counterpart == Some(*candidate)
                }
                MemoryEdgeKind::Resolves => false,
            };
            if names_candidate {
                facts.named.push(NamedEdge {
                    digest: record.episode_digest,
                    kind: edge.kind,
                });
            }
        }
        for record in self
            .records
            .iter()
            .filter(|record| record.guild_ref == guild_ref)
        {
            let EpisodeEvent::MemoryEdge { edge, .. } = &record.event else {
                continue;
            };
            if edge.kind != MemoryEdgeKind::Resolves {
                continue;
            }
            if facts.named.iter().any(|named| named.digest == edge.target) {
                facts.resolutions.push(Resolution {
                    closed: edge.target,
                    digest: record.episode_digest,
                    reason: edge.reason,
                });
            }
        }
        facts
    }

    /// The lineage of one record.
    fn lineage(
        record: &StoredRecord,
        memories: Option<&GuildMemories>,
        facts: &EdgeFacts,
    ) -> Lineage {
        let digest = record.episode_digest;
        let mut open_contradictions = Vec::new();
        if let Some(memories) = memories {
            for (&(low, high), &edge) in &memories.contradictions {
                if low == digest {
                    open_contradictions.push(OpenContradiction {
                        counterpart: high,
                        edge,
                    });
                } else if high == digest {
                    open_contradictions.push(OpenContradiction {
                        counterpart: low,
                        edge,
                    });
                }
            }
            open_contradictions.sort_by_key(|item| item.counterpart);
        }
        Lineage {
            previous_digest: record.previous_digest,
            superseded_by: memories
                .and_then(|memories| memories.superseded_by.get(&digest).copied()),
            open_contradictions,
            resolution_edge: facts.resolutions.last().map(|resolution| resolution.digest),
        }
    }
}
