//! Deterministic independent-edit merge for the offline outbox.
//!
//! # The model
//!
//! A *mergeable object* is a configuration item with fields: a specimen's
//! description, a learning configuration, a network configuration, or an
//! authority binding. The vocabulary is closed and small on purpose: this is
//! not a general CRDT, and it does not try to be one.
//!
//! An *edit* is a set of field changes. Each change names a field key and
//! commits to the field's new value by digest — never by value — so the
//! outbox holds a commitment to the edit, not the edit's content.
//!
//! The merge rule is one sentence:
//!
//! > Two edits that touch **disjoint** fields of the same object merge
//! > automatically; two edits that touch the **same** field conflict.
//!
//! A conflict is never resolved by the outbox. There is no last-write-wins,
//! no timestamp comparison, and no invented winner. The conflicting branches
//! are recorded and preserved, and an authorized caller resolves them.
//!
//! # Determinism
//!
//! The merge is a pure function of the operations it is given. Objects are
//! grouped in key order, fields in key order, competing digests in
//! ascending order, and contributing operations in identifier order. Nothing
//! reads a clock, and nothing depends on the order in which the operations
//! were enqueued: the same set of edits produces the same merged object
//! whatever order they arrived in.
//!
//! # Versions
//!
//! Two edits to the same object that carry *different* bindings — a
//! different contract revision, a different policy version, a different
//! consent epoch — are incompatible versions, not independent edits. They
//! are preserved as a conflict rather than merged into a hybrid nobody
//! wrote.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::types::{OperationId, OperationPayload, OutboxOperation, bounded_identifier};

/// The greatest length of a mergeable object identifier, in bytes.
pub const MAX_OBJECT_ID_BYTES: usize = 128;
/// The greatest length of a field key, in bytes.
pub const MAX_FIELD_KEY_BYTES: usize = 64;
/// The greatest number of fields one edit may carry.
pub const MAX_EDIT_FIELDS: usize = 64;

/// The closed kinds of mergeable configuration object.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MergeObjectKind {
    /// A specimen's descriptive configuration.
    Specimen,
    /// A learning configuration.
    Learning,
    /// A network configuration.
    Network,
    /// An authority binding.
    Authority,
}

/// The identity of one mergeable configuration object.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(deny_unknown_fields)]
pub struct MergeTarget {
    /// The kind of object.
    pub kind: MergeObjectKind,
    /// The object's identifier within its kind.
    pub id: String,
}

/// One field-level edit of a mergeable object.
///
/// The value is committed to by digest. The outbox never holds the field's
/// value, so two edits are compared by commitment and never by content.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(deny_unknown_fields)]
pub struct FieldEdit {
    /// The field's key within the object.
    pub key: String,
    /// SHA-256 over the field's canonical value bytes.
    pub value_digest: [u8; 32],
}

/// A field-level edit proposal of one mergeable object.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditProposal {
    /// The object the edit touches.
    pub target: MergeTarget,
    /// The edited fields, in key order, without duplicates.
    pub fields: Vec<FieldEdit>,
}

/// Why two operations conflict.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Two operations set the same field to different values.
    DivergentValues,
    /// Two operations carry incompatible versions of the same object.
    IncompatibleVersion,
}

/// One recorded conflict.
///
/// The conflict is content-free: it names a field, the competing value
/// *digests*, and the operations that compete. It never holds a value.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MergeConflict {
    /// The object the conflict is in.
    pub target: MergeTarget,
    /// The field the conflict is in; absent for an object-level conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Why the operations conflict.
    pub kind: ConflictKind,
    /// The competing value digests, in ascending order.
    pub value_digests: Vec<[u8; 32]>,
    /// The operations that compete, in identifier order.
    pub operations: Vec<OperationId>,
}

/// The conflict as the outbox records it on the operation.
///
/// It is the same conflict, held beside the operation it belongs to, so a
/// reopened outbox still knows why an operation is in conflict.
pub type RecordedConflict = MergeConflict;

/// The content-free summary of a recorded conflict.
///
/// The summary names the field and counts the branches. It holds no digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictSummary {
    /// The kind of object the conflict is in.
    pub object_kind: MergeObjectKind,
    /// The field the conflict is in, when the conflict is about one field.
    pub field: Option<String>,
    /// How many competing value digests the conflict holds.
    pub value_digests: usize,
    /// How many operations compete in the conflict.
    pub operations: usize,
}

impl MergeConflict {
    /// The content-free summary of this conflict.
    #[must_use]
    pub fn summary(&self) -> ConflictSummary {
        ConflictSummary {
            object_kind: self.target.kind,
            field: self.field.clone(),
            value_digests: self.value_digests.len(),
            operations: self.operations.len(),
        }
    }
}

/// One merged object.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MergedObject {
    /// The object the merge produced.
    pub target: MergeTarget,
    /// The merged fields, in key order.
    pub fields: Vec<FieldEdit>,
    /// The operations that contributed, in identifier order.
    pub contributors: Vec<OperationId>,
}

