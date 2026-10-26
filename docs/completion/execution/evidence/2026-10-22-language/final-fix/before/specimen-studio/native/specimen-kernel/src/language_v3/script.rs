//! Separate quote-aware compiler, registry-bound capabilities and atomic pure plans.
mod tape;
use super::{LanguageError, PreparedAction};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    RelationalTriple,
    PatternBind,
    NodeAction,
    Vote,
    Orchestration,
    Reserved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handler {
    Text,
    Concat,
    Math,
    Tape,
    LookUp,
    Bind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub aliases: Vec<String>,
    pub scopes: Vec<Scope>,
    pub reads: Vec<String>,
    pub handler: Handler,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    identity: String,
    revision: u64,
    subsystems: BTreeSet<String>,
    entries: BTreeMap<String, Declaration>,
    aliases: BTreeMap<String, String>,
}
impl Registry {
    pub fn new(
        identity: &str,
        revision: u64,
        subsystems: Vec<String>,
        declarations: Vec<Declaration>,
    ) -> Result<Self, LanguageError> {
        if !name(identity) || subsystems.len() > 256 || declarations.len() > 256 {
            return Err(LanguageError::InvalidRegistry);
        }
        let mut systems = BTreeSet::new();
        for s in subsystems {
            if !name(&s) || !systems.insert(s) {
                return Err(LanguageError::InvalidRegistry);
            }
        }
        let mut entries = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        for d in declarations {
            if !name(&d.name)
                || d.scopes.is_empty()
                || d.scopes.contains(&Scope::Reserved)
                || d.scopes.iter().collect::<BTreeSet<_>>().len() != d.scopes.len()
                || d.reads.len() > 256
                || d.aliases.len() > 32
                || d.reads.iter().any(|s| !systems.contains(s))
                || d.reads.iter().collect::<BTreeSet<_>>().len() != d.reads.len()
            {
                return Err(LanguageError::InvalidRegistry);
            }
            for alias in std::iter::once(&d.name).chain(d.aliases.iter()) {
                if !name(alias) || aliases.insert(alias.clone(), d.name.clone()).is_some() {
                    return Err(LanguageError::InvalidRegistry);
                }
            }
            if entries.insert(d.name.clone(), d).is_some() {
                return Err(LanguageError::InvalidRegistry);
            }
        }
        Ok(Self {
            identity: identity.into(),
            revision,
            subsystems: systems,
            entries,
            aliases,
        })
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn declarations(&self) -> impl Iterator<Item = &Declaration> {
        self.entries.values()
    }
    pub fn capabilities(&self, reads: Vec<String>) -> Result<CapabilityLedger, LanguageError> {
        if reads.len() > 256 || reads.iter().any(|s| !self.subsystems.contains(s)) {
            return Err(LanguageError::UndeclaredContact);
        }
        Ok(CapabilityLedger {
            registry: self.clone(),
            reads: reads.into_iter().collect(),
        })
    }
}
fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
#[derive(Debug, Clone)]
pub struct CapabilityLedger {
    registry: Registry,
    reads: BTreeSet<String>,
}
impl CapabilityLedger {
    pub fn reads(&self) -> impl Iterator<Item = &str> {
        self.reads.iter().map(String::as_str)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptNode {
    Literal(String),
    Call { name: String, args: Vec<ScriptNode> },
}
#[derive(Debug, Clone)]
pub struct CompiledScript {
    source: String,
    scope: Scope,
    registry: Registry,
    ast: ScriptNode,
}
impl CompiledScript {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn ast(&self) -> &ScriptNode {
        &self.ast
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ScriptBudget {
    pub calls: usize,
    pub arithmetic_steps: usize,
    pub tape_steps: usize,
    pub output_bytes: usize,
}
impl Default for ScriptBudget {
    fn default() -> Self {
        Self {
            calls: 500,
            arithmetic_steps: 500,
            tape_steps: 10000,
            output_bytes: 32768,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
    Bytes(Vec<u8>),
    Sequence(Vec<Value>),
    ReadReference { subsystem: String, key: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Read { subsystem: String, key: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trace {
    Call(String),
    BindComputed,
    BindExecuted,
}
#[derive(Debug, Clone)]
pub struct EffectPlan {
    value: Value,
    effects: Vec<Effect>,
    trace: Vec<Trace>,
    registry_identity: String,
    registry_revision: u64,
}
impl EffectPlan {
    pub fn value(&self) -> &Value {
        &self.value
    }
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
    pub fn trace(&self) -> &[Trace] {
        &self.trace
    }
    pub fn registry_identity(&self) -> &str {
        &self.registry_identity
    }
    pub fn registry_revision(&self) -> u64 {
        self.registry_revision
    }
}
/// Compile only literal-registered subsystem names; quoted call text remains inert.
pub fn compile(
    source: &str,
    scope: Scope,
    registry: &Registry,
) -> Result<CompiledScript, LanguageError> {
    if source.chars().count() > 8000 {
        return Err(LanguageError::InputLimit);
    }
    if scope == Scope::Reserved {
        return Err(LanguageError::InvalidScope);
    }
    let mut parser = Parser {
        source,
        pos: 0,
        nodes: 0,
    };
    let mut ast = parser.node(1)?;
    parser.space();
    if parser.pos != source.len() {
        return Err(LanguageError::Syntax);
    }
    validate(&mut ast, scope, registry)?;
    Ok(CompiledScript {
        source: source.into(),
        scope,
        registry: registry.clone(),
        ast,
    })
}
fn validate(node: &mut ScriptNode, scope: Scope, r: &Registry) -> Result<(), LanguageError> {
    if let ScriptNode::Call { name, args } = node {
        *name = r
            .aliases
            .get(name)
            .ok_or(LanguageError::InvalidScope)?
            .clone();
        let declaration = r.entries.get(name).ok_or(LanguageError::InvalidScope)?;
        if !declaration.scopes.contains(&scope) {
            return Err(LanguageError::InvalidScope);
        }
        match declaration.handler {
            Handler::LookUp => {
                if args.len() != 2 {
                    return Err(LanguageError::TypeMismatch);
                }
                let ScriptNode::Literal(subsystem) = &args[0] else {
                    return Err(LanguageError::DynamicContact);
                };
                if !r.subsystems.contains(subsystem) || !declaration.reads.contains(subsystem) {
                    return Err(LanguageError::UndeclaredContact);
                }
                if !matches!(args[1], ScriptNode::Literal(_)) {
                    return Err(LanguageError::TypeMismatch);
                }
            }
            Handler::Math | Handler::Bind => {
                if args.len() != 1 || !matches!(args[0], ScriptNode::Literal(_)) {
                    return Err(LanguageError::TypeMismatch);
                }
            }
            Handler::Tape => {
                if args.len() != 2 || args.iter().any(|a| !matches!(a, ScriptNode::Literal(_))) {
                    return Err(LanguageError::TypeMismatch);
                }
            }
            Handler::Text => {
                if args.len() != 1 || !matches!(args[0], ScriptNode::Literal(_)) {
                    return Err(LanguageError::TypeMismatch);
                }
            }
            Handler::Concat => {}
        }
        for arg in args {
            validate(arg, scope, r)?
        }
    }
    Ok(())
}
/// Return a whole plan or a typed failure; all state below is invocation-local.
pub fn execute_script(
    script: &CompiledScript,
    current: &Registry,
    caps: &CapabilityLedger,
    budget: ScriptBudget,
) -> Result<EffectPlan, LanguageError> {
    if &script.registry != current || &caps.registry != current {
        return Err(LanguageError::StaleRegistry);
    }
    if budget.calls == 0
        || budget.calls > 500
        || budget.arithmetic_steps == 0
        || budget.arithmetic_steps > 500
        || budget.tape_steps == 0
        || budget.tape_steps > 10000
        || budget.output_bytes == 0
        || budget.output_bytes > 32768
    {
        return Err(LanguageError::InvalidBudget);
    }
    let mut work = Work {
        current,
        budget,
        calls: 0,
        arithmetic_left: budget.arithmetic_steps,
        tape_left: budget.tape_steps,
        effects: vec![],
        trace: vec![],
    };
    // Contact validation traverses the entire tree before computing any bind/output.
    contacts(&script.ast, &caps.reads, current)?;
    let value = work.eval(&script.ast, &caps.reads)?;
    Ok(EffectPlan {
        value,
        effects: work.effects,
        trace: work.trace,
        registry_identity: current.identity.clone(),
        registry_revision: current.revision,
    })
}
pub fn execute_for_action(
    script: &CompiledScript,
    action: &PreparedAction,
    current: &Registry,
    caps: &CapabilityLedger,
    budget: ScriptBudget,
) -> Result<EffectPlan, LanguageError> {
    if !action.executable() {
        return Err(LanguageError::ActionBlocked);
    }
    if script.scope != Scope::NodeAction {
        return Err(LanguageError::InvalidScope);
    }
    execute_script(script, current, caps, budget)
}
fn contacts(
    node: &ScriptNode,
    inherited: &BTreeSet<String>,
    r: &Registry,
) -> Result<(), LanguageError> {
    if let ScriptNode::Call { name, args } = node {
        let d = &r.entries[name];
        let narrowed = inherited
            .intersection(&d.reads.iter().cloned().collect())
            .cloned()
            .collect::<BTreeSet<_>>();
        if d.handler == Handler::LookUp
            && let ScriptNode::Literal(s) = &args[0]
            && !narrowed.contains(s)
        {
            return Err(LanguageError::UndeclaredContact);
        }
        for a in args {
            contacts(a, &narrowed, r)?
        }
    }
    Ok(())
}
struct Work<'a> {
    current: &'a Registry,
    budget: ScriptBudget,
    calls: usize,
    arithmetic_left: usize,
    tape_left: usize,
    effects: Vec<Effect>,
    trace: Vec<Trace>,
}
impl Work<'_> {
    fn bounded_value(&self, value: Value) -> Result<Value, LanguageError> {
        if value_bytes(&value)? > self.budget.output_bytes {
            return Err(LanguageError::OutputBudget);
        }
        Ok(value)
    }
    fn eval(
        &mut self,
        node: &ScriptNode,
        inherited: &BTreeSet<String>,
    ) -> Result<Value, LanguageError> {
        match node {
            ScriptNode::Literal(s) => self.bounded_value(Value::Text(s.clone())),
            ScriptNode::Call { name, args } => {
                self.calls += 1;
                if self.calls > self.budget.calls {
                    return Err(LanguageError::StepBudget);
                }
                self.trace.push(Trace::Call(name.clone()));
                let d = self.current.entries[name].clone();
                let narrowed = inherited
                    .intersection(&d.reads.into_iter().collect())
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let literal = |i| match &args[i] {
                    ScriptNode::Literal(s) => Ok(s.as_str()),
                    _ => Err(LanguageError::TypeMismatch),
                };
                let value = match d.handler {
                    Handler::Text => Value::Text(literal(0)?.into()),
                    Handler::Math | Handler::Bind => {
                        let source = literal(0)?;
                        if self.arithmetic_left == 0 {
                            return Err(LanguageError::StepBudget);
                        }
                        // The parser itself charges exact Pratt steps, not a guessed token count.
                        let (value, used) =
                            super::arithmetic::evaluate_counted(source, self.arithmetic_left)?;
                        self.arithmetic_left -= used;
                        if d.handler == Handler::Bind {
                            self.trace
                                .extend([Trace::BindComputed, Trace::BindExecuted]);
                        }
                        Value::Number(value)
                    }
                    Handler::Tape => {
                        let (bytes, used) =
                            tape::run(literal(0)?, literal(1)?.as_bytes(), self.tape_left)?;
                        self.tape_left -= used;
                        Value::Bytes(bytes)
                    }
                    Handler::LookUp => {
                        let subsystem = literal(0)?.to_string();
                        let key = literal(1)?.to_string();
                        if !narrowed.contains(&subsystem) {
                            return Err(LanguageError::UndeclaredContact);
                        }
                        self.effects.push(Effect::Read {
                            subsystem: subsystem.clone(),
                            key: key.clone(),
                        });
                        Value::ReadReference { subsystem, key }
                    }
                    Handler::Concat => {
                        let mut s = String::new();
                        let mut values = Vec::new();
                        let mut all_text = true;
                        for a in args {
                            let value = self.eval(a, &narrowed)?;
                            match &value {
                                Value::Text(t) => s.push_str(t),
                                Value::Number(n) => s.push_str(&n.to_string()),
                                _ => all_text = false,
                            }
                            values.push(value)
                        }
                        if all_text {
                            Value::Text(s)
                        } else {
                            Value::Sequence(values)
                        }
                    }
                };
                self.bounded_value(value)
            }
        }
    }
}
// Container wrappers never erase payload charges or bill carried children twice.
fn value_bytes(value: &Value) -> Result<usize, LanguageError> {
    match value {
        Value::Text(s) => Ok(s.len()),
        Value::Bytes(b) => Ok(b.len()),
        Value::Number(_) => Ok(8),
        Value::ReadReference { subsystem, key } => subsystem
            .len()
            .checked_add(key.len())
            .ok_or(LanguageError::OutputBudget),
        Value::Sequence(values) => values.iter().try_fold(0usize, |sum, v| {
            sum.checked_add(value_bytes(v)?)
                .ok_or(LanguageError::OutputBudget)
        }),
    }
}
struct Parser<'a> {
    source: &'a str,
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self.source[self.pos..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            self.pos += self.source[self.pos..]
                .chars()
                .next()
                .map_or(0, char::len_utf8)
        }
    }
    fn consume(&mut self, c: char) -> bool {
        self.space();
        if self.source[self.pos..].starts_with(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }
    fn node(&mut self, depth: usize) -> Result<ScriptNode, LanguageError> {
        if depth > 16 {
            return Err(LanguageError::DepthLimit);
        }
        self.nodes += 1;
        if self.nodes > 500 {
            return Err(LanguageError::StepBudget);
        }
        self.space();
        if self.source[self.pos..].starts_with('"') {
            let start = self.pos;
            self.pos += 1;
            let mut escaped = false;
            let mut closed = false;
            while let Some(c) = self.source[self.pos..].chars().next() {
                self.pos += c.len_utf8();
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == '"' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err(LanguageError::UnclosedQuote);
            }
            let s = serde_json::from_str::<String>(&self.source[start..self.pos])
                .map_err(|_| LanguageError::Syntax)?;
            return Ok(ScriptNode::Literal(s));
        }
        let start = self.pos;
        while self.source[self.pos..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.pos += 1
        }
        if start == self.pos {
            return Err(LanguageError::Syntax);
        }
        let name = self.source[start..self.pos].to_string();
        if !self.consume('(') {
            return Err(LanguageError::Syntax);
        }
        let mut args = vec![];
        if !self.consume(')') {
            loop {
                args.push(self.node(depth + 1)?);
                if self.consume(')') {
                    break;
                }
                if !self.consume(',') {
                    return Err(LanguageError::Syntax);
                }
            }
        }
        Ok(ScriptNode::Call { name, args })
    }
}
