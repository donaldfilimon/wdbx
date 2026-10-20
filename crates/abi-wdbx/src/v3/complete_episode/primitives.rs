//! Validated bounded leaves. Debug never prints supplied values.
use super::{CompleteEpisodeError as E, Result};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OpaqueRef(pub(super) String);
redacted!(OpaqueRef);
impl OpaqueRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
        {
            return Err(E::InvalidIdentifier);
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TenantRef(pub(super) OpaqueRef);
redacted!(TenantRef);
impl TenantRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OrganizationRef(pub(super) OpaqueRef);
redacted!(OrganizationRef);
impl OrganizationRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PrincipalRef(pub(super) OpaqueRef);
redacted!(PrincipalRef);
impl PrincipalRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GuildRef(pub(super) OpaqueRef);
redacted!(GuildRef);
impl GuildRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NodeRef(pub(super) OpaqueRef);
redacted!(NodeRef);
impl NodeRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RegimeRef(pub(super) OpaqueRef);
redacted!(RegimeRef);
impl RegimeRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClockRef(pub(super) OpaqueRef);
redacted!(ClockRef);
impl ClockRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ComponentRef(pub(super) OpaqueRef);
redacted!(ComponentRef);
impl ComponentRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ActionRef(pub(super) OpaqueRef);
redacted!(ActionRef);
impl ActionRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ParameterClassRef(pub(super) OpaqueRef);
redacted!(ParameterClassRef);
impl ParameterClassRef {
    pub(crate) fn new(value: &str) -> Result<Self> {
        OpaqueRef::new(value).map(Self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EpisodeId(pub(super) [u8; 16]);
redacted!(EpisodeId);
impl EpisodeId {
    pub(crate) fn new(value: [u8; 16]) -> Result<Self> {
        if value == [0; 16] || value[8] & 0xc0 != 0x80 || !matches!(value[6] >> 4, 4 | 7) {
            return Err(E::InvalidUuid);
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestId(pub(super) [u8; 16]);
redacted!(RequestId);
impl RequestId {
    pub(crate) fn new(value: [u8; 16]) -> Result<Self> {
        if value == [0; 16] || value[8] & 0xc0 != 0x80 || !matches!(value[6] >> 4, 4 | 7) {
            return Err(E::InvalidUuid);
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaskId(pub(super) [u8; 16]);
redacted!(TaskId);
impl TaskId {
    pub(crate) fn new(value: [u8; 16]) -> Result<Self> {
        if value == [0; 16] || value[8] & 0xc0 != 0x80 || !matches!(value[6] >> 4, 4 | 7) {
            return Err(E::InvalidUuid);
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Digest32(pub(super) [u8; 32]);
redacted!(Digest32);
impl Digest32 {
    pub(crate) const fn new(value: [u8; 32]) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct UtcMillis(pub(super) u64);
redacted!(UtcMillis);
impl UtcMillis {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct MonoNanos(pub(super) u64);
redacted!(MonoNanos);
impl MonoNanos {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SignedMicros(pub(super) i64);
redacted!(SignedMicros);
impl SignedMicros {
    pub(crate) const fn new(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitPpm(pub(super) u32);
redacted!(UnitPpm);
impl UnitPpm {
    pub(crate) fn new(value: u32) -> Result<Self> {
        if value > 1_000_000 {
            return Err(E::InvalidRange);
        }
        Ok(Self(value))
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Estimate<T> {
    Unknown,
    Known(T),
}
impl<T> std::fmt::Debug for Estimate<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unknown => "Estimate::Unknown",
            Self::Known(_) => "Estimate::Known { .. }",
        })
    }
}
