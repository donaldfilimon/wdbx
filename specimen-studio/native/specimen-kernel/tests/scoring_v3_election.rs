use specimen_kernel::scoring_v3::*;
#[test]
fn s09_alternatives_use_closed_grid_endpoints_and_inhibition() {
    for (draw, winner) in [(0, Some(1)), (u32::MAX, Some(3)), (u32::MAX / 2, Some(3))] {
        let values = [draw];
        let mut tape = DrawTape::new(12, "pool", &values, 1).unwrap();
        let result = elect(
            &[100.0, 1.0, 0.0, 3.0],
            &[false, true, true, true],
            &mut tape,
        )
        .unwrap();
        assert_eq!(result.winner, winner);
        assert_eq!(result.refused, vec![0]);
        assert_eq!(result.draw.unwrap().value, draw);
    }
}
#[test]
fn zero_and_single_positive_abstain_or_select_without_draw() {
    let mut t = DrawTape::new(0, "pool", &[], 0).unwrap();
    assert_eq!(
        elect(&[0.0, 0.0], &[true, true], &mut t).unwrap().winner,
        None
    );
    assert_eq!(
        elect(&[0.0, 4.0], &[true, true], &mut t).unwrap().winner,
        Some(1)
    );
    assert_eq!(t.consumed(), 0);
}
#[test]
fn finite_extreme_weights_do_not_overflow_and_late_invalid_input_is_atomic() {
    let mut t = DrawTape::new(0, "huge", &[u32::MAX], 1).unwrap();
    assert_eq!(
        elect(&[f64::MAX, f64::MAX], &[true, true], &mut t)
            .unwrap()
            .winner,
        Some(1)
    );
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        let mut t = DrawTape::new(0, "bad", &[0], 1).unwrap();
        assert_eq!(
            elect(&[1.0, bad], &[true, false], &mut t),
            Err(ScoringError::InvalidWeight)
        );
        assert_eq!(t.consumed(), 0);
    }
}
#[test]
fn endpoint_zero_selects_first_positive_even_when_scaling_underflows() {
    let mut t = DrawTape::new(0, "tiny", &[0], 1).unwrap();
    assert_eq!(
        elect(&[f64::from_bits(1), f64::MAX], &[true, true], &mut t)
            .unwrap()
            .winner,
        Some(0)
    );
}
#[test]
fn cumulative_intervals_are_half_open_at_an_exact_grid_boundary() {
    let mut t = DrawTape::new(0, "boundary", &[1_431_655_765], 1).unwrap();
    assert_eq!(
        elect(&[1.0, 2.0], &[true, true], &mut t).unwrap().winner,
        Some(1)
    );
}
