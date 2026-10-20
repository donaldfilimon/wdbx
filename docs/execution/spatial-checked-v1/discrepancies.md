# Execution discrepancy register

The canonical source/authority discrepancy register and scoped ADR02/03/18
record are in `../../superpowers/specs/2026-10-20-wdbx-3d-source-audit.md`.

Initial missing-artifact finding is resolved: the original v6 specification,
v5 master plan and requirements JSON were found in the user-supplied Downloads
package. Their digests and inspected ranges are in
`../../claims/wdbx-3d/source-manifest.json`. Earlier incomplete discovery is not
proof of their absence. No supplied source artifact was changed.

The earlier test pause was explicitly resolved in this conversation: local
tests are authorized; CI remains paused. No CI configuration was changed.

Baseline gate failed formatting in cpu.rs before running clippy/tests; a
formatting-only correction was independently reviewed, and the next baseline
gate passed. Raw failure and success logs are preserved separately.

The referenced ~/.claude/CLAUDE.md is absent here. Supplied repository policy
and explicit task authorization govern isolation and no-commit scope.

No authorization for member erasure, key destruction, production migration,
provider/model downloads, remote provisioning or later-profile execution has
been inferred from the first spatial packet.
