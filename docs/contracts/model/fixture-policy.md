# Frozen diagnostic fixture policy

Fixtures are under `crates/abi-wdbx/tests/fixtures/model-v1/`. `manifest.json`
inventories exact bytes, hashes, schema versions, expected validation stage and
error. Positive identities have independent ordinary and domain-separated digest
pins. `field-table.json` gives human-checkable component-relative offsets; literal
RNG states/outputs live in `rng-vectors.json`.

The initial fixture worksheet uses explicit little-endian field construction,
literal tensors/moments/topology and independent identity formulas. It does not
import the Python validator or any Rust serializer. The worksheet is private
scratch, not a regeneration tool. Rust tests independently hash the frozen bytes
and verify identities/catalogs. The Python witness decodes all eight schemas,
checks framing before semantics, and validates topology, axes, BPE, optimizer,
RNG/order and provenance cross-links. A framed semantic-invalid fixture cannot
produce a validated model result. Semantic negative fixtures have recomputed
component/directory hashes; integrity failures are separate vectors.

No test or gate rewrites expected values. Published version 1 fixtures are never
repurposed: deliberate changes need review and newly versioned identities. A
missing Python interpreter fails the contract target and root gate. The tests
also deliberately alter a digest expectation and require that comparison to fail.

Run:

```sh
python3 -m unittest discover -s tools/tests -p 'test_wdbx_model_reference.py'
cargo test --locked -p abi-wdbx --test model_contracts < /dev/null
```

These checks prove contract/witness agreement only. They do not prove a production
reader, bounded production allocation, training improvement, exact runtime
resume, policy eligibility, generation publication or device performance.
