# Native WDBX model task sequence and requirement index

This repository-owned index implements the deliverable required by
[binding design section 13](../contracts/model/native-package-design-v1.md#13-n01-implementation-packet-freeze-contracts-and-independent-witnesses).
It indexes the approved work; it does not replace or expand the company master.
The company-owned master is `docs/plans/2026-10-25-products-modernization-approved.md`,
ordered implementation items 2 and 4 (M2/M4). Its requirement register is
`docs/plans/modernization/coverage.json`. Those are paths relative to the company
repository, not WDBX build/runtime dependencies or machine-specific locations.

The [binding design](../contracts/model/native-package-design-v1.md), sections
1–12, governs exact identities, bytes, schemas, bounds and ownership. The
[N01 execution plan](2026-10-25-native-model-contracts.md) records the current
packet. The [contract index](../contracts/model/README.md) links its implemented
schema and fixture references.

## Task sequence

| Task | Responsibility and acceptance boundary | Current status |
| --- | --- | --- |
| N01 | Typed contracts, independent full binary/semantic witness and frozen diagnostic vectors. | Implemented in `a043e4f`; focused checks, root gate and completion checks passed. This missing index is the review fix; controller review acceptance remains pending. |
| N02 | Bounded owned snapshot, real package parser/writer, staging and tokenizer; framing, semantic and resume validation remain distinct. | Pending N01 review acceptance; no production implementation claimed. See [section 14](../contracts/model/native-package-design-v1.md#14-n02-implementation-packet-bounded-readerwriter-staging-real-tokenizer). |
| N03 | Actual recurrent spatial CPU forward execution for causal text and graph ranking, with independent numerical and causal-isolation evidence. | Pending validated N02 inputs; contract fixtures do not qualify execution. |
| N04 | Actual backward/loss/optimizer implementation, admitted data preparation, checkpointing, objective improvement and deterministic training/resume equivalence. | Pending N02/N03 execution and data contracts; no training outcome claimed. |
| N05 | Real ABI native-package admission, license/audience/artifact authority and normal Models/CLI/MCP selection. | Pending actual N03/N04 model support; no dummy provider or activation. |
| N06 | Durable generation publication/recovery, lineage allocation history, audience partitioning, revocation and current authority. | Pending validated state, checkpoint and policy integration; package identity is not publication or permission. |
| N07 | Authorized topology proposals and stable-ID/tensor/moment migration under capacity and lineage floors. | Pending execution/training and N06 publication/policy boundaries. |

Later acceptance remains exactly as specified in
[section 16](../contracts/model/native-package-design-v1.md#16-remaining-boundaries-and-reviewer-watchpoints).
This sequence does not authorize starting dependent work before its required
review or replace later tasks' implementation plans.

## Requirement mapping index

[Section 12](../contracts/model/native-package-design-v1.md#12-requirement-mapping-exact-coveragejson-identities)
is the exact mapping source, including all ten fully spelled company coverage
keys and inherited source/owner identities. M2/M4 and N01–N07 are task labels,
not replacement requirement IDs. The following index retains those obligations:

| Source owner and exact requirement identities | Native task responsibility / retained obligation |
| --- | --- |
| company `wdbx-model-package-v1` | N01 contracts/fixtures, N02 real loading/export/import; N04/N06 checkpoint publication remains open. |
| company `wdbx-spatial-neural-v1` | N01 catalog/axes prerequisite; N03/N04 must prove actual execution and training. |
| company `wdbx-online-learning-v1` | N01 explicit profile/state references; N06/N07 must prove authorized updates, isolation and recovery. |
| abi `C23` | Original Rust causal decoder training remains binding; spatial-model work does not discharge it. |
| abi `C24`, `C25`, `C26`, `C27` | Tokenizer/checkpoint/optimizer contracts support later deterministic resume, licensed provenance, split isolation and broad evaluation; N01 is not runtime/quality evidence. |
| wdbx `PLAN-0073`, `V6-1403`, `V6-1067` | Preserve existing storage/hash/WAL/locking/checkpoint and Specimen readers/save-load; native identities do not replace them. |
| wdbx `V6-1215` | Explicit unsupported-format/version failure; no silent reinterpretation. |
| wdbx `V6-0459`, `V6-0589`, `V6-0628` | N05/N06 verify artifact/license/audience/consent/source authority and evidence; package references are never permission. |
| wdbx `P08.04`, `H-P08.04`, `V6-0986` | Historical fixed/manual-learning profiles remain independent; online learning is an explicit bounded profile. |
| wdbx `V6-1080` | Separate native capability negotiation; control/ABI integration remains later M2/N05 work. |

No coverage row or C0 status is promoted by this index. Preserve legacy neural
codec, original tokenizer/training and substrate obligations. Accelerator/TPU,
desktop-platform, production model quality, live deployment, sustained operation
and LYLX replacement remain separately qualified outcomes. N01 neither changes
source authority nor proves any of those outcomes.
