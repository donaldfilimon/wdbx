# Permanent inhibition commitment v2 — proposed C0

This supersedes the v1 pair-plus-policy commitment before experiments. The v1
document and vectors remain unchanged and unqualified; no production rows exist
under this proposed format. Historical witness source is contract_vectors_v1.py.

Owner: WDBX durable semantics and tenant key administration. Studio supplies typed
candidate structures, never chooses MAC keys or canonical commitment bytes.
This is a proposed cryptographic use of standard HMAC-SHA256, not independent
crypto approval. Production qualification still requires independent review.

## Canonical candidate and fan-out equivalence

At mutation admission, freeze the original pattern/action under an immutable
`equivalence_version` and explicit classes. Class IDs are ASCII (1–128 bytes),
not user-chosen authorization identities. A versioned table maps exact normalized
Language Governance AST bytes to pattern/action class IDs. There is no unbounded
semantic equivalence oracle. Membership requires exact structural equality or a
reviewed rewrite rule; compatible coarse Pattern IDs alone are insufficient.
The table can include numeric-word normalization (two→2, four→4, plus→+) in
arithmetic ASTs, but must preserve operands, roles, structural negation and
authenticated tenant/scope. Current policy-code metadata is deliberately excluded
from pair identity, as specified below.
Fan-out variants are admitted to the same equivalence class only if that invariant
holds; uncertain variants produce distinct classes. A pair not resolved under
**every retained relevant version** fails growth with UnresolvedInhibition, rather
than bypassing old bans. Normalization-rule versions and tables are retained as
public algorithm metadata; per-tenant learned plaintext is not retained there.
Custom private equivalences must be representable by a general structural rule
or remain exact-structure-only keyed matches. Never persist the forgotten phrase
inside an equivalence table to enable matching.

Concrete normative test table v1:

- Pattern AST `[add,2,2]`, from `two plus two` or `2 + 2`, maps to
  `pattern:add-two:v1`; `[add,2,3]` maps to `pattern:add-two-three:v1`.
- Action AST `[literal_integer,4]`, from `four` or `4`, maps to
  `action:sum-two:v1`; `[literal_integer,5]` maps to `action:sum-three:v1`.
- Pattern/action class IDs themselves encode a public structural class in this
  synthetic example. Production private pairs use HMAC over their canonical AST
  bytes, not a public lookup table of phrases; the same framing treats the bytes
  as opaque. Thus class IDs need not disclose private semantics in retained rows.

The permanent ban is **pair-wide**. Changes, additions or removal of policy/
inhibition code metadata never change the pair identity. Scope authorization
still applies independently, and structural negation remains part of the action
meaning. Canonical pattern/action ASTs exclude current policy code lists; do not
hide policy metadata inside the structural class ID.

Canonical pair bytes are `frame([domain,mode,version,pattern,action])`. `frame`
concatenates unsigned32 big-endian byte length plus bytes for every field. Domain
is ASCII `wdbx/inhibited-pair/v2`; mode is `public-class` or `exact-ast`; version is
`v1` for the retained normalization/class rules in these vectors. Pattern/action
use canonical class IDs for public vectors or compact typed JSON AST bytes for
private exact matching. ASTs use arrays only: node-kind strings, children, UTF-8
string leaves and signed64 integers; no floats/maps/null/whitespace, ensure_ascii
false with required JSON quote/backslash/control escaping. Never renormalize
literal leaves after the versioned language boundary. Pair <=16KiB; class IDs
<=128 ASCII bytes. Mode/version are bound in the MAC and storage partition.

Policy codes are **not** in these bytes and are **not** required for matching.
Retaining code metadata is optional and governed separately by retention policy;
it may be erased without affecting the permanent ban. If a metadata integrity
commitment is needed, use a separately domain-separated metadata record, never
replace the pair-wide tag with it. This version does not require or define a
metadata commitment and retains no code list merely to support inhibition.

At proposal time compute only the candidate's normalized pair bytes for permanent
matching. Do not substitute its current codes for unknown historical codes and
do not enumerate code sets: neither is needed. Compare this same pair against
all retained relevant normalization modes/versions and MAC epochs. A matching
pair rejects regardless of candidate codes, even an empty list after erasure.

## Keys, scope, storage and matching

Each tenant has an independent randomly generated 256-bit inhibition MAC key per
epoch, managed by KMS. This key is **not** any per-object payload key. For tenant T
and authenticated scope S derive:

```
Kscope = HMAC-SHA256(Ktenant_epoch,
  frame(["wdbx/inhibition-scope/v1", UTF8(T), UTF8(S)]))
tag = HMAC-SHA256(Kscope, canonical_pair_bytes_without_policy_codes)
```

T and S are server-resolved stable identifiers, not request-supplied paths. The
row's storage partition also binds T/S, mode and equivalence version. Read/match
requires current scoped authorization before key use. Persist only tag, key epoch,
normalization/equivalence version, mode, opaque retired node/slot refs, deletion
generation, mutation operation ID and minimum lineage refs. No original text,
original Pattern key, public plaintext hash or payload key is retained. Access to
rows and the KMS is separate from public source access. Administrative diagnostics
never expose MACs or matching outcomes without scoped authorization; rate limits
and auditing prevent an unbounded phrase-guessing oracle. HMAC protects low-entropy
inputs against offline dictionary recovery **only while its secret stays secret**.

To check a new pairing after payload erasure: authorize scope; normalize candidate
under every relevant retained version/mode; compute tag with every relevant
retained key epoch; constant-time compare against that scope's partition. Any match
rejects growth. Destroying the object payload key leaves this minimal anti-growth
metadata and inhibition keys intact. A deletion ledger remains a separate source
of anti-resurrection authority; failure to resolve/match is not permission to grow.

KMS wrapping-key rotation rewraps unchanged inhibition keys, preserving all tags.
Actual MAC-key rotation creates epoch N+1 for new rows and retains prior keys
wrapped and match-only, indefinitely for rows that still require protection. New
attempts check old and current epochs. Old tags cannot be re-MACed after erasure
without their preimage; therefore do not delete old keys or pretend rewrapping is
MAC rotation. Budget: at most 32 active historical MAC epochs per scope; attempting
a 33rd blocks rotation and growth pending an explicit reviewed migration, never
silently forgets bans. A known compromise of an old MAC key is a privacy incident:
retaining bans cannot undo offline guesses already enabled by that compromise.
Escalate/restrict the scope and obtain a reviewed recovery policy; do not claim
rotation repairs old tags' secrecy.

Full tenant retirement may destroy all inhibition keys only after permanently
closing that namespace to future writes/imports. Reusing its identity is forbidden.
Otherwise a key destruction request conflicting with permanent inhibition needs an
explicit policy decision; the implementation must report the conflict, not claim
both perpetual matching and key destruction are possible.

`inhibition-vectors-v2.json` uses visibly synthetic keys. The Python reference
reproduces exact preimage/MAC bytes, equivalent-attempt rejection after object-key
destruction, old-epoch matching after real MAC rotation, different tenant/scope
tags and a non-equivalent action. Empty and changed-code attempts reproduce the
same pair tag in each MAC epoch; matching against retained old-epoch rows remains
effective after both payload-key destruction and MAC rotation. The two equivalent text spellings are resolved
by the concrete table above. Matching vectors are not security review evidence.
