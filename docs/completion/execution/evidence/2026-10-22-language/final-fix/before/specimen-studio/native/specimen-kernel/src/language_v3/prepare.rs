use super::tables::{PreparationWork, is_negator, safe_lexical};
use super::*;

/// Validate supplied snapshots/budgets before producing any prepared result.
pub fn prepare(
    input: Input,
    context: &ContextSnapshot,
    tables: &UtilityTables,
    budget: Budget,
    draws: &mut dyn DrawSource,
) -> Result<Prepared, LanguageError> {
    if input.text.chars().count() > 8000 {
        return Err(LanguageError::InputLimit);
    }
    if context.population != tables.population() || context.utility_version != tables.version() {
        return Err(LanguageError::ContextMismatch);
    }
    if context.tokens.len() > 256
        || context.conditions.len() > 256
        || context.tokens.iter().any(|s| s.chars().count() > 256)
        || context
            .conditions
            .iter()
            .any(|(s, _)| s.chars().count() > 8000)
    {
        return Err(LanguageError::ContextLimit);
    }
    if budget.variants_per_clause == 0
        || budget.variants_per_clause > 16
        || budget.total_variants == 0
        || budget.total_variants > 256
        || budget.draws > 4096
        || budget.arithmetic_steps == 0
        || budget.arithmetic_steps > 500
        || budget.work_steps == 0
        || budget.work_steps > 1_000_000
    {
        return Err(LanguageError::InvalidBudget);
    }
    let mut work = PreparationWork::new(budget.work_steps);
    let tokens = tokenize::tokenize(&input.text, 0)?;
    let Split {
        parts,
        links,
        conditions,
    } = split(tokens, tables)?;
    if parts.len() > 64 {
        return Err(LanguageError::ClauseLimit);
    }
    let mut clauses = Vec::new();
    let mut draw_count = 0;
    let mut total = 0;
    let mut condition = ConditionalDisposition::None;
    let mut condition_ast = None;
    for (id, part) in parts.into_iter().enumerate() {
        if id > 0 {
            match links[id - 1] {
                Link::Then => {
                    let start = conditions[&id];
                    condition_ast = Some(join(&clauses, &links, start, id));
                    let first: &PreparedClause = &clauses[start];
                    let last: &PreparedClause = &clauses[id - 1];
                    let key = input.text[first.span.start..last.span.end]
                        .trim()
                        .to_lowercase();
                    condition = context
                        .conditions
                        .iter()
                        .find(|(s, _)| s.to_lowercase() == key)
                        .map(|(_, v)| {
                            if *v {
                                ConditionalDisposition::Satisfied
                            } else {
                                ConditionalDisposition::Refuted
                            }
                        })
                        .unwrap_or(ConditionalDisposition::Unresolved)
                }
                Link::Separate => {
                    condition = ConditionalDisposition::None;
                    condition_ast = None
                }
                _ => {}
            }
        }
        let mut original = action(&input, id, part, condition, tables, budget, &mut work)?;
        if id > 0
            && !matches!(links[id - 1], Link::Separate | Link::Then)
            && clauses[id - 1].variants[0].uncertain_negation
        {
            original.inhibited = true;
            original.uncertain_negation = true;
            original.ast = make_ast(&original.tokens, true, true);
            original.routes.push(RouteHint::Inhibited);
        }
        if let Some(cond) = &condition_ast {
            original.ast = SemanticAst::IfThen(Box::new(cond.clone()), Box::new(original.ast));
        }
        if conditions
            .iter()
            .any(|(body, start)| (*start..*body).contains(&id))
        {
            original.conditional = ConditionalDisposition::Unresolved;
            original.routes.push(RouteHint::Conditional)
        }
        let mut variants = vec![original.clone()];
        let options = alternatives(&original, tables, context)?;
        // Compatible senses are explicit; do not silently pick one by entropy.
        for option in options.iter().filter(|o| o.sense) {
            let variant = replace(&original, option, variants.len(), tables, &mut work)?;
            push_variant(&mut variants, variant, budget.variants_per_clause)?;
        }
        let stochastic = options
            .iter()
            .filter(|o| !o.sense && o.weight > 0.0)
            .collect::<Vec<_>>();
        let weight = stochastic.iter().map(|o| o.weight).sum::<f64>();
        if !weight.is_finite() {
            return Err(LanguageError::InvalidTables);
        }
        if weight > 0.0 {
            // Draw a bounded permutation of alternatives, never enumerate products.
            let mut remaining = stochastic;
            while !remaining.is_empty() && variants.len() < budget.variants_per_clause {
                if draw_count >= budget.draws {
                    return Err(LanguageError::DrawBudget);
                }
                let draw = draws.draw();
                draw_count += 1;
                if !draw.is_finite() || !(0.0..1.0).contains(&draw) {
                    return Err(LanguageError::DrawBudget);
                }
                let target = draw * remaining.iter().map(|o| o.weight).sum::<f64>();
                let mut accum = 0.0;
                let mut selected = remaining.len() - 1;
                for (i, o) in remaining.iter().enumerate() {
                    accum += o.weight;
                    if target < accum {
                        selected = i;
                        break;
                    }
                }
                let option = remaining.remove(selected);
                let variant = replace(&original, option, variants.len(), tables, &mut work)?;
                push_variant(&mut variants, variant, budget.variants_per_clause)?;
            }
        }
        total += variants.len();
        if total > budget.total_variants {
            return Err(LanguageError::VariantLimit);
        }
        clauses.push(PreparedClause {
            span: original.span,
            variants,
        });
    }
    let ast = join(&clauses, &links, 0, clauses.len());
    if ast.depth() > 16 {
        return Err(LanguageError::DepthLimit);
    }
    Ok(Prepared {
        input,
        clauses,
        ast,
        draws: draw_count,
    })
}
#[derive(Clone, Copy)]
enum Link {
    And,
    Or,
    But,
    Then,
    Separate,
}
struct Split {
    parts: Vec<Vec<Token>>,
    links: Vec<Link>,
    conditions: std::collections::BTreeMap<usize, usize>,
}
fn split(tokens: Vec<Token>, tables: &UtilityTables) -> Result<Split, LanguageError> {
    let mut parts = Vec::new();
    let mut links = Vec::new();
    let mut current = Vec::new();
    let mut i = 0;
    let mut pending_if = None;
    let mut group_if = false;
    let mut conditions = std::collections::BTreeMap::new();
    while i < tokens.len() {
        let t = &tokens[i];
        let mut count = 1;
        let mut link = None;
        if !t.quoted() {
            let custom = tables.matches(UtilityKind::Conjunction, &tokens, i);
            let connective = custom
                .first()
                .and_then(|(r, n)| {
                    if let UtilityEffect::Conjunction(c) = r.effect {
                        count = *n;
                        Some(c)
                    } else {
                        None
                    }
                })
                .or_else(|| match t.lemma() {
                    "and" => Some(Connective::And),
                    "or" => Some(Connective::Or),
                    "but" => Some(Connective::But),
                    "if" => Some(Connective::If),
                    "then" => Some(Connective::Then),
                    _ => None,
                });
            link = match connective {
                Some(Connective::And) => Some(Link::And),
                Some(Connective::Or) => Some(Link::Or),
                Some(Connective::But) => Some(Link::But),
                Some(Connective::Then) => {
                    if let Some(start) = pending_if.take() {
                        conditions.insert(parts.len() + 1, start);
                        Some(Link::Then)
                    } else {
                        return Err(LanguageError::Syntax);
                    }
                }
                Some(Connective::If) => {
                    if !current.is_empty()
                        || group_if
                        || !links.is_empty() && !matches!(links.last(), Some(Link::Separate))
                    {
                        return Err(LanguageError::Syntax);
                    }
                    group_if = true;
                    pending_if = Some(parts.len());
                    i += count;
                    continue;
                }
                None => {
                    if t.source == ";" {
                        Some(Link::Separate)
                    } else {
                        None
                    }
                }
            };
        }
        if let Some(link) = link {
            if matches!(link, Link::Separate) {
                if pending_if.is_some() {
                    return Err(LanguageError::Syntax);
                }
                group_if = false;
            }
            if current.is_empty() {
                return Err(LanguageError::Syntax);
            }
            parts.push(std::mem::take(&mut current));
            links.push(link);
        } else {
            current.push(t.clone())
        }
        i += count;
    }
    if !current.is_empty() {
        parts.push(current)
    } else if !links.is_empty() {
        return Err(LanguageError::Syntax);
    }
    if pending_if.is_some() {
        return Err(LanguageError::Syntax);
    }
    Ok(Split {
        parts,
        links,
        conditions,
    })
}
fn join(clauses: &[PreparedClause], links: &[Link], start: usize, end: usize) -> SemanticAst {
    if start == end {
        return SemanticAst::Action(vec![]);
    }
    if end - start == 1 {
        let a = &clauses[start].variants[0];
        return make_ast(&a.tokens, a.inhibited, a.uncertain_negation);
    }
    let priority = |l: Link| match l {
        Link::Separate => 0,
        Link::Then => 1,
        Link::But => 2,
        Link::Or => 3,
        Link::And => 4,
    };
    let min = links[start..end - 1]
        .iter()
        .map(|l| priority(*l))
        .min()
        .unwrap_or(0);
    let candidates = (start..end - 1)
        .filter(|i| priority(links[*i]) == min)
        .collect::<Vec<_>>();
    let pivot = candidates[candidates.len() / 2];
    let a = Box::new(join(clauses, links, start, pivot + 1));
    let b = Box::new(join(clauses, links, pivot + 1, end));
    match links[pivot] {
        Link::Separate | Link::And => SemanticAst::And(a, b),
        Link::Or => SemanticAst::Or(a, b),
        Link::But => SemanticAst::But(a, b),
        Link::Then => SemanticAst::IfThen(a, b),
    }
}
fn action(
    input: &Input,
    id: usize,
    mut tokens: Vec<Token>,
    conditional: ConditionalDisposition,
    tables: &UtilityTables,
    budget: Budget,
    work: &mut PreparationWork,
) -> Result<PreparedAction, LanguageError> {
    let span = Span {
        start: tokens.first().map_or(0, |t| t.span.start),
        end: tokens.last().map_or(0, |t| t.span.end),
    };
    let original = input.text[span.start..span.end].to_string();
    let mut inhibited = false;
    let mut uncertain = false;
    let mut definitions = Vec::new();
    let mut charges = Vec::new();
    // Safety operators are recognized before dictionary/morphological interpretation.
    for i in 0..tokens.len() {
        if tokens[i].quoted() {
            continue;
        }
        let matches = tables.matches(UtilityKind::Negation, &tokens, i);
        if let Some((row, n)) = matches.first()
            && let UtilityEffect::Negation { uncertain: u } = row.effect
        {
            inhibited = true;
            uncertain |= u;
            for t in &mut tokens[i..i + n] {
                t.role = Role::Negation;
                t.rule_refs.push(row.reference.clone())
            }
        }
        if is_negator(tokens[i].lemma()) {
            tokens[i].role = Role::Negation;
            inhibited = true;
        }
    }
    uncertain |= inhibited
        && tokens
            .iter()
            .any(|t| !t.quoted() && matches!(t.lemma(), "perhaps" | "maybe" | "possibly"));
    for i in 0..tokens.len() {
        if tokens[i].quoted() || tokens[i].role == Role::Negation {
            continue;
        }
        if let Some((row, n)) = tables.matches(UtilityKind::Dictionary, &tokens, i).first()
            && let UtilityEffect::Dictionary { lemma, definition } = &row.effect
        {
            tables.validate_rewrite(&tokens, i..i + 1, lemma, work)?;
            definitions.push(Definition {
                rule_ref: row.reference.clone(),
                definition: definition.clone(),
                span: Span {
                    start: tokens[i].span.start,
                    end: tokens[i + n - 1].span.end,
                },
            });
            tokens[i].lemma = lemma.to_lowercase();
            tokens[i].rule_refs.push(row.reference.clone());
        }
        if let Some((row, n)) = tables.matches(UtilityKind::Chargebook, &tokens, i).first()
            && let UtilityEffect::Charge { valence, intensity } = row.effect
        {
            charges.push(Charge {
                rule_ref: row.reference.clone(),
                valence,
                intensity,
                span: Span {
                    start: tokens[i].span.start,
                    end: tokens[i + n - 1].span.end,
                },
            })
        }
        let suffix = tables
            .rows
            .iter()
            .filter(|r| {
                matches!(r.effect, UtilityEffect::Suffix { .. })
                    && tokens[i].lemma.ends_with(&r.phrase.to_lowercase())
                    && tokens[i].lemma.len() > r.phrase.to_lowercase().len()
            })
            .max_by(|a, b| {
                a.phrase
                    .len()
                    .cmp(&b.phrase.len())
                    .then_with(|| b.reference.cmp(&a.reference))
            });
        if tokens[i].kind == TokenKind::Word
            && safe_lexical(tokens[i].lemma())
            && let Some(row) = suffix
            && let UtilityEffect::Suffix { replacement } = &row.effect
        {
            let old = &tokens[i].lemma;
            let candidate = format!(
                "{}{}",
                &old[..old.len() - row.phrase.to_lowercase().len()],
                replacement.to_lowercase()
            );
            tables.validate_rewrite(&tokens, i..i + 1, &candidate, work)?;
            tokens[i].lemma = candidate;
            tokens[i].rule_refs.push(row.reference.clone())
        }
    }
    // A literal/operator in a leading tail, or an explicit command, supplies intent;
    // number words alone stay lexical. The normalizer still checks the whole tail.
    let start = tokens.iter().position(|t| {
        !t.quoted()
            && (t.kind == TokenKind::Number
                || matches!(
                    t.lemma(),
                    "zero"
                        | "one"
                        | "two"
                        | "three"
                        | "four"
                        | "five"
                        | "six"
                        | "seven"
                        | "eight"
                        | "nine"
                        | "ten"
                        | "-"
                        | "+"
                        | "("
                ))
    });
    let mut value = None;
    if let Some(start) = start {
        let arithmetic_tail = start == 0 && arithmetic::has_explicit_syntax(&tokens[start..])
            || tokens[..start]
                .iter()
                .any(|t| !t.quoted() && matches!(t.lemma(), "calculate" | "compute" | "evaluate"));
        if arithmetic_tail {
            value = arithmetic::normalize(&mut tokens[start..], budget.arithmetic_steps)?;
        }
    }
    let (spo, unresolved_frame) = roles(&mut tokens, tables)?;
    let captures = arithmetic::captures(&tokens);
    let operators = tokens
        .iter()
        .filter(|t| t.kind == TokenKind::Operator)
        .map(|t| t.lemma.clone())
        .collect::<Vec<_>>();
    let ast = make_ast(&tokens, inhibited, uncertain);
    let mut routes = vec![RouteHint::Lexical];
    if value.is_some() {
        routes.push(RouteHint::Arithmetic)
    }
    if inhibited {
        routes.push(RouteHint::Inhibited)
    }
    if unresolved_frame {
        routes.push(RouteHint::Unresolved)
    }
    if conditional != ConditionalDisposition::None {
        routes.push(RouteHint::Conditional)
    }
    Ok(PreparedAction {
        original,
        span,
        tokens,
        ast,
        captures,
        operators,
        spo,
        inhibited,
        uncertain_negation: uncertain,
        conditional,
        unresolved_frame,
        value,
        definitions,
        charges,
        origin: Origin {
            input_id: input.id.clone(),
            clause_id: id,
            action_id: id,
            fanout_id: 0,
            rule_refs: vec![],
        },
        routes,
    })
}
fn make_ast(tokens: &[Token], inhibited: bool, uncertain: bool) -> SemanticAst {
    let leaf = SemanticAst::Action(
        tokens
            .iter()
            .filter(|t| t.role != Role::Negation)
            .cloned()
            .collect(),
    );
    if inhibited {
        SemanticAst::NegatedAction {
            uncertain,
            action: Box::new(leaf),
        }
    } else {
        leaf
    }
}
fn roles(tokens: &mut [Token], tables: &UtilityTables) -> Result<(Spo, bool), LanguageError> {
    for i in 0..tokens.len() {
        if let Some((r, n)) = tables.matches(UtilityKind::Grammar, tokens, i).first()
            && let UtilityEffect::Role(role) = r.effect
        {
            for t in &mut tokens[i..i + n] {
                if t.role != Role::Negation {
                    t.role = role;
                    t.rule_refs.push(r.reference.clone())
                }
            }
        }
    }
    let frame = (0..tokens.len()).find_map(|i| {
        tables
            .matches(UtilityKind::VerbFrames, tokens, i)
            .first()
            .map(|(r, n)| (i, *n, *r))
    });
    let mut unresolved = false;
    if let Some((i, n, r)) = frame
        && let UtilityEffect::Frame {
            subject,
            object,
            numeric_object,
        } = r.effect
    {
        for t in &mut tokens[i..i + n] {
            t.role = Role::Predicate;
            t.rule_refs.push(r.reference.clone())
        }
        let s = tokens
            .iter()
            .position(|t| t.role == Role::Subject)
            .or_else(|| {
                tokens[..i]
                    .iter()
                    .rposition(|t| t.role == Role::Other && t.kind == TokenKind::Word)
            });
        let o = tokens
            .iter()
            .position(|t| t.role == Role::Object)
            .or_else(|| {
                (i + n..tokens.len()).find(|j| {
                    tokens[*j].role == Role::Other
                        && matches!(
                            tokens[*j].kind,
                            TokenKind::Word | TokenKind::Quoted | TokenKind::Number
                        )
                })
            });
        if let Some(s) = s {
            tokens[s].role = Role::Subject
        } else {
            unresolved |= subject
        }
        if let Some(o) = o {
            tokens[o].role = Role::Object;
            if numeric_object {
                let numeric = match tokens[o].lemma.as_str() {
                    "zero" => "0",
                    "one" => "1",
                    "two" => "2",
                    "three" => "3",
                    "four" => "4",
                    "five" => "5",
                    "six" => "6",
                    "seven" => "7",
                    "eight" => "8",
                    "nine" => "9",
                    "ten" => "10",
                    s => s,
                };
                if tokens[o].quoted() {
                    unresolved = true
                } else {
                    match CanonicalDecimal::parse(numeric) {
                        Ok(d) => {
                            tokens[o].decimal = Some(d);
                            tokens[o].kind = TokenKind::Number
                        }
                        Err(_) => unresolved = true,
                    }
                }
            }
        } else {
            unresolved |= object
        }
    }
    let slot = |role| {
        tokens
            .iter()
            .find(|t| t.role == role)
            .map(|t| Slot::Resolved(t.lemma.clone()))
            .unwrap_or(Slot::Unresolved)
    };
    Ok((
        Spo {
            subject: slot(Role::Subject),
            predicate: slot(Role::Predicate),
            object: slot(Role::Object),
        },
        unresolved,
    ))
}
struct Alternative {
    at: usize,
    count: usize,
    replacement: String,
    reference: String,
    weight: f64,
    sense: bool,
}
fn alternatives(
    a: &PreparedAction,
    tables: &UtilityTables,
    context: &ContextSnapshot,
) -> Result<Vec<Alternative>, LanguageError> {
    let mut options = Vec::new();
    for i in 0..a.tokens.len() {
        if !safe_lexical(a.tokens[i].lemma())
            || a.tokens[i].quoted()
            || a.tokens[i].role == Role::Negation
        {
            continue;
        }
        for kind in [UtilityKind::AntiThesaurus, UtilityKind::Thesaurus] {
            for (r, n) in tables.matches(kind, &a.tokens, i) {
                if a.tokens[i..i + n].iter().any(|t| {
                    t.quoted()
                        || t.decimal.is_some()
                        || t.role == Role::Negation
                        || !safe_lexical(t.lemma())
                }) {
                    continue;
                }
                let (replacement, weight, sense) = match &r.effect {
                    UtilityEffect::Equivalent {
                        replacement,
                        weight,
                    } => (replacement, *weight, false),
                    UtilityEffect::Sense {
                        lemma,
                        required_context,
                        weight,
                    } => {
                        if !required_context.iter().all(|c| {
                            context
                                .tokens
                                .iter()
                                .any(|x| x.to_lowercase() == c.to_lowercase())
                        }) {
                            continue;
                        }
                        (lemma, *weight, true)
                    }
                    _ => continue,
                };
                if weight > 0.0 {
                    // A fixed work ceiling also bounds candidate storage before sampling.
                    if options.len() >= 4096 {
                        return Err(LanguageError::StepBudget);
                    }
                    options.push(Alternative {
                        at: i,
                        count: n,
                        replacement: replacement.clone(),
                        reference: r.reference.clone(),
                        weight,
                        sense,
                    })
                }
            }
        }
    }
    Ok(options)
}
fn replace(
    a: &PreparedAction,
    o: &Alternative,
    fanout: usize,
    tables: &UtilityTables,
    work: &mut PreparationWork,
) -> Result<PreparedAction, LanguageError> {
    let mut result = a.clone();
    let source_span = Span {
        start: a.tokens[o.at].span.start,
        end: a.tokens[o.at + o.count - 1].span.end,
    };
    let mut replacement =
        tables.validate_rewrite(&a.tokens, o.at..o.at + o.count, &o.replacement, work)?;
    let source = a
        .original
        .get(source_span.start - a.span.start..source_span.end - a.span.start)
        .ok_or(LanguageError::UnsafeSubstitution)?;
    for t in &mut replacement {
        t.span = source_span;
        t.source = source.to_string();
        t.role = a.tokens[o.at].role;
        t.rule_refs.push(o.reference.clone())
    }
    result.tokens.splice(o.at..o.at + o.count, replacement);
    result.ast = make_ast(&result.tokens, result.inhibited, result.uncertain_negation);
    if let SemanticAst::IfThen(cond, _) = &a.ast {
        result.ast = SemanticAst::IfThen(cond.clone(), Box::new(result.ast));
    }
    result.origin.fanout_id = fanout;
    result.origin.rule_refs.push(o.reference.clone());
    // Preserve frame resolution and update its lexical projection without inventing a slot.
    let slot = |role| {
        result
            .tokens
            .iter()
            .find(|t| t.role == role)
            .map(|t| Slot::Resolved(t.lemma.clone()))
            .unwrap_or(Slot::Unresolved)
    };
    result.spo = Spo {
        subject: slot(Role::Subject),
        predicate: slot(Role::Predicate),
        object: slot(Role::Object),
    };
    Ok(result)
}
fn push_variant(
    variants: &mut Vec<PreparedAction>,
    variant: PreparedAction,
    cap: usize,
) -> Result<(), LanguageError> {
    if variants.iter().any(|a| {
        a.tokens
            .iter()
            .map(Token::lemma)
            .eq(variant.tokens.iter().map(Token::lemma))
    }) {
        return Ok(());
    }
    if variants.len() >= cap {
        return Err(LanguageError::VariantLimit);
    }
    variants.push(variant);
    Ok(())
}
