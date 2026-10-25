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
        Ok(Self { v, a, cooldown_seconds })
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
        Ok(Self { valence, intensity, urgency })
    }

    pub(super) fn valence(self) -> f32 { self.valence }
    pub(super) fn intensity(self) -> f32 { self.intensity }
    pub(super) fn urgency(self) -> bool { self.urgency }
}
