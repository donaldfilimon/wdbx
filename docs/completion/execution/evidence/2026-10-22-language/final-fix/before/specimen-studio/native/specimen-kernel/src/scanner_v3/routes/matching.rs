use super::*;
type Match = (CandidatePath, Vec<RouteCapture>, SimilarityBypass);
pub(super) fn invocation(handler: &str, program: &str, input: &str) -> Result<String, RouteError> {
    // Only a registered handler name is executable; both arguments are literal
    // JSON strings consumed by the existing producer parser, never interpolated.
    let p = serde_json::to_string(program).map_err(|_| RouteError::InvalidRule)?;
    let i = serde_json::to_string(input).map_err(|_| RouteError::InvalidRule)?;
    Ok(format!("{handler}({p},{i})"))
}
pub(super) fn matches(
    rule: &RouteRule,
    a: &PreparedAction,
    current: &Registry,
    caps: &CapabilityLedger,
    work: &mut Work,
) -> Result<Vec<Match>, RouteError> {
    match &rule.kind {
        RouteKind::Grammar(atoms) => {
            work.charge(atoms.len() + a.tokens().len())?;
            if atoms.len() != a.tokens().len() {
                return Ok(vec![]);
            }
            let mut captures = vec![];
            for (atom, token) in atoms.iter().zip(a.tokens()) {
                let capture = match atom {
                    GrammarAtom::Literal(s)
                        if !token.quoted() && token.decimal().is_none() && token.lemma() == s =>
                    {
                        None
                    }
                    GrammarAtom::Quoted(s) if token.quoted() && token.lemma() == s => None,
                    GrammarAtom::Number(name) if token.decimal().is_some() => Some(RouteCapture {
                        name: name.clone(),
                        span: token.span(),
                        value: CaptureValue::Number(token.decimal().unwrap().clone()),
                    }),
                    GrammarAtom::Operator { name, allowed }
                        if token.kind() == TokenKind::Operator
                            && allowed.iter().any(|op| op == token.lemma()) =>
                    {
                        Some(RouteCapture {
                            name: name.clone(),
                            span: token.span(),
                            value: CaptureValue::Operator(token.lemma().into()),
                        })
                    }
                    _ => return Ok(vec![]),
                };
                if let Some(c) = capture {
                    captures.push(c);
                }
            }
            Ok(vec![(
                CandidatePath::Grammar(rule.id.clone()),
                captures,
                SimilarityBypass::RequireFullScan,
            )])
        }
        RouteKind::Wildcard {
            mode,
            literal,
            bypass,
        } => {
            work.charge(
                a.original()
                    .len()
                    .checked_mul(literal.len().max(1))
                    .ok_or(RouteError::BudgetExceeded)?,
            )?;
            let positions = match mode {
                WildcardMode::Prefix => {
                    if a.original().starts_with(literal) {
                        vec![0]
                    } else {
                        vec![]
                    }
                }
                WildcardMode::Contains => a
                    .original()
                    .match_indices(literal)
                    .map(|(i, _)| i)
                    .collect(),
            };
            // Reserve all substring copies before allocating captured strings.
            work.charge(
                positions
                    .len()
                    .checked_mul(a.original().len() + 3)
                    .ok_or(RouteError::BudgetExceeded)?,
            )?;
            Ok(positions
                .into_iter()
                .map(|start| {
                    let end = start + literal.len();
                    let capture = |name: &str, s: usize, e: usize| RouteCapture {
                        name: name.into(),
                        span: Span {
                            start: a.span().start + s,
                            end: a.span().start + e,
                        },
                        value: CaptureValue::Text(a.original()[s..e].into()),
                    };
                    (
                        CandidatePath::Wildcard(rule.id.clone()),
                        vec![
                            capture("literal", start, end),
                            capture("before", 0, start),
                            capture("after", end, a.original().len()),
                        ],
                        *bypass,
                    )
                })
                .collect())
        }
        RouteKind::Automaton {
            handler,
            program,
            expected,
        } => {
            if !a.executable() {
                return Ok(vec![(
                    CandidatePath::Automaton {
                        rule: rule.id.clone(),
                        status: AutomatonStatus::Blocked,
                    },
                    vec![],
                    SimilarityBypass::RequireFullScan,
                )]);
            }
            work.tape(program, a.original())?;
            let script = compile(
                &invocation(handler, program, a.original())?,
                Scope::NodeAction,
                current,
            )?;
            let plan = execute_for_action(
                &script,
                a,
                current,
                caps,
                ScriptBudget {
                    tape_steps: work.tape_steps,
                    output_bytes: 1024,
                    ..ScriptBudget::default()
                },
            )?;
            let Value::Bytes(bytes) = plan.value() else {
                return Err(RouteError::Language(LanguageError::TypeMismatch));
            };
            if bytes != expected {
                return Ok(vec![]);
            }
            Ok(vec![(
                CandidatePath::Automaton {
                    rule: rule.id.clone(),
                    status: AutomatonStatus::Matched,
                },
                vec![RouteCapture {
                    name: "output".into(),
                    span: a.span(),
                    value: CaptureValue::Bytes(bytes.clone()),
                }],
                SimilarityBypass::RequireFullScan,
            )])
        }
    }
}
