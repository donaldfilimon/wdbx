# Package 2 owner-review boundary

This packet requests no deployment and records no approval. It makes the next
review concrete. The existing authoritative proposed design is
`docs/superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md`; its slice 5
requires an ABI-reviewed exact schema/profile/error/receipt diff and hashes,
shared positive/malformed vectors, a named owner receipt and independent review
before durable ordering/integration. Its slice 2 also requires a reviewed crypto
dependency decision before signature-verifier implementation.

## Current reviewable changes

- Private full-payload P1 parser: `crates/abi-wdbx/src/v3/complete_episode/crypto/`.
  Profile is exactly the existing proposed P1: tag 18, four entries, protected
  `{1: -19, 4: kid}`, empty unprotected map, detached null payload, 64-byte
  signature, fixed profile AAD, full canonical complete payload as signed message.
  Input/buffer caps and original-byte retention follow that design. Eleven tests
  and a separately reproducible synthetic Python vector cover parsing/encoding.
  The continuation adds private full-payload signing and strict mathematical
  verification, immutable reports, bounded synthetic trust and admission models.
  These produce no durable admission. See `continuation.md` for the current
  reviewed scope and the final evidence manifest for exact source identities.
- Existing complete-ledger compatibility API gains explicit authenticated open.
  This is raw-digest Ed25519, not P1 full-payload signing. Each existing signature
  must verify against the host's supplied historical keys; the writer key must
  be included. No record or canonical digest changes. This mode is selected by
  the host at each open, not persisted or integrated with governance.
- The older digest-payload COSE prototype's in-memory trust list now refuses key
  ID mismatches and non-Active predecessors for rotation. It remains separate
  from P1 and authorizes no durable admission.

## Decisions still requiring actual ownership evidence

1. Accept or revise full-payload P1 versus the canonical proposal's raw-digest
   equation; publish no alias between the profiles.
2. Accept or revise domain/AAD/header/key-ID representation, new-purpose key
   separation, signature requirements, and compatibility/downgrade rules.
3. Bind normative schemas, typed receipts/errors, authenticated principal/scope,
   namespace/grants, consent/policy revisions, replay and privacy retention.
4. Review exact crypto dependency versions, transitive graph/advisories/licenses,
   MSRV/target support, strict message verification and weak-key rejection.
5. Qualify the independent second-language cryptographic verifier beyond the
   local bounded matrix. The explicit isolated libsodium witness now verifies
   signatures; the bundled PyCA weak-key disagreement remains a retained failure.
   Neither result establishes production qualification or general equivalence.
6. Review durable trust/admission ordering, restart/rollback protection, key
   custody and crash recovery only after the preceding interface agreement.

The acceptance receipt must identify an actual owner/reviewer, explicit decisions,
exact source/schema/vector/dependency hashes and review scope. Generic program
implementation authorization and automated source review are not substituted for
that receipt. Independent cryptographic review is additionally required before
production qualification. No ABI schema, consumer or normative fixture is changed
by this execution tranche.

## Continuation artifacts

The private dependency decisions are `p1-dependency-decision.md` and
`p1-libsodium-decision.md`. Trust semantics and independently reproduced command
bytes are recorded in `p1-trust-slice.md`; the admission simulator has its narrow
contract in `crates/abi-wdbx/src/v3/complete_episode/crypto/admission/SPEC.md`.
`docs/execution/p1-local-slices-summary.md` supplements this packet. Final raw
results, review scope and source hashes are under
`evidence/2026-10-22-continuation/`. All paths are repository-relative.

The next durable slice remains gated on the named owner receipt above. Local
review acceptance permits only the private in-memory experiment; it supplies
no credential service, durable key history, restore guarantee or public ABI.
