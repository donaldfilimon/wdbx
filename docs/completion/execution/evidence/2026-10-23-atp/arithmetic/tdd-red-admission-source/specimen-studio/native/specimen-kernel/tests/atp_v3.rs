#[path = "../src/atp_v3/mod.rs"]
mod atp_v3;

use atp_v3::{State, StateError, Stimulus, StimulusError};

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
                assert_eq!(state_bits(State::new(v, a, c).unwrap()), [v.to_bits(), a.to_bits(), c.to_bits()]);
            }
        }
    }
}

// Catch omission of finite/range checks, or accidentally treating urgency as permission.
#[test]
fn d10_stimulus_admission_rejects_each_nonfinite_and_out_of_range_field() {
    for urgency in [false, true] {
        for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(Stimulus::new(x, 0.0, urgency), Err(StimulusError::ValenceNotFinite));
            assert_eq!(Stimulus::new(0.0, x, urgency), Err(StimulusError::IntensityNotFinite));
        }
        for x in [-1.000_000_1, 1.000_000_1] {
            assert_eq!(Stimulus::new(x, 0.0, urgency), Err(StimulusError::ValenceOutOfRange));
        }
        for x in [-f32::MIN_POSITIVE, 1.000_000_1] {
            assert_eq!(Stimulus::new(0.0, x, urgency), Err(StimulusError::IntensityOutOfRange));
        }
        for v in [-1.0, 1.0] {
            for i in [0.0, 1.0] {
                let stimulus = Stimulus::new(v, i, urgency).unwrap();
                assert_eq!((stimulus.valence(), stimulus.intensity(), stimulus.urgency()), (v, i, urgency));
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
    assert_eq!([stimulus.valence().to_bits(), stimulus.intensity().to_bits()], [0; 2]);
    assert!(stimulus.urgency());
    assert_eq!(fields.map(f32::to_bits), [0x8000_0000; 3]);
}

fn state_bits(state: State) -> [u32; 3] {
    [state.v().to_bits(), state.a().to_bits(), state.cooldown_seconds().to_bits()]
}
