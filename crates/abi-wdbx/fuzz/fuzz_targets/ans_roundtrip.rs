#![no_main]

//! Round-trip the rANS codec over arbitrary plaintext.
//!
//! This mirrors the only in-tree consumer, `abi-cli/src/wdbx/secure.rs`, which
//! calls `ans_encode`/`ans_encode_order1` on caller-supplied bytes and decodes
//! the result immediately. Nothing in the workspace rehydrates an `AnsEncoded`
//! from stored bytes, so a decode-only harness would be testing an invariant
//! production cannot violate; the round trip is the faithful surface.
//!
//! Every assertion below restates an invariant the crate's own unit tests
//! already assert, so a failure here is a real defect rather than a harness
//! artefact.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Encoding only fails above the frozen u32 length limit, which -max_len
    // keeps far out of reach; a failure here would itself be the finding.
    let order0 = abi_wdbx::ans_encode(data).expect("ans_encode rejected a short input");
    let decoded = abi_wdbx::ans_decode(&order0).expect("ans_decode rejected its own encoder output");
    assert_eq!(decoded, data, "order-0 round trip lost data");

    let order1 =
        abi_wdbx::ans_encode_order1(data).expect("ans_encode_order1 rejected a short input");
    let decoded1 =
        abi_wdbx::ans_decode(&order1).expect("ans_decode rejected its own order-1 output");
    assert_eq!(decoded1, data, "order-1 round trip lost data");
});
