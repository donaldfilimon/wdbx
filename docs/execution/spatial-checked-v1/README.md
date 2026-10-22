# spatial-checked-v1 execution

Isolated detached worktree: `/Users/donald/sources/repos/wdbx-spatial-checked-v1`
at `08a69b3f22db61093e06527b3308b10589687022`. No commits or pushes.
See [HANDOFF.md](HANDOFF.md) for final status, commands, limits and rollback.

T00–T06 implement the bounded checked spatial packet: source qualification,
stable baseline, compatibility fixtures, private validated f64 primitives,
independent references, checked complete scans and a bounded transient adapter
around the existing octree. Legacy APIs, f32 storage, crate identities and
signed formats are preserved. No persistent geometry or service is introduced.

The source manifest is `../../claims/wdbx-3d/source-manifest.json`; authority
reconciliation is `../../superpowers/specs/2026-10-20-wdbx-3d-source-audit.md`.
Task reports, reviews, progress and raw evidence distinguish reported history
from execution. Final source evidence is under `evidence/final-reviewed/`.
Earlier failures and the pre-fix passing gate remain historical evidence.

Local tests were explicitly reauthorized; CI remains paused. Canonical WDBX
and ABI were clean at initial inspection; later external dirty state is recorded
in `evidence/external-dirty-boundary.json` and preserved. This worktree is the
reviewable uncommitted deliverable. Optional FHE, consumer integration, hardware,
benchmarks, live data and production remain unqualified. T07+ are not started.
