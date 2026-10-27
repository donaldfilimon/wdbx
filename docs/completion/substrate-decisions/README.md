# Task 2 owner-decision proposal

Review-ready proposal; actual Task 2 remains incomplete. No schema owner, privacy owner or independent production crypto acceptance is recorded here. All decision rows await owner review. The approval template is empty and grants no authority. Task 1 C0 inventory approval does not ratify Task 2 interfaces.

Read `proposal-v1.json` for concrete proposed decisions, current private P1 constants/caps, dependency identities, ABI mismatches, missing artifacts and affected release tasks. `source-manifest-v1.json` pins deliberately inspected inputs by repository-relative identities. ABI files remain ABI-owned inputs. Private Rust structs and local Rust/Python witnesses are not a published ABI or shared producer/consumer adoption evidence.

The exact schema/profile/error/authority/receipt diff and shared ABI-producer/WDBX-consumer vectors are missing. In particular, proposed local shared-types `accepted/rejected/conflicted` is not the release operation vocabulary `committed/rejected/failed-before-commit/cancelled-before-commit`; unknown requires reconciliation. Private Accepted/HistoricalReplay and existing ABI execution outcome receipts supply neither mapping. No spelling or error alias is silently selected.

## Binding prerequisites

`docs/completion/shared-types-v1.json`, task-02 acceptance:

> Named owner signs schema diff; producer/consumer decode the same positive/negative synthetic vectors; no unknown variant accepted. Attach exact artifact hash and independent review. Until then implementation may prototype privately but no dependent merge or wire freeze.

`docs/superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md`, Slice 5:

> ABI-reviewed exact schema/profile/error/receipt diff and hashes; positive/malformed shared producer/consumer vectors; named owner receipt plus independent review. Only then design and review durable ordering, restart/rollback protection, replay/privacy retention and crash recovery, retaining one canonical writer.

`docs/completion/execution/owner-review.md`:

> The acceptance receipt must identify an actual owner/reviewer, explicit decisions, exact source/schema/vector/dependency hashes and review scope.

> Independent cryptographic review is additionally required before production qualification.

## Review sequence and limits

Named ABI/WDBX owners accept or revise each exact decision and commission missing normative artifacts and shared vectors. Privacy/lifecycle owners decide retention and erasure. Dependency and independent crypto reviewers assess exact versions, graph, targets, custody and provenance. Owner receipts must bind the final packet plus schema/vector/dependency hashes and explicit scope. Revised artifacts invalidate acceptance for affected decisions; rejection returns for amendment. No tool or template manufactures human acceptance.

Local slices 2–4 were independently reviewed as bounded private mechanisms in the continuation record. They supply strict P1 mathematics, synthetic trust and synthetic admission, not production credentials, custody, durable ordering, restart/rollback or publication. The PyCA low-order-key disagreement remains failed/open qualification evidence even after the bounded explicit libsodium experiment. Current TFHE and an unselected OpenRaft stable line are candidates, not approved or independently audited dependencies. No dependency queries, downloads, installs or upgrades were performed.

Each decision identifies affected tasks. Unchanged compatibility validation, previously authorized private synthetic checks and independent oracle/test design can proceed within their existing boundaries; blocked public/durable interfaces cannot be invented to advance implementation. No source pins, ABI fixtures, legacy goldens, evidence levels or runtime data are changed by this packet. Verification commands and artifact hashes are recorded in the plan workspace task-2 report; they validate documents/contracts, not Rust runtime or cryptographic qualification.
