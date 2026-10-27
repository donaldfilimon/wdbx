# Bounded Studio language/text/scanner v3 implementation profile

2026-10-22. Operationalizes frozen `designs.json` identity/language/scoring bundles
for local implementation and falsification. This is not a normative ABI schema,
production cutover, language-general efficacy result or amendment to frozen C0.
Legacy `language.rs`, engine, commands, identity ActionAst, IDs, saves and goldens
remain unchanged. New modules are opt-in Rust APIs; host authorization and later
versioned projection/import migration remain separate requirements.

## Preparation and utilities

Use immutable validated prepared structures, preserving original UTF-8, half-open
byte spans, caller input identity, action/clause IDs and fan-out origins. Getters
may expose immutable views; no Deserialize or arbitrary constructor may forge a
validated prepared/script result. A supplied host population/context and nine
versioned utility tables must agree before lookup. This is a synthetic/internal
host contract, not credential authentication. Invalid table weights/refs/versions
or limits fail before preparation or effects. Tables retain original phrases and
rule refs. Persistent resource-key insertion/migration is not supplied here.

All nine utilities must influence observable preparation: dictionary definition/
lemma evidence; weighted thesaurus equivalence alternatives; context-conditioned
anti-thesaurus senses; charge valence/intensity; verb required frames; conjunction
rules; negation rules; finite morphology suffix rules; grammatical role mapping.
Longest phrase wins; equal-length ties use stable rule-ref order. Ambiguous
compatible senses remain explicit alternatives; missing required slots are
Unresolved, never invented. Locale-independent Unicode lowercase applies only to
unquoted lexical interpretation; original bytes and quoted literals survive.
Morphology is only configured finite suffix replacement, not universal stemming.

Quotes are opaque literal tokens with preserved content/spans; quoted negation,
conjunctions and number words do not execute. Unclosed quotes are a typed syntax
failure. Parse And/Or/But/IfThen/NegatedAction explicitly. Negation attaches to the
affected action subtree, survives all variants and routes, and uncertain scope
conservatively inhibits affected actions. Conditions not resolved by supplied
context remain explicit/unexecutable; preparation does not assert their truth.
Within a semicolon-delimited group, IfThen binds the full antecedent and body
below conjunctions. Context attests only the full antecedent; any canonical
case comparison applies only to unquoted lexical text. Every quoted byte,
including literal case and Unicode content, must match exactly. Incomplete
or nested natural-language if groups return typed Syntax refusal; this bounded
profile does not flatten them or infer truth.

Numeric normalization occurs only in a recognized arithmetic grammar or explicitly
numeric frame slot. The bounded numeric-word profile is zero..ten; decimals retain
source plus exact canonical signed coefficient/scale (checked i128, scale<=18,
<=18 significant digits). Canonical numeric comparison never uses rounded f64.
Arithmetic captures preserve order and operators. Other number words remain text.

Limits: input 8000 characters,64 clauses,16 variants/clause,256 total variants,
AST depth 16,4096 aggregate utility rows, context 256 tokens,4096 draws and
4096 alternative opportunities per clause before sampling. Preparation also shares
one checked rewrite-work counter across all clauses and variants: caller
Budget.work_steps is 1..=1,000,000, default and ceiling 1,000,000. Charge before
replacement parsing, view expansion and safety matching; exhaustion returns
StepBudget with no Prepared result. Host supplies draws;
no global RNG/time/I/O. Include original plus bounded stochastic alternatives for
all inputs; sample rather than enumerate combinations. All-zero weights yield
original only. Draw/variant/step ceilings are checked. Substitutions preserve
quotes, numeric captures, operators, negation/inhibition and source lineage; reject
unsafe substitutions. No frozen corpus/holdout data is used to construct fixtures.

## Scripts and effect planning

Use a separate quote-aware typed v3 script tree and six scopes. Do not widen or
replace legacy Expr/ledger. Registry entries explicitly bind names, scopes and
subsystem reads. Literal quoted args remain inert; LookUp subsystem must be a
validated literal registered name. Curated aliases normalize once; unknown or
dynamic subsystem expressions refuse. Current execution must reject stale
registry identity/revision rather than rely on former declarations.

Each nested call receives inherited capabilities intersected with its declaration;
it cannot obtain a globally allowed but parent-undeclared contact. Validate all
contacts/budgets before emitting any EffectPlan. No callbacks, I/O, persistence
or real host effects occur. execute_for_action consumes immutable prepared action
constraints; inhibited/unresolved actions emit no prohibited effect. Return only a
complete plan on success; failures expose no partial output/effects. Pure bind
computations and their execution phase are traced once per invocation, not an
actual durable idempotency service.

Pratt operators: +,- precedence 10; *,/,% precedence 20; unary +/- binding 25;
right-associative ^ precedence 30. Maximum 500 arithmetic steps; zero division,
nonfinite results and type mismatch are explicit. Computation uses finite IEEE
f64 with libm::pow; retained decimal operands remain exact input evidence.

