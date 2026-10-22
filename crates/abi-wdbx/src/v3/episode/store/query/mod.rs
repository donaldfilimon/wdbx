//! Constitutional query: the evidence-aware read model of the episode ledger.
//!
//! The canonical episode contract asks for trust to be eight separate
//! dimensions and never one number. This module is that read model: a
//! guild-scoped query over the memory-candidate episodes a store holds, each
//! result carrying the eight dimensions separately, the lineage the record
//! belongs to, a per-dimension explanation, and a disposition.
//!
//! Three rules shape everything here.
//!
//! * **Missing evidence is not negative evidence.** The signing module keeps
//!   four states apart for the same reason: an unsigned episode is not an
//!   invalid one, and a signature under a key the verifier cannot resolve is
//!   unverifiable rather than forged. A dimension the store does not hold
//!   reports `Unknown`, never a zero and never a default that could pass for
//!   a measurement.
//! * **Uncertainty is never hidden.** A quarantined, contradicted, superseded,
//!   forgotten, held, or revoked record is still listed, with the disposition
//!   that says what is wrong with it. A memory edge never hides its target,
//!   and a governance decision about a candidate never hides it either.
//! * **There is no single number.** No public type here carries a
//!   floating-point value, and none carries one number standing in for the
//!   eight dimensions.
//!
//! The dimensions are named from the canonical contract: cryptographic,
//! source, calibration, outcome, compatibility, regime, lifecycle, and
//! confidence. Calibration, regime, and the uncertainty half of confidence
//! are `Unknown` for every record, because the ledger holds no calibration
//! record, no task-regime field, and no uncertainty interval. That is the
//! honest answer, and the one the contract requires.
//!
//! # Holds and revocations: where the facts and the verdicts go
//!
//! A *hold* is a placement that freezes a memory candidate: while it is live
//! the candidate is not expired away, not reclaimed, not silently superseded,
//! and its payload is not destroyed. A *revocation* withdraws a candidate by
//! authority: it is not live, no later correction may name it, and it is
//! never resurrected, while its record and its anti-resurrection metadata
//! are retained. Both are governance decisions *about* a candidate rather
//! than new kinds of episode, so both live in derived sidecars beside the
//! ledger (`store/holds/`), not in the committed record.
//!
//! The question this module has to answer is where those two facts belong in
//! the read model. The answer is **both**, and deliberately so:
//!
//! * **The fact belongs to the lifecycle dimension (E07).** The canonical
//!   contract puts "age, `superseded_by`, retention class, quarantine" in the
//!   lifecycle dimension, and the constitution's own record vocabulary puts
//!   "retention class, hold state, deletion-key reference" there too. A hold
//!   and a revocation are the same kind of thing as a quarantine: a fact
//!   about where the record stands in the guild's memory graph right now.
//!   So [`LifecycleEvidence`](dimensions::LifecycleEvidence) carries
//!   [`HoldEvidence`](dimensions::HoldEvidence) and
//!   [`RevocationEvidence`](dimensions::RevocationEvidence), and the
//!   dimension distinguishes *no hold* from *held* and *not revoked* from
//!   *revoked*. Neither is a fabricated zero: both are measured states, and
//!   a hold that was released is not in force and reads as no hold.
//! * **The verdict belongs to the disposition.** The disposition is the
//!   closed vocabulary of *what a caller may do with one record*. A consumer
//!   that filters or orders on dispositions must be able to say "do not offer
//!   held or revoked records" in the verdict itself, without digging into a
//!   dimension. Reporting the fact in the dimension alone would leave the
//!   verdict answering "accepted" for a record that an authority has
//!   withdrawn.
//!
//! Doing only one of the two would leave either the fact undiscoverable to a
//! disposition-only consumer or the verdict unexpressed. So the read model
//! does both, and the two always agree: a held record never reads
//! `Accepted`, and a revoked record never reads `Accepted`.
//!
//! # The precedence
//!
//! The disposition precedence is fixed and documented in
//! [`verdict::disposition_of`]. It runs from the most severe finding to the
//! least: a rejected record (invalid evidence or erased content), a revoked
//! record (withdrawn by an authority), a quarantined record (an open
//! review), a contradicted record (an open conflict), a held record (a live
//! freeze), an uncertain record (retained, inspectable, semantically
//! uncertain), an unknown record (too little evidence), and finally an
//! accepted record.
//!
//! Two orderings in that list are worth stating outright, because they are
//! judgement calls and not derivations:
//!
//! * **A revocation takes precedence over a quarantine and a contradiction.** A
//!   quarantine and a contradiction are open, reviewable disputes about a
//!   record's content; a revocation is a terminal withdrawal by an
//!   authority. A record that is both revoked and contradicted is therefore
//!   read as `Revoked`: the authority's decision governs, and the
//!   contradiction is still visible in the outcome dimension.
//! * **A hold comes after a quarantine and a contradiction, and before a
//!   superseded record.** A hold says nothing about the content's truth; it
//!   freezes the record's destruction and its supersession. A dispute about
//!   the content is worse news than a freeze, and a freeze is worse news
//!   than a stale correction, because the freeze is a live constraint on the
//!   record while the correction is a fact about its history.
//!
//! # The constraints
//!
//! [`QueryConstraints`](constraints::QueryConstraints) lets a caller exclude
//! records it does not want to see. The default includes a held or a revoked
//! record, with the disposition that says why: a hold and a revocation are
//! evidence the caller must not silently ignore, exactly as a contradiction
//! and a quarantine are, and exactly as a rejected record is listed rather
//! than dropped. A forgotten record is the one exclusion the default makes,
//! because a tombstone means the content is gone and there is nothing left
//! to report.

mod constraints;
mod dimensions;
mod project;
mod result;
mod verdict;

pub use constraints::{ConstitutionalQuery, KeyResolver, MemberScope, QueryConstraints};
pub use dimensions::{
    AgeEvidence, CalibrationEvidence, CompatibilityEvidence, CryptographicEvidence,
    EvidenceDimension, EvidenceDimensions, HoldEvidence, HoldKinds, LifecycleEvidence,
    OutcomeEvidence, RegimeEvidence, ReputationEvidence, RevocationEvidence, SignatureEvidence,
    SignerKeyStatus, SourceEvidence, UncertaintyEvidence,
};
pub use result::{ConstitutionalResult, Lineage, MemoryResults};
pub use verdict::{Disposition, Explanation, Reason};
