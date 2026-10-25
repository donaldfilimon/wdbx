# Private P1 trust slice 3

This implements the synthetic in-memory slice of the
[private design](../../superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md).
It is unratified and crate-private. No credential authentication, filesystem,
key custody, remote authority, admission or production qualification is supplied.
The legacy digest-signing trust list is unchanged.

Local status, 2026-10-22: independent automated specification/quality review
accepted this private slice without actionable findings. Twelve focused trust and
crypto-sibling tests passed; the standard-library vector builder agreed; feature
all-target Clippy passed with existing/pedantic warnings. Initial Clippy failures
and corrected final outputs are retained under the trust evidence directory.
These results cover the reviewed local slice snapshot; later combined source
changes require their own gates and explicit hash reconciliation. They are not
a named ABI-owner approval, independent production crypto review or task-02
closure.

## Interfaces and ordering

`TrustState::apply(command, authorization, expected_revision)` accepts typed
Register/Activate/Retire/Revoke/Rotate commands. The explicitly named
`AdminAuthorization::for_synthetic_host_policy` constructor provisions immutable
namespace, administrator principal and exact scope grants for trusted test hosts;
it does not authenticate credentials or consume episode role claims.

The model checks current namespace and all affected scope grants before command
history lookup. Before returning a replay or conflict it also checks access to
original successful-command scopes. Exact replay compares both canonical bytes
and digest, returns the original revision/digest as `HistoricalReplay`, and never
changes current state. Changed principal, arguments or expected revision conflicts.
Fresh authorization evidence itself is not command identity. Only successful
commands reserve IDs; every refusal preserves keys, history and revision.

Register creates Pending. Activate accepts Pending only; Retire accepts Active
only. Revoke accepts Pending/Active/Retired and is terminal. Rotate requires an
already registered Pending replacement and Active predecessor with identical
principal, purpose, profile and canonical scoped grants; both transitions publish
at one revision. All work is prepared in a bounded copy before publication.

Limits are 1,024 retained public keys, 64 explicit nonduplicate grants/key and
4,096 successful commands. Overflow refuses without eviction; revision increment
is checked. Retired/revoked public keys remain mathematically resolvable.
`TrustSnapshot` is immutable and exposes current disposition/principal/revocation
and scoped eligibility separately from its `verification_keys()` mathematical
snapshot. No snapshot or command result is an append capability. Existing
`ExpectedContext` and `VerificationReport` remain separate verifier-owned types.

## Exact private encoding

All integer tags are unsigned canonical CBOR; identifiers use the existing bounded
ASCII leaf types. Command bytes are:

```
["wdbx-trust-command-prototype-v1", namespace, command_id,
 authenticated_admin_principal, expected_revision, operation]
```

Operation layouts:

- Register: `[0, kid, public_bstr32, principal, 0, profile, grant_bstrs]`.
  Purpose tag 0 means P1 canonical writer. Other purposes/profiles are refused.
- Activate: `[1, kid]`; Retire: `[2, kid]`.
- Revoke: `[3, kid, reason, claimed_time_or_null]`; Administration=0,
  SuspectedCompromise=1. Present zero differs from absent null.
- Rotate: `[4, predecessor_kid, replacement_kid]`.

Each grant byte string is the existing canonical `ScopeBinding` encoding.
Grant arrays sort those exact bytes lexicographically and reject duplicates.
No wildcard grants, time-based transitions, delegation or foreign trust import
exist. The canonical command digest is SHA-256 of these complete bytes.

Snapshot encoding is:

```
["wdbx-p1-trust-snapshot-prototype-v1", namespace, revision,
 [[registration_without_operation_tag, status, since_revision, revocation], ...],
 [successful_command_digest_bstr32, ...]]
```

Keys sort by derived signer ID; history retains successful command order.
Status tags are Pending=0, Active=1, Retired=2, Revoked=3. Revocation is null or
`[reason, claimed_time_or_null]`. Snapshot SHA-256 binds all key metadata and
successful command digests and uses a domain distinct from mathematical key
snapshots. Claimed compromise time remains evidence and never changes ordering.

## Synthetic vectors and tests

The [independent standard-library builder](evidence/2026-10-22-continuation/trust/build_vectors.py)
reproduces [command and snapshot vectors](evidence/2026-10-22-continuation/trust/commands.json)
using published RFC 8032 public example keys. It implements deterministic CBOR
and SHA-256 only; it performs no signature cryptography. Rust asserts exact
command bytes/digests and each snapshot digest through register, activate,
replacement registration, rotation and revocation. Null versus zero is pinned.
Malformed fixtures exercise typed constructor refusals explicitly; this slice
has no command-wire decoder and claims no malformed-wire parser coverage.

Tests cover the full single-key transition matrix, old/new rotation-state matrix,
metadata mismatch atomicity, retries after revocation, changed arguments/admin/
revision conflicts, denied retries, corrected failed-ID reuse, duplicate/weak/
mismatched keys, purpose/profile/scope/namespace constraints, grant cap, key/history
capacity boundaries and revision overflow. Capacity tests inject bounded synthetic
state to isolate counters; they do not claim a durable history workload. Immutable
snapshot and crypto-sibling visibility tests exercise the private interface.

Run from repository root with the pinned toolchain:

```sh
cargo test -p abi-wdbx --lib v3::complete_episode::crypto::trust --offline
cargo clippy -p abi-wdbx --all-targets --features p1-crypto-witness --offline
python3 docs/completion/execution/evidence/2026-10-22-continuation/trust/build_vectors.py
```

These checks are not the full workspace gate, independent security review,
production administration or task-02 completion. Owner ratification, admission
simulation, durable ordering/recovery, custody and independent qualification remain
separate work. All completion requirements remain open/C0.
