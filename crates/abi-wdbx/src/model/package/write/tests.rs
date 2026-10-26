//! Preflight owns no output and admits both memory phases before reservation.
use super::*;
fn sample() -> ModelComponents {
    let text = include_str!("../../../../tests/fixtures/model-v1/inference.hex");
    let raw = text
        .trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    let limits = LoadLimits::default();
    super::super::validate_model(
        super::super::validate_framing(
            super::super::read_bounded_snapshot(raw.as_slice(), limits).unwrap(),
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap()
    .components()
    .clone()
}
#[test]
fn measuring_pass_has_no_output_storage_and_peak_boundary_is_checked_first() {
    let c = sample();
    let limits = LoadLimits::default();
    let mut e = Encoder::measure(limits).unwrap();
    e.zeroes(736).unwrap();
    for kind in 1..=8 {
        schema::component(kind, &c, &mut e).unwrap();
        assert_eq!(e.out.capacity(), 0);
        assert_eq!(e.out.len(), 0);
    }
    let plan = preflight::calculate(&c, &[], limits).unwrap();
    assert_eq!(e.len(), plan.output_len);
    let exact = LoadLimits {
        loading_bytes: plan.peak,
        ..limits
    };
    assert!(preflight::calculate(&c, &[], exact).is_ok());
    let below = LoadLimits {
        loading_bytes: plan.peak - 1,
        ..limits
    };
    let error = preflight::calculate(&c, &[], below).expect_err("peak boundary");
    assert_eq!(error.field, "writer-peak-budget");
    assert!(build_package(c, &[], exact).is_ok());
}

#[test]
fn aggregate_peak_refusal_never_requests_an_output_buffer() {
    use std::cell::Cell;
    let mut c = sample();
    if let crate::model::contract::tensors::TensorValues::F32(values) = &mut c.tensors[0].data {
        values.reserve_exact(65536);
    }
    let limits = LoadLimits::default();
    let plan = preflight::calculate(&c, &[], limits).unwrap();
    // The below-peak budget can fit output alone and input alone, but not both.
    assert!(plan.output_len as u64 + crate::model::limits::LOADING_OVERHEAD_BYTES < plan.peak - 1);
    let below = LoadLimits {
        loading_bytes: plan.peak - 1,
        ..limits
    };
    let calls = Cell::new(0);
    let error = build_with_reservation(c, &[], below, |_| {
        calls.set(calls.get() + 1);
        Err(err(E::Io, "unexpected-output-reservation"))
    })
    .expect_err("aggregate peak was admitted");
    assert_eq!(error.field, "writer-peak-budget");
    assert_eq!(calls.get(), 0);
}
