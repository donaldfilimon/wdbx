# T00 report — source qualification

Status: source audit and public inventory produced; concrete source conflicts
and unavailable historical evidence remain explicitly recorded. No full-system
or production conformance is claimed. Stop boundary remains T00–T06.

Outputs:

- `docs/superpowers/specs/2026-10-20-wdbx-3d-source-audit.md`
- `docs/claims/wdbx-3d/source-manifest.json`
- Root-owned validator and negative-fixture tests under `tools/`.

Complete controlling-document reads were performed collectively by root and
source-audit workers. V6 and requirements registry were read fully; v5 reading
covers only shared contracts/DAG/T00–T06/original matrix/source register.
Manifest rows distinguish these reads from hash-only compatibility inventory.
External supplied files carry SHA-256 identity without invented Git provenance.

Found the actual acceptance ledger at `specimen-studio/VALIDATION.md`; its
current integration pointer is `docs/verification/2026-10-08-main-integration.md`.
Both remain reported historical evidence. Private external final receipt bytes
and the original massive-plan artifact were not supplied or opened. Their
absence limits original provenance/historical reproduction, not unrelated
approved checked-point implementation. Current Studio is embedded, with its
own workspace and pinned WDBX dependency revision, not the archived checkout.

Material conflicts: constitution C0–C7 definitions differ from claims.md;
nightly/std::simd prose predates current stable source; gateway proto has ten
RPCs versus North-Star's eight-method wording; Studio depends on an older pinned
revision. Original docs remain untouched. The audit records scoped ADR02/03/18
from user direction and leaves erasure/constitutional approvals separately gated.

Verification by this worker:

- `python3 tools/validate_wdbx_3d_evidence.py docs/claims/wdbx-3d/source-manifest.json`:
  exit 0 after stricter review and explicit dependency-inventory lockfile scope.
  Output: Source manifest inventory accepted (content/execution not independently verified).
- `git diff --check`: exit 0 at documentation checkpoint.

Root owns exact validator red/green test logs, positive case counts and final
source/run receipts. This worker ran no broad Cargo gate, Studio runtime,
hardware experiment, CI, commit, push, deployment or live-store operation.
