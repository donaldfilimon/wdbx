use super::index::Map;
use super::*;
use crate::language::{Expr, Scope, ledger};
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, DefaultHasher};

pub(super) fn original(value: &str) -> Result<(), IdentityError> {
    if value.chars().count() > 2000 {
        return Err(IdentityError::PatternTooLong);
    }
    Ok(())
}
pub(super) fn entry(record: &EntryRecord) -> Result<(), IdentityError> {
    original(&record.original)?;
    ast(&record.ast.bind, Scope::PatternBind)?;
    for alternative in &record.ast.alternatives {
        if !alternative.weight.is_finite() {
            return Err(IdentityError::InvalidWeight);
        }
        ast(&alternative.action, Scope::NodeAction)?;
    }
    Ok(())
}

// Enforce the existing language parser's registered scopes, 16-level nesting and
// 8,000-byte action budget on complete prepared trees. No new grammar is parsed.
fn ast(tree: &[Expr], scope: Scope) -> Result<(), IdentityError> {
    if tree.len() > 8000 {
        return Err(IdentityError::InvalidAst);
    }
    let registry = ledger();
    let mut pending = tree.iter().map(|expr| (expr, 0usize)).collect::<Vec<_>>();
    let mut bytes = 0usize;
    let mut nodes = 0usize;
    while let Some((expr, depth)) = pending.pop() {
        nodes += 1;
        if depth > 16 || nodes > 8000 {
            return Err(IdentityError::InvalidAst);
        }
        match expr {
            Expr::Text { value } => bytes = bytes.saturating_add(value.len()),
            Expr::Call { name, args } => {
                if !registry.iter().any(|contact| {
                    contact.name.eq_ignore_ascii_case(name) && contact.scopes.contains(&scope)
                }) {
                    return Err(IdentityError::InvalidAst);
                }
                if name.eq_ignore_ascii_case("literal")
                    && !matches!(args.as_slice(), [arg] if matches!(arg.as_slice(), [Expr::Text { .. }]))
                {
                    return Err(IdentityError::InvalidAst);
                }
                bytes = bytes
                    .saturating_add(name.len())
                    .saturating_add(3)
                    .saturating_add(args.len());
                if bytes > 8000 {
                    return Err(IdentityError::InvalidAst);
                }
                for arg in args {
                    if pending.len().saturating_add(arg.len()) > 8000 {
                        return Err(IdentityError::InvalidAst);
                    }
                    for child in arg {
                        pending.push((child, depth + 1));
                    }
                }
            }
        }
        if bytes > 8000 {
            return Err(IdentityError::InvalidAst);
        }
    }
    Ok(())
}

pub(super) fn snapshot(snapshot: &TableSnapshot) -> Result<(), IdentityError> {
    if snapshot.resources.len() > 10000 {
        return Err(IdentityError::ResourceLimit);
    }
    let mut refs: HashSet<u128, BuildHasherDefault<DefaultHasher>> = HashSet::default();
    let mut nodes = Map::default();
    for node in &snapshot.nodes {
        if !refs.insert(node.reference.0) {
            return Err(IdentityError::DuplicateRef);
        }
        nodes.insert(node.reference, (node, 0usize));
    }
    let mut semantics: Map<(&Population, &str), Vec<&EntryRecord>> = Map::default();
    for value in &snapshot.entries {
        entry(value)?;
        if !refs.insert(value.reference.0) {
            return Err(IdentityError::DuplicateRef);
        }
        let (node, count) = nodes
            .get_mut(&value.node)
            .ok_or(IdentityError::MissingNode)?;
        if node.population != value.population {
            return Err(IdentityError::PopulationMismatch);
        }
        if node.key != value.key {
            return Err(IdentityError::KeyMismatch);
        }
        *count += 1;
        if *count > 20 {
            return Err(IdentityError::NodeCapacity);
        }
        // Hashes route candidates only. Compare complete prepared structures.
        let bucket = semantics
            .entry((&value.population, value.original.as_str()))
            .or_default();
        if bucket.iter().any(|old| old.same_semantics(value)) {
            return Err(IdentityError::DuplicateEntry);
        }
        bucket.push(value);
    }
    for value in &snapshot.resources {
        original(&value.original)?;
        if !refs.insert(value.reference.0) {
            return Err(IdentityError::DuplicateRef);
        }
    }
    Ok(())
}