/// The deterministic result of merging a set of edit operations.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MergePlan {
    /// The merged objects, in canonical order.
    pub merged: Vec<MergedObject>,
    /// The conflicts, in canonical order.
    pub conflicts: Vec<MergeConflict>,
}

/// The object groups, keyed by kind and identifier.
type Groups<'a> = BTreeMap<(MergeObjectKind, String), Vec<&'a OutboxOperation>>;

/// Merge a set of edit operations.
///
/// Non-edit operations are ignored: only the `Edit` payloads take part in
/// the merge. The result is a pure function of the operations given, and
/// does not depend on their order.
pub(crate) fn merge_edits(operations: &[&OutboxOperation]) -> MergePlan {
    let mut groups: Groups<'_> = BTreeMap::new();
    for operation in operations {
        let OperationPayload::Edit(edit) = &operation.payload else {
            continue;
        };
        groups
            .entry((edit.target.kind, edit.target.id.clone()))
            .or_default()
            .push(operation);
    }
    let mut plan = MergePlan::default();
    for ((kind, id), group) in groups {
        let target = MergeTarget { kind, id };
        merge_group(&target, &group, &mut plan);
    }
    plan
}

/// Merge one object's group of edits.
fn merge_group(target: &MergeTarget, group: &[&OutboxOperation], plan: &mut MergePlan) {
    // Incompatible versions never merge: the branches are preserved whole.
    if !compatible_bindings(group) {
        let mut operations: Vec<OperationId> = group.iter().map(|op| op.id.clone()).collect();
        operations.sort();
        operations.dedup();
        plan.conflicts.push(MergeConflict {
            target: target.clone(),
            field: None,
            kind: ConflictKind::IncompatibleVersion,
            value_digests: Vec::new(),
            operations,
        });
        return;
    }
    // Every distinct value digest proposed for one field.
    let mut proposals: BTreeMap<&str, BTreeMap<[u8; 32], Vec<&OutboxOperation>>> = BTreeMap::new();
    for operation in group {
        let OperationPayload::Edit(edit) = &operation.payload else {
            continue;
        };
        for field in &edit.fields {
            proposals
                .entry(field.key.as_str())
                .or_default()
                .entry(field.value_digest)
                .or_default()
                .push(operation);
        }
    }
    // A field with two distinct digests is a conflict, and every operation
    // that touches it is a branch of that conflict.
    let mut branches: BTreeSet<OperationId> = BTreeSet::new();
    for (key, by_digest) in &proposals {
        if by_digest.len() < 2 {
            continue;
        }
        let mut value_digests: Vec<[u8; 32]> = by_digest.keys().copied().collect();
        value_digests.sort_unstable();
        let mut operations: Vec<OperationId> = by_digest
            .values()
            .flatten()
            .map(|op| op.id.clone())
            .collect();
        operations.sort();
        operations.dedup();
        branches.extend(operations.iter().cloned());
        plan.conflicts.push(MergeConflict {
            target: target.clone(),
            field: Some((*key).to_string()),
            kind: ConflictKind::DivergentValues,
            value_digests,
            operations,
        });
    }
    // The merged object holds the fields of the operations that are not a
    // branch of any conflict. A field that no branch touched has exactly
    // one digest among the remaining operations.
    let mut fields: BTreeMap<String, [u8; 32]> = BTreeMap::new();
    let mut contributors: BTreeSet<OperationId> = BTreeSet::new();
    for operation in group {
        if branches.contains(&operation.id) {
            continue;
        }
        let OperationPayload::Edit(edit) = &operation.payload else {
            continue;
        };
        let mut contributed = false;
        for field in &edit.fields {
            fields
                .entry(field.key.clone())
                .or_insert(field.value_digest);
            contributed = true;
        }
        if contributed {
            contributors.insert(operation.id.clone());
        }
    }
    if contributors.is_empty() {
        return;
    }
    plan.merged.push(MergedObject {
        target: target.clone(),
        fields: fields
            .into_iter()
            .map(|(key, value_digest)| FieldEdit { key, value_digest })
            .collect(),
        contributors: contributors.into_iter().collect(),
    });
}

/// Whether every operation in the group carries the same binding.
fn compatible_bindings(group: &[&OutboxOperation]) -> bool {
    let mut bindings = group.iter().map(|operation| &operation.binding);
    let Some(first) = bindings.next() else {
        return true;
    };
    bindings.all(|binding| binding == first)
}

/// Whether the edit proposal is well formed.
pub(crate) fn validate_edit(edit: &EditProposal) -> bool {
    if !bounded_identifier(&edit.target.id, MAX_OBJECT_ID_BYTES)
        || edit.fields.is_empty()
        || edit.fields.len() > MAX_EDIT_FIELDS
    {
        return false;
    }
    let mut previous: Option<&str> = None;
    for field in &edit.fields {
        if !bounded_identifier(&field.key, MAX_FIELD_KEY_BYTES) || field.value_digest == [0; 32] {
            return false;
        }
        if previous.is_some_and(|key| key >= field.key.as_str()) {
            return false;
        }
        previous = Some(field.key.as_str());
    }
    true
}
