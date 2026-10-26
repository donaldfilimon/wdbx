#[path = "../src/atp_v3/mod.rs"]
mod atp_v3;

use atp_v3::{ElapsedError, State, StateError, Stimulus, StimulusError, Transition, advance};

// Catch omission of finite/range validation in any individual state field.
#[test]
fn d10_state_admission_rejects_each_nonfinite_and_out_of_range_field() {
    for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(State::new(x, 0.0, 0.0), Err(StateError::ValenceNotFinite));
        assert_eq!(State::new(0.0, x, 0.0), Err(StateError::ArousalNotFinite));
        assert_eq!(State::new(0.0, 0.0, x), Err(StateError::CooldownNotFinite));
    }
    for x in [-1.000_000_1, 1.000_000_1] {
        assert_eq!(State::new(x, 0.0, 0.0), Err(StateError::ValenceOutOfRange));
    }
    for x in [-f32::MIN_POSITIVE, 1.000_000_1] {
        assert_eq!(State::new(0.0, x, 0.0), Err(StateError::ArousalOutOfRange));
    }
    for x in [-f32::MIN_POSITIVE, f32::from_bits(30.0_f32.to_bits() + 1)] {
        assert_eq!(State::new(0.0, 0.0, x), Err(StateError::CooldownOutOfRange));
    }
    for v in [-1.0, 1.0] {
        for a in [0.0, 1.0] {
            for c in [0.0, 30.0] {
                assert_eq!(
                    state_bits(State::new(v, a, c).unwrap()),
                    [v.to_bits(), a.to_bits(), c.to_bits()]
                );
            }
        }
    }
}

// Catch omission of finite/range checks, or accidentally treating urgency as permission.
#[test]
fn d10_stimulus_admission_rejects_each_nonfinite_and_out_of_range_field() {
    for urgency in [false, true] {
        for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                Stimulus::new(x, 0.0, urgency),
                Err(StimulusError::ValenceNotFinite)
            );
            assert_eq!(
                Stimulus::new(0.0, x, urgency),
                Err(StimulusError::IntensityNotFinite)
            );
        }
        for x in [-1.000_000_1, 1.000_000_1] {
            assert_eq!(
                Stimulus::new(x, 0.0, urgency),
                Err(StimulusError::ValenceOutOfRange)
            );
        }
        for x in [-f32::MIN_POSITIVE, 1.000_000_1] {
            assert_eq!(
                Stimulus::new(0.0, x, urgency),
                Err(StimulusError::IntensityOutOfRange)
            );
        }
        for v in [-1.0, 1.0] {
            for i in [0.0, 1.0] {
                let stimulus = Stimulus::new(v, i, urgency).unwrap();
                assert_eq!(
                    (stimulus.valence(), stimulus.intensity(), stimulus.urgency()),
                    (v, i, urgency)
                );
            }
        }
    }
}

// Catch allowing negative-zero representations into immutable admitted inputs.
#[test]
fn d10_admission_normalizes_signed_zero_without_mutating_callers() {
    let fields = [-0.0_f32; 3];
    let state = State::new(fields[0], fields[1], fields[2]).unwrap();
    assert_eq!(state_bits(state), [0; 3]);
    let stimulus = Stimulus::new(fields[0], fields[1], true).unwrap();
    assert_eq!(
        [stimulus.valence().to_bits(), stimulus.intensity().to_bits()],
        [0; 2]
    );
    assert!(stimulus.urgency());
    assert_eq!(fields.map(f32::to_bits), [0x8000_0000; 3]);
}

fn state_bits(state: State) -> [u32; 3] {
    [
        state.v().to_bits(),
        state.a().to_bits(),
        state.cooldown_seconds().to_bits(),
    ]
}

// Catch accepting a backwards/nonfinite host duration or mutating a refused input.
#[test]
fn d10_elapsed_refusal_is_typed_and_inputs_remain_immutable() {
    let state = State::new(0.25, 0.5, 10.0).unwrap();
    let stimulus = Stimulus::new(-0.5, 0.75, true).unwrap();
    let original = (state, stimulus);
    for dt in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(advance(state, stimulus, dt), Err(ElapsedError::NotFinite));
    }
    for dt in [-f32::MIN_POSITIVE, -1.0, -f32::MAX] {
        assert_eq!(advance(state, stimulus, dt), Err(ElapsedError::Negative));
    }
    assert_eq!((state, stimulus), original);
}

