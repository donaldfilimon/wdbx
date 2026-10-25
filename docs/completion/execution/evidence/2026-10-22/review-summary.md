# Independent automated reviews

These are automated source reviews in separate agent contexts, not named ABI
owner agreement or independent cryptographic qualification. No requirement is
promoted by this summary.

## Authenticated ledger and existing trust list

Reviewer `authentication_review` found no blocking defect in the strict historical
signature checks or ID-binding/Active-only rotation. It independently ran the
five authenticated-ledger tests and five trust unit tests successfully. It
identified the boundary that failed replay preserves ledger bytes, while opening
may create/chmod the store/lock; documentation records that limit. Compatibility
opens, rollback resistance and current authorization are outside the strict mode.

## Private P1 parser

Reviewer `register_review` compared the production parser to the proposed exact
P1 slice and found no correctness defect. Seven focused tests passed during
review. The reviewer requested additional zero-limit, hard-cap, nesting and
protected-header cases. Four tests were added, all eleven passed, and the final
full root gate passed. There is no signature verification or admission result.

## Execution register

Reviewer `register_review` initially found incomplete implementation mappings,
missing required artifact enforcement, blocker type confusion, in-root symlinks,
malformed-reference crashes and replaceable candidate identities. Validator bugs
received regression tests and fixes. A separately hash-pinned per-requirement
candidate catalog now protects mapping identity; references themselves pin bytes.
Final reviewer verdict: scaffold passes, no false promotion, no outstanding
validator correctness finding. Reviewer ran nineteen tests; a final malformed-row
regression increased the suite to twenty, all passing under both Python versions.
The register now has 95 candidate source/test mappings and 68 explicit unknowns.
Neither mappings nor review establish complete behavioral qualification.
