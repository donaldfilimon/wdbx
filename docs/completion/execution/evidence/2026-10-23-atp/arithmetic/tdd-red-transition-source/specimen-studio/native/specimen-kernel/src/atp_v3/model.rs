#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct State {
    v: f32,
    a: f32,
    cooldown_seconds: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StateError {
    ValenceNotFinite,
    ValenceOutOfRange,
    ArousalNotFinite,
    ArousalOutOfRange,
    CooldownNotFinite,
    CooldownOutOfRange,
}

impl State {
    pub(super) fn new(v: f32, a: f32, cooldown_seconds: f32) -> Result<Self, StateError> {
        if !v.is_finite() { return Err(StateError::ValenceNotFinite); }
        if !(-1.0_f32..=1.0_f32).contains(&v) { return Err(StateError::ValenceOutOfRange); }
        if !a.is_finite() { return Err(StateError::ArousalNotFinite); }
        if !(0.0_f32..=1.0_f32).contains(&a) { return Err(StateError::ArousalOutOfRange); }
        if !cooldown_seconds.is_finite() { return Err(StateError::CooldownNotFinite); }
        if !(0.0_f32..=30.0_f32).contains(&cooldown_seconds) { return Err(StateError::CooldownOutOfRange); }
        Ok(Self { v: positive_zero(v), a: positive_zero(a), cooldown_seconds: positive_zero(cooldown_seconds) })
    }

    pub(super) fn v(self) -> f32 { self.v }
    pub(super) fn a(self) -> f32 { self.a }
    pub(super) fn cooldown_seconds(self) -> f32 { self.cooldown_seconds }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stimulus {
    valence: f32,
    intensity: f32,
    urgency: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StimulusError {
    ValenceNotFinite,
    ValenceOutOfRange,
    IntensityNotFinite,
    IntensityOutOfRange,
}

impl Stimulus {
    pub(super) fn new(valence: f32, intensity: f32, urgency: bool) -> Result<Self, StimulusError> {
        if !valence.is_finite() { return Err(StimulusError::ValenceNotFinite); }
        if !(-1.0_f32..=1.0_f32).contains(&valence) { return Err(StimulusError::ValenceOutOfRange); }
        if !intensity.is_finite() { return Err(StimulusError::IntensityNotFinite); }
        if !(0.0_f32..=1.0_f32).contains(&intensity) { return Err(StimulusError::IntensityOutOfRange); }
        Ok(Self { valence: positive_zero(valence), intensity: positive_zero(intensity), urgency })
    }

    pub(super) fn valence(self) -> f32 { self.valence }
    pub(super) fn intensity(self) -> f32 { self.intensity }
    pub(super) fn urgency(self) -> bool { self.urgency }
}

fn positive_zero(value: f32) -> f32 {
    if value == 0.0_f32 { 0.0_f32 } else { value }
}