// Catch consulting time or missing charge at a zero-duration transition.
#[test]
fn d10_zero_elapsed_charges_and_success_leaves_inputs_immutable() {
    let state = State::new(0.0, 0.0, 0.0).unwrap();
    let stimulus = Stimulus::new(1.0, 1.0, false).unwrap();
    let original = (state, stimulus);
    let step = advance(state, stimulus, 0.0).unwrap();
    assert_step(step, [0.2, 0.2, 0.0], 0.0, false, false);
    assert_eq!((state, stimulus), original);
}

// Catch unweighted valence charging and incorrectly coupling urgency to intensity.
#[test]
fn d10_zero_intensity_blocks_valence_charge_but_declared_urgency_adds_point_one() {
    let state = State::new(0.25, 0.25, 0.0).unwrap();
    for valence in [-1.0, 1.0] {
        assert_step(
            advance(state, Stimulus::new(valence, 0.0, false).unwrap(), 0.0).unwrap(),
            [0.25, 0.25, 0.0],
            0.0,
            false,
            false,
        );
        assert_step(
            advance(state, Stimulus::new(valence, 0.0, true).unwrap(), 0.0).unwrap(),
            [0.25, 0.35, 0.0],
            0.0,
            false,
            false,
        );
    }
    assert_step(
        advance(
            State::new(0.0, 0.0, 0.0).unwrap(),
            Stimulus::new(-0.5, 0.5, true).unwrap(),
            0.0,
        )
        .unwrap(),
        [-0.05, 0.2, 0.0],
        0.0,
        false,
        false,
    );
}

// e^-1, e^-1/2 and e^-1/4 have independently rounded IEEE-754 binary32 values.
// Catch missing/incorrect decay or charging while a positive cooldown is present.
#[test]
fn d10_idle_decay_matches_independently_rounded_exp_minus_one() {
    let step = advance(State::new(-0.5, 0.25, 0.0).unwrap(), idle(), 180.0).unwrap();
    assert_eq!(state_bits(step.state()), [0xbe3c_5ab2, 0x3dbc_5ab2, 0]);
    assert_eq!(
        (
            step.applied_elapsed(),
            step.released(),
            step.cooldown_at_entry()
        ),
        (180.0, false, false)
    );
}

// Catch omitting the cap, capping before duration refusal, or reporting raw elapsed.
#[test]
fn d10_elapsed_cap_is_exactly_3600_for_all_larger_finite_values() {
    let state = State::new(-1.0, 1.0, 30.0).unwrap();
    let stimulus = Stimulus::new(1.0, 1.0, true).unwrap();
    let cap = advance(state, stimulus, 3600.0).unwrap();
    assert_eq!(state_bits(cap.state()), [0xb10d_a433, 0x310d_a433, 0]);
    assert_eq!(
        (
            cap.applied_elapsed(),
            cap.released(),
            cap.cooldown_at_entry()
        ),
        (3600.0, false, true)
    );
    for dt in [f32::from_bits(3600.0_f32.to_bits() + 1), 7200.0, f32::MAX] {
        assert_eq!(advance(state, stimulus, dt).unwrap(), cap);
    }
}

// Catch strict > instead of >= at release, or a rounded/quantized threshold.
#[test]
fn d10_release_is_inclusive_at_point_eight_five_and_preserves_adjacent_float_boundary() {
    let threshold = 0.85_f32;
    let below = f32::from_bits(threshold.to_bits() - 1);
    let above = f32::from_bits(threshold.to_bits() + 1);
    assert_step(
        advance(State::new(0.5, below, 0.0).unwrap(), idle(), 0.0).unwrap(),
        [0.5, below, 0.0],
        0.0,
        false,
        false,
    );
    assert_step(
        advance(State::new(0.5, threshold, 0.0).unwrap(), idle(), 0.0).unwrap(),
        [0.4, 0.425, 30.0],
        0.0,
        true,
        false,
    );
    // Multiplication by 1/2 changes only the exponent in this normal f32 range.
    assert_step(
        advance(State::new(0.5, above, 0.0).unwrap(), idle(), 0.0).unwrap(),
        [0.4, f32::from_bits(0.425_f32.to_bits() + 1), 30.0],
        0.0,
        true,
        false,
    );
}

// Catch failing to clamp either sign of valence or arousal before release scaling.
#[test]
fn d10_clamps_before_releasing_both_valence_signs_and_arousal() {
    for sign in [-1.0, 1.0] {
        assert_step(
            advance(
                State::new(sign, 1.0, 0.0).unwrap(),
                Stimulus::new(sign, 1.0, true).unwrap(),
                0.0,
            )
            .unwrap(),
            [0.8 * sign, 0.5, 30.0],
            0.0,
            true,
            false,
        );
    }
}

