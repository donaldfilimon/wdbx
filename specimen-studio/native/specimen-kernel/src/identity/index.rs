use super::*;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};

// Deterministic in-memory hashers; observable order always comes from records.
// Neither random seeds nor hash iteration order participate in results.
pub(super) type Map<K, V> = HashMap<K, V, BuildHasherDefault<DefaultHasher>>;
type KeyIndex<R> = Map<Population, Map<PatternKey, Vec<R>>>;
#[derive(Default)]
pub(super) struct Indexes {
    pub entries: KeyIndex<EntryRef>,
    pub resources: KeyIndex<ResourceRef>,
    pub links: Map<Population, Map<ResourceId, Vec<ResourceRef>>>,
}
impl Indexes {
    pub fn from_records(records: &TableSnapshot) -> Self {
        let mut entry_counts: Map<&Population, usize> = Map::default();
        let mut resource_counts: Map<&Population, usize> = Map::default();
        for e in &records.entries {
            *entry_counts.entry(&e.population).or_default() += 1;
        }
        for r in &records.resources {
            *resource_counts.entry(&r.population).or_default() += 1;
        }
        let mut indexes = Self::default();
        for e in &records.entries {
            if entry_counts[&e.population] > 64 {
                indexes
                    .entries
                    .entry(e.population.clone())
                    .or_default()
                    .entry(e.key.clone())
                    .or_default()
                    .push(e.reference);
            }
        }
        for r in &records.resources {
            if resource_counts[&r.population] > 64 {
                indexes
                    .resources
                    .entry(r.population.clone())
                    .or_default()
                    .entry(r.key.clone())
                    .or_default()
                    .push(r.reference);
                // Include an empty map for large unlinked populations as well.
                let links = indexes.links.entry(r.population.clone()).or_default();
                if let Some(link) = &r.link {
                    links.entry(link.clone()).or_default().push(r.reference);
                }
            }
        }
        indexes
    }
}
