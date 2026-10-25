//! Scoped identity/table foundation using caller-prepared keys and language trees.
//!
//! This module does not compute text-v3 keys, reinterpret text-v2 IDs, publish a
//! wire format, migrate saves or provide grammar/wildcard matching. Canonical
//! spelling normalization belongs to the later language packet. Here canonical
//! equality means exact original UTF-8 plus validated typed bind/action trees,
//! alternative order, inhibition and finite weight bits. Caller keys are routing
//! hints, never equality authority. Logical resource links are scoped labels,
//! not foreign keys or credentials; only concrete node references link entries.
//!
//! All concrete references are host-generated, globally unique across the three
//! concrete reference types in this table, and checked before mutation. Scope is
//! supplied by the host and is not authentication. Lookups return insertion order
//! in both array (up to 64 records per population/collection) and hash modes.
mod index;
mod model;
mod validate;
use index::Indexes;
pub use model::*;

/// Pure owned table. Derived indexes can always be rebuilt from its records.
#[derive(Default)]
pub struct HybridTable {
    records: TableSnapshot,
    indexes: Indexes,
}
impl HybridTable {
    /// Validate before mutation. A full target uses the supplied fresh sibling
    /// reference, or returns `Full` unchanged when allocation is unavailable.
    /// A duplicate returns its original concrete ref without relinking it.
    pub fn insert(
        &mut self,
        mut entry: EntryRecord,
        spill: Option<NodeRef>,
    ) -> Result<InsertOutcome, IdentityError> {
        validate::entry(&entry)?;
        let node = self
            .records
            .nodes
            .iter()
            .find(|n| n.reference == entry.node);
        if let Some(node) = node {
            if node.population != entry.population {
                return Err(IdentityError::PopulationMismatch);
            }
            if node.key != entry.key {
                return Err(IdentityError::KeyMismatch);
            }
        } else if self.reference_used(entry.node.0) || entry.node.0 == entry.reference.0 {
            return Err(IdentityError::DuplicateRef);
        }
        if let Some(old) = self
            .records
            .entries
            .iter()
            .find(|e| e.reference == entry.reference)
        {
            if old.same_semantics(&entry) {
                return Ok(InsertOutcome::Duplicate {
                    node: old.node,
                    entry: old.reference,
                });
            }
            return Err(IdentityError::DuplicateRef);
        }
        if self.reference_used(entry.reference.0) {
            return Err(IdentityError::DuplicateRef);
        }
        // Equality deliberately spans all keys and nodes in the population. A
        // caller may have changed key projections without changing the original.
        if let Some(old) = self
            .records
            .entries
            .iter()
            .find(|old| old.same_semantics(&entry))
        {
            return Ok(InsertOutcome::Duplicate {
                node: old.node,
                entry: old.reference,
            });
        }
        let count = self
            .records
            .entries
            .iter()
            .filter(|e| e.node == entry.node)
            .count();
        if count == 20 {
            let Some(fresh) = spill else {
                return Ok(InsertOutcome::Full);
            };
            if self.reference_used(fresh.0) || fresh.0 == entry.reference.0 {
                return Err(IdentityError::DuplicateRef);
            }
            entry.node = fresh;
        }
        // All fallible validation and host-allocation checks have completed.
        if node.is_none() || count == 20 {
            self.records.nodes.push(NodeRecord {
                population: entry.population.clone(),
                reference: entry.node,
                key: entry.key.clone(),
            });
        }
        let result = InsertOutcome::Inserted {
            node: entry.node,
            entry: entry.reference,
        };
        self.records.entries.push(entry);
        self.refresh_indexes();
        Ok(result)
    }
    pub fn find_all(&self, population: &Population, key: &PatternKey) -> Vec<EntryRef> {
        if let Some(index) = self.indexes.entries.get(population) {
            return index.get(key).cloned().unwrap_or_default();
        }
        self.records
            .entries
            .iter()
            .filter(|e| &e.population == population && &e.key == key)
            .map(|e| e.reference)
            .collect()
    }
    pub fn get(
        &self,
        population: &Population,
        node: NodeRef,
        entry: EntryRef,
    ) -> Option<&EntryRecord> {
        self.records
            .entries
            .iter()
            .find(|e| &e.population == population && e.node == node && e.reference == entry)
    }
    /// Target a concrete entry; a wrong node or population never removes it.
    /// Empty nodes are retained and remain bound to their original key/scope.
    pub fn remove(
        &mut self,
        population: &Population,
        node: NodeRef,
        entry: EntryRef,
    ) -> Result<bool, IdentityError> {
        let owner = self
            .records
            .nodes
            .iter()
            .find(|n| n.reference == node)
            .ok_or(IdentityError::MissingNode)?;
        if &owner.population != population {
            return Err(IdentityError::PopulationMismatch);
        }
        let Some(index) = self
            .records
            .entries
            .iter()
            .position(|e| e.reference == entry)
        else {
            return Ok(false);
        };
        let value = &self.records.entries[index];
        if &value.population != population {
            return Err(IdentityError::PopulationMismatch);
        }
        if value.node != node {
            return Err(IdentityError::NodeMismatch);
        }
        self.records.entries.remove(index);
        self.refresh_indexes();
        Ok(true)
    }
    pub fn insert_resource(&mut self, resource: ResourceRecord) -> Result<(), IdentityError> {
        validate::original(&resource.original)?;
        if self.reference_used(resource.reference.0) {
            return Err(IdentityError::DuplicateRef);
        }
        if self.records.resources.len() == 10000 {
            return Err(IdentityError::ResourceLimit);
        }
        self.records.resources.push(resource);
        self.refresh_indexes();
        Ok(())
    }
    pub fn find_resources(&self, population: &Population, key: &PatternKey) -> Vec<ResourceRef> {
        if let Some(index) = self.indexes.resources.get(population) {
            return index.get(key).cloned().unwrap_or_default();
        }
        self.records
            .resources
            .iter()
            .filter(|r| &r.population == population && &r.key == key)
            .map(|r| r.reference)
            .collect()
    }
    pub fn find_resource_links(
        &self,
        population: &Population,
        link: &ResourceId,
    ) -> Vec<ResourceRef> {
        if let Some(index) = self.indexes.links.get(population) {
            return index.get(link).cloned().unwrap_or_default();
        }
        self.records
            .resources
            .iter()
            .filter(|r| &r.population == population && r.link.as_ref() == Some(link))
            .map(|r| r.reference)
            .collect()
    }
    pub fn get_resource(
        &self,
        population: &Population,
        resource: ResourceRef,
    ) -> Option<&ResourceRecord> {
        self.records
            .resources
            .iter()
            .find(|r| &r.population == population && r.reference == resource)
    }
    /// Borrow entries in insertion order in O(1), without copying any records.
    /// Callers can reserve traversal/materialization work before inspecting them.
    pub fn entries(&self) -> &[EntryRecord] {
        &self.records.entries
    }
    pub fn snapshot(&self) -> TableSnapshot {
        self.records.clone()
    }
    /// Validate the complete owned replacement before publishing any state.
    /// Import preserves caller record order; refs, node linkage, caps, complete
    /// trees and semantic duplicates are checked before rebuilding hash indexes.
    pub fn rebuild(&mut self, records: TableSnapshot) -> Result<(), IdentityError> {
        validate::snapshot(&records)?;
        let indexes = Indexes::from_records(&records);
        self.records = records;
        self.indexes = indexes;
        Ok(())
    }
    fn reference_used(&self, reference: u128) -> bool {
        self.records
            .nodes
            .iter()
            .any(|n| n.reference.0 == reference)
            || self
                .records
                .entries
                .iter()
                .any(|e| e.reference.0 == reference)
            || self
                .records
                .resources
                .iter()
                .any(|r| r.reference.0 == reference)
    }
    fn refresh_indexes(&mut self) {
        self.indexes = Indexes::from_records(&self.records);
    }
}