// Catch charging/releasing on the cooldown branch or consuming it at dt=0.
#[test]
fn d10_entry_cooldown_ignores_stimulus_and_zero_elapsed_preserves_it() {
    let state = State::new(0.5, 1.0, 30.0).unwrap();
    let intense = Stimulus::new(-1.0, 1.0, true).unwrap();
    let step = advance(state, intense, 10.0).unwrap();
    assert_eq!(step, advance(state, idle(), 10.0).unwrap());
    assert_eq!(step.state().cooldown_seconds(), 20.0);
    assert_eq!(
        (
            step.applied_elapsed(),
            step.released(),
            step.cooldown_at_entry()
        ),
        (10.0, false, true)
    );
    assert!(step.state().v() < state.v());
    assert!(step.state().a() < state.a());
    assert_step(
        advance(state, intense, 0.0).unwrap(),
        [0.5, 1.0, 30.0],
        0.0,
        false,
        true,
    );
}

// Catch charging during a step whose entry cooldown crosses zero.
#[test]
fn s24_cooldown_crossing_zero_waits_until_the_next_separate_step_to_charge() {
    let state = State::new(0.0, 0.0, 1.0).unwrap();
    let intense = Stimulus::new(1.0, 1.0, true).unwrap();
    let crossed = advance(state, intense, 2.0).unwrap();
    assert_step(crossed, [0.0, 0.0, 0.0], 2.0, false, true);
    assert_step(
        advance(crossed.state(), intense, 0.0).unwrap(),
        [0.2, 0.3, 0.0],
        0.0,
        false,
        false,
    );
}

// Catch negative zero produced by decay underflow or preserved in admitted elapsed.
#[test]
fn d10_output_and_negative_zero_elapsed_have_only_positive_zero_bits() {
    let state = State::new(-f32::from_bits(1), 0.0, 30.0).unwrap();
    assert_eq!(
        state_bits(advance(state, idle(), 3600.0).unwrap().state()),
        [0; 3]
    );
    let step = advance(
        State::new(-0.0, -0.0, -0.0).unwrap(),
        Stimulus::new(-0.0, -0.0, false).unwrap(),
        -0.0,
    )
    .unwrap();
    assert_step(step, [0.0; 3], 0.0, false, false);
}

// Catch integrating jitter, hidden mutable state, or losing buildup/release over steps.
#[test]
fn s24_repeated_scalar_calls_have_no_jitter_or_hidden_state_and_cycles_build_release_decay() {
    let initial = State::new(0.0, 0.0, 0.0).unwrap();
    let stimulus = Stimulus::new(1.0, 1.0, false).unwrap();
    let first = advance(initial, stimulus, 0.0).unwrap();
    assert_step(first, [0.2, 0.2, 0.0], 0.0, false, false);
    let second = advance(first.state(), stimulus, 0.0).unwrap();
    assert_step(second, [0.4, 0.4, 0.0], 0.0, false, false);
    let mut state = second.state();
    for _ in 0..3 {
        state = advance(state, stimulus, 0.0).unwrap().state();
    }
    assert_eq!(
        state_bits(state),
        [0.8_f32.to_bits(), 0.5_f32.to_bits(), 30.0_f32.to_bits()]
    );
    for _ in 0..64 {
        assert_eq!(advance(initial, stimulus, 0.0).unwrap(), first);
        state = advance(state, idle(), 3600.0).unwrap().state();
        assert!(state.v().is_finite() && state.a().is_finite());
        assert!((-1.0..=1.0).contains(&state.v()) && (0.0..=1.0).contains(&state.a()));
        assert!((0.0..=30.0).contains(&state.cooldown_seconds()));
    }
    assert_eq!(state_bits(state), [0; 3]);
}

fn idle() -> Stimulus {
    Stimulus::new(0.0, 0.0, false).unwrap()
}

fn assert_step(step: Transition, expected: [f32; 3], elapsed: f32, released: bool, cooldown: bool) {
    assert_eq!(state_bits(step.state()), expected.map(f32::to_bits));
    assert_eq!(step.applied_elapsed().to_bits(), elapsed.to_bits());
    assert_eq!(
        (step.released(), step.cooldown_at_entry()),
        (released, cooldown)
    );
}
