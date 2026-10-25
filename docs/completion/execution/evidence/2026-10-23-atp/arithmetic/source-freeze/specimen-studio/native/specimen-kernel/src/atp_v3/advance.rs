#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ElapsedError {
    NotFinite,
    Negative,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Transition {
    state: State,
    applied_elapsed: f32,
    released: bool,
    cooldown_at_entry: bool,
}

impl Transition {
    pub(super) fn state(self) -> State {
        self.state
    }
    pub(super) fn applied_elapsed(self) -> f32 {
        self.applied_elapsed
    }
    pub(super) fn released(self) -> bool {
        self.released
    }
    pub(super) fn cooldown_at_entry(self) -> bool {
        self.cooldown_at_entry
    }
}

/// One logicalf32-v1 scalar step. Urgency is a supplied declaration only.
///
/// Every named operation rounds to f32; no fused multiply-add or wider intermediate
/// is used. A positive cooldown at entry excludes charging for the entire step.
pub(super) fn advance(
    state: State,
    stimulus: Stimulus,
    elapsed: f32,
) -> Result<Transition, ElapsedError> {
    if !elapsed.is_finite() {
        return Err(ElapsedError::NotFinite);
    }
    if elapsed < 0.0_f32 {
        return Err(ElapsedError::Negative);
    }
    let dt = positive_zero(elapsed.min(3600.0_f32));
    let d = libm::expf((-dt) / 180.0_f32);
    let v0 = d * state.v();
    let a0 = d * state.a();
    let cooldown_at_entry = state.cooldown_seconds() > 0.0_f32;
    let (v, a, cooldown, released) = if cooldown_at_entry {
        let cooldown = (state.cooldown_seconds() - dt).max(0.0_f32);
        (v0, a0, cooldown, false)
    } else {
        let vcharge = (0.2_f32 * stimulus.valence()) * stimulus.intensity();
        let v1 = (v0 + vcharge).clamp(-1.0_f32, 1.0_f32);
        let abase = a0 + (0.2_f32 * stimulus.intensity());
        let urgency = if stimulus.urgency() { 1.0_f32 } else { 0.0_f32 };
        let a1 = (abase + (0.1_f32 * urgency)).clamp(0.0_f32, 1.0_f32);
        if a1 >= 0.85_f32 {
            (0.8_f32 * v1, 0.5_f32 * a1, 30.0_f32, true)
        } else {
            (v1, a1, 0.0_f32, false)
        }
    };
    // Finite admitted operands, d in [0,1], and the clamps prove these bounds.
    // Use the same validated constructor, including positive-zero normalization.
    let state = State::new(v, a, cooldown).expect("bounded logicalf32-v1 transition");
    Ok(Transition {
        state,
        applied_elapsed: dt,
        released,
        cooldown_at_entry,
    })
}
