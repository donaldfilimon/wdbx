//! Population/scope fenced explicit routes and a pure structural/route join.
mod matching;
mod model;
mod work;
use crate::{
    identity::*,
    language_v3::{script::*, *},
    text_v3,
};
pub use model::*;
use std::collections::{BTreeMap, BTreeSet};

/// Derived, bounded index. Registration checks a complete batch before mutation.
/// Rules are ordered by stable rule ID; concrete references retain supplied order.
pub struct RouteIndex {
    population: Population,
    scopes: BTreeSet<Scope>,
    registry: Registry,
    rules: Vec<RouteRule>,
    grammar: Vec<usize>,
    prefix: BTreeMap<u8, Vec<usize>>,
    prefixless: Vec<usize>,
    automata: Vec<usize>,
}
impl RouteIndex {
    pub fn new(
        population: Population,
        scopes: Vec<Scope>,
        registry: &Registry,
    ) -> Result<Self, RouteError> {
        let declared = scopes.iter().copied().collect::<BTreeSet<_>>();
        if declared.is_empty()
            || declared.len() != scopes.len()
            || declared.contains(&Scope::Reserved)
        {
            return Err(RouteError::UndeclaredScope);
        }
        Ok(Self {
            population,
            scopes: declared,
            registry: registry.clone(),
            rules: vec![],
            grammar: vec![],
            prefix: BTreeMap::new(),
            prefixless: vec![],
            automata: vec![],
        })
    }
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    pub fn register(
        &mut self,
        table: &HybridTable,
        current: &Registry,
        new: Vec<RouteRule>,
    ) -> Result<(), RouteError> {
        if current != &self.registry {
            return Err(RouteError::StaleRegistry);
        }
        if new.len() > 4096 || self.rules.len() + new.len() > 4096 {
            return Err(RouteError::RuleLimit);
        }
        let entries = table.entries();
        let mut ids = self
            .rules
            .iter()
            .map(|r| r.id.clone())
            .collect::<BTreeSet<_>>();
        let mut prefixless = self.prefixless.len();
        let mut automata = self.automata.len();
        let mut references = self.rules.iter().map(|r| r.entries.len()).sum::<usize>();
        for rule in &new {
            if rule.id.is_empty()
                || rule.id.len() > 128
                || !ids.insert(rule.id.clone())
                || rule.entries.is_empty()
                || rule.entries.len() > 64
                || rule
                    .entries
                    .iter()
                    .map(|e| e.0)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != rule.entries.len()
            {
                return Err(RouteError::InvalidRule);
            }
            if rule.scopes.is_empty()
                || rule.scopes.iter().any(|s| !self.scopes.contains(s))
                || rule.scopes.iter().collect::<BTreeSet<_>>().len() != rule.scopes.len()
            {
                return Err(RouteError::UndeclaredScope);
            }
            references = references
                .checked_add(rule.entries.len())
                .ok_or(RouteError::RuleLimit)?;
            if references > 4096 {
                return Err(RouteError::RuleLimit);
            }
            for reference in &rule.entries {
                let entry = entries
                    .iter()
                    .find(|e| e.reference == *reference)
                    .ok_or(RouteError::MissingEntry)?;
                if entry.population != self.population {
                    return Err(RouteError::PopulationMismatch);
                }
            }
            match &rule.kind {
                RouteKind::Grammar(atoms) => validate_grammar(atoms)?,
                RouteKind::Wildcard { mode, literal, .. } => {
                    if literal.chars().count() > 2000
                        || (*mode == WildcardMode::Contains && literal.is_empty())
                    {
                        return Err(RouteError::InvalidRule);
                    }
                    if *mode == WildcardMode::Contains || literal.is_empty() {
                        prefixless += 1;
                    }
                }
                RouteKind::Automaton {
                    handler,
                    program,
                    expected,
                } => {
                    automata += 1;
                    if program.len() > 2000
                        || expected.len() > 1024
                        || rule.scopes != [Scope::NodeAction]
                        || !current.declarations().any(|d| {
                            d.name == *handler
                                && d.handler == Handler::Tape
                                && d.scopes.contains(&Scope::NodeAction)
                        })
                    {
                        return Err(RouteError::InvalidRule);
                    }
                    // Quote-safe compilation binds names/scopes to this registry.
                    compile(
                        &matching::invocation(handler, program, "")?,
                        Scope::NodeAction,
                        current,
                    )?;
                }
            }
            if prefixless > 64 || automata > 64 {
                return Err(RouteError::RuleLimit);
            }
        }
        // Validation is complete. Publishing this owned batch cannot fail.
        self.rules.extend(new);
        self.rules.sort_by(|a, b| a.id.cmp(&b.id));
        self.reindex();
        Ok(())
    }
    fn reindex(&mut self) {
        self.grammar.clear();
        self.prefix.clear();
        self.prefixless.clear();
        self.automata.clear();
        for (i, r) in self.rules.iter().enumerate() {
            match &r.kind {
                RouteKind::Grammar(_) => self.grammar.push(i),
                RouteKind::Automaton { .. } => self.automata.push(i),
                RouteKind::Wildcard {
                    mode: WildcardMode::Prefix,
                    literal,
                    ..
                } if !literal.is_empty() => self
                    .prefix
                    .entry(literal.as_bytes()[0])
                    .or_default()
                    .push(i),
                RouteKind::Wildcard { .. } => self.prefixless.push(i),
            }
        }
    }
    fn possible(&self, original: &str) -> Vec<usize> {
        let mut out = self.grammar.clone();
        out.extend(&self.prefixless);
        out.extend(&self.automata);
        if let Some(first) = original.as_bytes().first()
            && let Some(rules) = self.prefix.get(first)
        {
            out.extend(rules);
        }
        out.sort_unstable();
        out
    }
}
fn validate_grammar(atoms: &[GrammarAtom]) -> Result<(), RouteError> {
    if atoms.is_empty() || atoms.len() > 256 {
        return Err(RouteError::InvalidRule);
    }
    let characters = atoms
        .iter()
        .map(|a| match a {
            GrammarAtom::Literal(s) | GrammarAtom::Quoted(s) => s.chars().count(),
            _ => 1,
        })
        .sum::<usize>();
    if characters > 2000 {
        return Err(RouteError::InvalidRule);
    }
    let mut names = BTreeSet::new();
    for a in atoms {
        match a {
            GrammarAtom::Literal(s) | GrammarAtom::Quoted(s) => {
                if s.is_empty() || s.chars().count() > 2000 {
                    return Err(RouteError::InvalidRule);
                }
            }
            GrammarAtom::Number(name) | GrammarAtom::Operator { name, .. } => {
                if name.is_empty() || name.len() > 128 || !names.insert(name) {
                    return Err(RouteError::InvalidRule);
                }
            }
        }
        if let GrammarAtom::Operator { allowed, .. } = a
            && (allowed.is_empty()
                || allowed.len() > 8
                || allowed
                    .iter()
                    .any(|s| !matches!(s.as_str(), "+" | "-" | "*" | "/" | "%" | "^" | "(" | ")")))
        {
            return Err(RouteError::InvalidRule);
        }
    }
    Ok(())
}
struct Work {
    steps: usize,
    output: usize,
    origins: usize,
    tape_steps: usize,
}
impl Work {
    fn new(b: RouteBudget) -> Result<Self, RouteError> {
        if b.work_steps == 0
            || b.work_steps > 1_000_000
            || b.output_bytes == 0
            || b.output_bytes > 65536
            || b.origins == 0
            || b.origins > 65536
            || b.tape_steps == 0
            || b.tape_steps > 10000
        {
            return Err(RouteError::InvalidBudget);
        }
        Ok(Self {
            steps: b.work_steps,
            output: b.output_bytes,
            origins: b.origins,
            tape_steps: b.tape_steps,
        })
    }
    fn charge(&mut self, n: usize) -> Result<(), RouteError> {
        self.steps = self
            .steps
            .checked_sub(n)
            .ok_or(RouteError::BudgetExceeded)?;
        Ok(())
    }
    fn tape(&mut self, program: &str, input: &str) -> Result<(), RouteError> {
        self.charge(program.len() + input.len() + self.tape_steps + 500)?;
        self.output = self
            .output
            .checked_sub(1024)
            .ok_or(RouteError::BudgetExceeded)?;
        Ok(())
    }
}
/// Complete result or typed failure, never partial candidates. Structural first,
/// then rule-ID order per clause/variant; only concrete EntryRef is deduplicated.
#[allow(clippy::too_many_arguments)]
pub fn candidates(
    table: &HybridTable,
    index: &RouteIndex,
    population: &Population,
    scope: Scope,
    prepared: &Prepared,
    current: &Registry,
    caps: &CapabilityLedger,
    budget: RouteBudget,
) -> Result<Vec<Candidate>, RouteError> {
    if population != &index.population {
        return Err(RouteError::PopulationMismatch);
    }
    if !index.scopes.contains(&scope) {
        return Err(RouteError::UndeclaredScope);
    }
    if current != &index.registry {
        return Err(RouteError::StaleRegistry);
    }
    let mut work = Work::new(budget)?;
    let entries = table.entries();
    // O(1) slice metadata lets us refuse before traversing or indexing even an
    // empty query. Charge an upper bound on BTreeMap insertion comparisons.
    let index_steps = entries
        .len()
        .checked_mul(1 + (usize::BITS - entries.len().leading_zeros()) as usize)
        .ok_or(RouteError::BudgetExceeded)?;
    work.charge(index_steps)?;
    let records = entries
        .iter()
        .map(|e| (e.reference.0, e))
        .collect::<BTreeMap<_, _>>();
    // Stale refs/populations refuse the whole query before any matching.
    for rule in &index.rules {
        work.charge(1 + rule.entries.len())?;
        for reference in &rule.entries {
            work.charge(1 + (usize::BITS - records.len().leading_zeros()) as usize)?;
            let entry = records.get(&reference.0).ok_or(RouteError::MissingEntry)?;
            work.population(&entry.population)?;
            work.population(population)?;
            if &entry.population != population {
                return Err(RouteError::PopulationMismatch);
            }
        }
    }
    let mut result = vec![];
    let mut positions = BTreeMap::new();
    for clause in prepared.clauses() {
        for action in clause.variants() {
            work.charge(1 + action.tokens().len() + action.original().len())?;
            work.semantic_ast(action.ast())?;
            let key = text_v3::encode(action)?;
            // Stream complete structural matches in insertion order; no uncharged
            // all-match Vec or hidden table traversal occurs before reservation.
            work.charge(entries.len())?;
            for entry in entries {
                work.population(&entry.population)?;
                work.population(population)?;
                if &entry.population != population {
                    continue;
                }
                work.key(&entry.key)?;
                work.key(key.pattern_key())?;
                if &entry.key != key.pattern_key() {
                    continue;
                }
                append(
                    &mut result,
                    &mut positions,
                    &records,
                    entry.reference,
                    action,
                    &CandidatePath::Structural(key.pattern_key().clone()),
                    &[],
                    SimilarityBypass::RequireFullScan,
                    &mut work,
                )?;
            }
            work.charge(
                index.rules.len()
                    * (1 + (usize::BITS - index.rules.len().leading_zeros()) as usize),
            )?;
            for number in index.possible(action.original()) {
                let rule = &index.rules[number];
                work.charge(1)?;
                if !rule.scopes.contains(&scope) {
                    continue;
                }
                for matched in matching::matches(rule, action, current, caps, &mut work)? {
                    for entry in &rule.entries {
                        append(
                            &mut result,
                            &mut positions,
                            &records,
                            *entry,
                            action,
                            &matched.0,
                            &matched.1,
                            matched.2,
                            &mut work,
                        )?;
                    }
                }
            }
        }
    }
    Ok(result)
}
#[allow(clippy::too_many_arguments)]
fn append(
    out: &mut Vec<Candidate>,
    positions: &mut BTreeMap<u128, usize>,
    records: &BTreeMap<u128, &EntryRecord>,
    reference: EntryRef,
    action: &PreparedAction,
    path: &CandidatePath,
    captures: &[RouteCapture],
    bypass: SimilarityBypass,
    work: &mut Work,
) -> Result<(), RouteError> {
    // Reserve every variable-size origin field before materialization, including
    // caller identity and fanout rule refs, which need not be short source text.
    work.origin(action, path, captures)?;
    work.origins = work
        .origins
        .checked_sub(1)
        .ok_or(RouteError::BudgetExceeded)?;
    work.charge(2 + 2 * (usize::BITS - records.len().leading_zeros()) as usize)?;
    let position = positions.get(&reference.0).copied();
    let record = records.get(&reference.0).ok_or(RouteError::MissingEntry)?;
    if position.is_none() {
        work.charge(record.original.len())?;
        work.target_ast(&record.ast)?;
    }
    let origin = CandidateOrigin {
        origin: action.origin().clone(),
        span: action.span(),
        token_spans: action.tokens().iter().map(Token::span).collect(),
        constraints: action.into(),
        path: path.clone(),
        captures: captures.to_vec(),
        bypass,
    };
    if let Some(position) = position {
        out[position].origins.push(origin);
    } else {
        positions.insert(reference.0, out.len());
        out.push(Candidate {
            entry: reference,
            node: record.node,
            original: record.original.clone(),
            target_ast: record.ast.clone(),
            origins: vec![origin],
        });
    }
    Ok(())
}