Automata adopt eight Brainfuck primitives,1024 wrapping u8 cells, wrapping pointer,
UTF-8 input bytes/zero at exhaustion, byte-vector output (not invented Unicode),
10000 steps,1024 output bytes, bracket/depth validation. Non-instruction nonspace
characters reject. Runaway/bad programs return typed failure and no partial plan.

## text-v3

Input is a validated v3 prepared action/chunk, not a legacy PatternId or serialized
caller JSON. Pattern original limit 2000 characters. AST lexical-token count is
leaf words/quoted literals/numbers/operators plus explicit connective/negation
operators; container wrappers do not add count. <=6 low,<=16 medium,else high.

Low: sorted unique lemmas with captured numeric literals replaced by #; quoted
literals carry a distinct quoted marker and preserve content. Medium: ordered
role-labelled lemmas and operator sequence, retaining quote/negation/conditional
constraints. High: complete normalized typed AST, including unresolved roles,
canonical numeric captures and all safety constraints. Exclude spans, host refs,
rule IDs, timestamps and random draw metadata from structural transforms.

Unambiguous encoding uses byte length as u64 big-endian followed by bytes for
all fields, list length framing and explicit node tags. SHA256 preimage is
LP(transform)||LP(class ASCII)||LP(text ASCII); resulting identity::PatternKey has
encoder_version=text-v3,modality=text,resolution=low|medium|high,full 32-byte digest.
Expose transform/preimage bytes for independent/parity witnesses. No automatic
legacy-ID conversion, saved-schema change or claim of lossless legacy-tree adapter.

## High-resolution scanner and candidate routes

Always scan full validated originals/prepared features, independently of candidate
key class. Action-scoped lexical features traverse every validated AST leaf,
including a retained conditional antecedent exactly once; punctuation is excluded
and quotes stay opaque. Body captures/operators/SPO remain explicit features; no
antecedent truth, role or numeric capture is invented. A full-chunk scanner is
not supplied in this packet. Expose scanner version, each rational component, spans, absence/conflict
reasons,S,D,checked difference and bounded confidence. ATP/jitter/strength/voting
and global scan scheduling are outside this scanner.

Multiset lemma Jaccard weight 40; ordered-bigram multiset Jaccard weight 20. For each,
intersection=sum(min counts),union=sum(max counts). Empty union gives 0 similarity
and full available dissimilarity. Roles weight 20: three SPO slots, one third each;
only equal resolved slots agree, unresolved/missing do not. Explicit negation
conflict in the compared body scope sets this role component to 0S/20D with a
labelled reason, not a hidden extra penalty. Uncertain body negation also leaves
role agreement unresolved (0S/20D with UncertainNegation); antecedent polarity
does not decide body SPO conflict or uncertainty. Operand/operator weight 20: all-or-nothing exact nonempty canonical
operand tuple AND operator sequence; absence or mismatch gives 0S/20D. All other
component dissimilarity is available weight minus similarity. Quoted tokens stay
opaque semantic literals rather than executable words.

Use checked signed i128 rational arithmetic with positive denominator and gcd
reduction; expose exact fractions, no intermediate rounding. Add S and D, subtract
once, clamp result to[-100,100]. Overflow returns explicit error. Direct reducer
tests exercise values beyond narrow integer capacity, cancellation and overflow.
No score is a probability or a combined constitutional trust dimension.

Explicit route index is population+scope fenced, stable ordered and bounded 4096
rules,64 prefixless wildcard rules,64 automaton rules. Grammar rules capture
numbers/operators explicitly; wildcard rules name literal prefix/contains and
explicit similarity-bypass policy. Automata use their own bucket. None depend on
accidental text-key equality; no route can remove inhibition, scope or budgets.
The local automaton adapter registers a bounded program, expected byte output and
current registry Tape handler. It quotes original UTF-8 as a literal invocation
and delegates to the reviewed script compiler/executor. Blocked actions retain
only an explicitly unexecuted Blocked origin, never a successful match or permit;
query-wide budgets bound all automata and failures expose no partial candidates.
A joining controller deduplicates concrete EntryRefs while retaining every clause,
fan-out, route, capture and constraint origin. No forced vote/admission follows
from retrieval; full scheduler and production engine integration remain open.

## Review/ruling costs

These bounded choices fill previously unspecified denominator, quote, lexical,
number, tape and framing details. They are versioned local implementation
choices, not retroactive interpretation of old IDs or authoritative published
wire contracts. Cost if wrong: rework this profile/new fixtures and a new version
before consumer adoption. Independent review checks consistency with frozen
requirements; scientific efficacy and ABI ownership remain separate gates.

Preflight consequence: missing-component penalties deliberately remain visible.
Identical nonnumeric text with all SPO resolved reaches at most60; unresolved
SPO makes identical two-token text20, and an identical single opaque token with
empty bigrams/unresolved SPO is-20. Do not special-case these results to improve
fixtures. They are profile consequences, not evidence of ordinary-language
accuracy or drop-in compatibility; efficacy stays open.
