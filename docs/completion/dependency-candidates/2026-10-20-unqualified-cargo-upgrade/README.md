# Archived unqualified Cargo dependency candidate

This directory preserves the complete dirty root lock observed on 2026-10-20. At capture it was an unqualified candidate, excluded from the active qualified checkpoint. This directory is archival evidence, not a Cargo workspace. Its author and generating command are unknown. No necessity, security improvement or preferred-version claim is made.

`Cargo.lock` retains exact candidate bytes. `baseline-to-candidate.patch` retains the exact patch from the baseline commit named in `metadata.json`; that metadata records SHA-256 identities and 25 version changes plus nine dependency-edge-only changes. Applying the patch to the baseline reproduced the saved candidate byte-for-byte before root restoration.

The initial reconciliation restored the qualified committed root lock; that intermediate disposition was superseded by the later direction below. The earlier offline failure to obtain uncached `ctutils 0.4.3` was not runtime evidence against that release.

Dated superseding status, 2026-10-20: later direct user instructions authorized verified backups, removal of source locks and compatible regeneration. Fresh root generation with pinned Rust 1.99.1 and the fallback resolver reproduced this archived lock byte-for-byte (`e2fac4f3f125cecc771a0dc7d6f33eb1629972b3979ac0771849fdf61368680e`), now the active root resolution. The historical capture, unknown authorship and original lock/patch bytes remain preserved.

[Refresh results and log hashes](../2026-10-20-compatible-refresh/results.json) and the [execution report](../../../../.superpowers/sdd/2026-10-20-completion-remaining/lock-report.md) record successful locked fetch and root workspace/all-targets compilation. Full root gate remains pending controller execution. These checks do not establish security qualification, optional FHE runtime, Studio or platform qualification.
