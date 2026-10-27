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
    pub(super) fn state(self) -> State { self.state }
    pub(super) fn applied_elapsed(self) -> f32 { self.applied_elapsed }
    pub(super) fn released(self) -> bool { self.released }
    pub(super) fn cooldown_at_entry(self) -> bool { self.cooldown_at_entry }
}

pub(super) fn advance(state: State, _stimulus: Stimulus, elapsed: f32) -> Result<Transition, ElapsedError> {
    Ok(Transition { state, applied_elapsed: elapsed, released: false, cooldown_at_entry: false })
}
