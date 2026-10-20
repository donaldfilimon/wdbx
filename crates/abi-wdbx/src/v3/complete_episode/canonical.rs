//! One typed mapping, interpreted first as checked sizes, then canonical values.
use super::*;
use crate::v3::commitment::CanonicalValue as C;
use crate::v3::episode::{ActorKind, EvidenceLevel, SignerKeyId};
mod mapping;

pub(super) trait Wire {
    fn wire<B: Builder>(&self) -> Result<B::Value>;
}
pub(super) trait Builder {
    type Value;
    fn unsigned(value: u64) -> Result<Self::Value>;
    fn signed(value: i64) -> Result<Self::Value>;
    fn text(value: &str) -> Result<Self::Value>;
    fn bytes(value: &[u8]) -> Result<Self::Value>;
    fn array(values: Vec<Self::Value>) -> Result<Self::Value>;
    fn map(values: Vec<(&str, Self::Value)>) -> Result<Self::Value>;
    fn envelope(values: Vec<Self::Value>) -> Result<Self::Value>;
    fn set(values: Vec<Self::Value>) -> Result<Self::Value>;
}
fn head(value: u64) -> usize {
    match value {
        0..=23 => 1,
        24..=255 => 2,
        256..=65535 => 3,
        65536..=4_294_967_295 => 5,
        _ => 9,
    }
}
pub(super) fn sum(head: usize, parts: impl IntoIterator<Item = usize>) -> Result<usize> {
    parts.into_iter().try_fold(head, |a, b| {
        a.checked_add(b)
            .ok_or(CompleteEpisodeError::EncodedSizeLimit)
    })
}
pub(super) struct Count;
impl Builder for Count {
    type Value = usize;
    fn unsigned(value: u64) -> Result<usize> {
        Ok(head(value))
    }
    fn signed(value: i64) -> Result<usize> {
        Ok(head(if value < 0 {
            value.unsigned_abs() - 1
        } else {
            value.unsigned_abs()
        }))
    }
    fn text(value: &str) -> Result<usize> {
        sum(head(value.len() as u64), [value.len()])
    }
    fn bytes(value: &[u8]) -> Result<usize> {
        sum(head(value.len() as u64), [value.len()])
    }
    fn array(values: Vec<usize>) -> Result<usize> {
        sum(head(values.len() as u64), values)
    }
    fn map(values: Vec<(&str, usize)>) -> Result<usize> {
        let mut total = head(values.len() as u64);
        for (key, value) in values {
            total = sum(total, [Self::text(key)?, value])?;
        }
        Ok(total)
    }
    fn envelope(values: Vec<usize>) -> Result<usize> {
        let mut total = head(values.len() as u64);
        for (i, value) in values.into_iter().enumerate() {
            total = sum(total, [head(i as u64), value])?;
        }
        Ok(total)
    }
    fn set(values: Vec<usize>) -> Result<usize> {
        Self::array(values)
    }
}
pub(super) fn counted_size(value: &impl Wire, limit: usize) -> Result<usize> {
    let size = value.wire::<Count>()?;
    if size > limit {
        return Err(CompleteEpisodeError::EncodedSizeLimit);
    }
    Ok(size)
}
pub(super) struct Tree;
impl Builder for Tree {
    type Value = C;
    fn unsigned(value: u64) -> Result<C> {
        Ok(C::Unsigned(value))
    }
    fn signed(value: i64) -> Result<C> {
        Ok(if value < 0 {
            C::Negative(value)
        } else {
            C::Unsigned(value.unsigned_abs())
        })
    }
    fn text(value: &str) -> Result<C> {
        Ok(C::Text(value.to_owned()))
    }
    fn bytes(value: &[u8]) -> Result<C> {
        Ok(C::Bytes(value.to_owned()))
    }
    fn array(values: Vec<C>) -> Result<C> {
        Ok(C::Array(values))
    }
    fn map(values: Vec<(&str, C)>) -> Result<C> {
        Ok(C::Map(
            values
                .into_iter()
                .map(|(k, v)| (C::Text(k.to_owned()), v))
                .collect(),
        ))
    }
    fn envelope(values: Vec<C>) -> Result<C> {
        Ok(C::Map(
            values
                .into_iter()
                .enumerate()
                .map(|(i, v)| (C::Unsigned(i as u64), v))
                .collect(),
        ))
    }
    fn set(values: Vec<C>) -> Result<C> {
        let mut keyed = values
            .into_iter()
            .map(|v| {
                v.canonical_bytes()
                    .map(|bytes| (bytes, v))
                    .map_err(|_| CompleteEpisodeError::Encoding)
            })
            .collect::<Result<Vec<_>>>()?;
        keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Self::array(keyed.into_iter().map(|(_, v)| v).collect())
    }
}
impl Wire for u64 {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::unsigned(*self)
    }
}
impl Wire for OpaqueRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::text(&self.0)
    }
}
impl Wire for SignerKeyId {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::text(self.as_str())
    }
}
impl<T: Wire> Wire for Estimate<T> {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        match self {
            Self::Unknown => B::array(vec![B::unsigned(0)?]),
            Self::Known(v) => B::array(vec![B::unsigned(1)?, v.wire::<B>()?]),
        }
    }
}
impl<T: Wire> Wire for Vec<T> {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::set(
            self.iter()
                .map(Wire::wire::<B>)
                .collect::<Result<Vec<_>>>()?,
        )
    }
}
fn sequence<B: Builder, T: Wire>(values: &[T]) -> Result<B::Value> {
    B::array(
        values
            .iter()
            .map(Wire::wire::<B>)
            .collect::<Result<Vec<_>>>()?,
    )
}
fn versions<B: Builder>(values: &[VersionRef]) -> Result<B::Value> {
    let mut values: Vec<_> = values.iter().collect();
    values.sort_unstable_by(|a, b| a.component.cmp(&b.component));
    B::array(
        values
            .into_iter()
            .map(Wire::wire::<B>)
            .collect::<Result<Vec<_>>>()?,
    )
}
fn parameters<B: Builder>(values: &[ParameterClassRef]) -> Result<B::Value> {
    let mut values: Vec<_> = values.iter().collect();
    values.sort_unstable();
    B::array(
        values
            .into_iter()
            .map(Wire::wire::<B>)
            .collect::<Result<Vec<_>>>()?,
    )
}
fn variant<B: Builder>(tag: u64, fields: Vec<(&str, B::Value)>) -> Result<B::Value> {
    B::array(vec![B::unsigned(tag)?, B::map(fields)?])
}
impl Wire for Scope {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        match self {
            Self::Guild(guild) => variant::<B>(0, vec![("guild", guild.wire::<B>()?)]),
            Self::GuildMember { guild, member } => variant::<B>(
                1,
                vec![
                    ("guild", guild.wire::<B>()?),
                    ("member", member.wire::<B>()?),
                ],
            ),
            Self::DirectMessage { user } => variant::<B>(2, vec![("user", user.wire::<B>()?)]),
            Self::Organization(org) => variant::<B>(3, vec![("organization", org.wire::<B>()?)]),
        }
    }
}
impl Wire for ConsultedRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        match self {
            Self::Local(v) => variant::<B>(0, vec![("reference", v.wire::<B>()?)]),
            Self::Foreign(v) => variant::<B>(1, vec![("reference", v.wire::<B>()?)]),
        }
    }
}
impl Wire for RetentionIntent {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        match self {
            Self::Ephemeral => B::array(vec![B::unsigned(0)?]),
            Self::Session {
                session,
                expires_at,
            } => variant::<B>(
                1,
                vec![
                    ("session", session.wire::<B>()?),
                    ("expires_at", expires_at.wire::<B>()?),
                ],
            ),
            Self::Operational { expires_at } => {
                variant::<B>(2, vec![("expires_at", expires_at.wire::<B>()?)])
            }
            Self::Durable { privacy_basis } => {
                variant::<B>(3, vec![("privacy_basis", privacy_basis.wire::<B>()?)])
            }
            Self::MandatoryIncident { obligation } => {
                variant::<B>(4, vec![("obligation", obligation.wire::<B>()?)])
            }
        }
    }
}
impl Wire for RegimeDistribution {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let mut values: Vec<_> = self.0.iter().collect();
        values.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        B::array(
            values
                .into_iter()
                .map(|(regime, ppm)| B::array(vec![regime.wire::<B>()?, ppm.wire::<B>()?]))
                .collect::<Result<Vec<_>>>()?,
        )
    }
}
impl Wire for ActorKind {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::text(match self {
            Self::HumanSubject => "human_subject",
            Self::OrganizationOwner => "organization_owner",
            Self::GuildOwner => "guild_owner",
            Self::GuildAdministrator => "guild_administrator",
            Self::GuildManager => "guild_manager",
            Self::Service => "service",
        })
    }
}
impl Wire for EvidenceLevel {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::text(match self {
            Self::C0 => "C0",
            Self::C1 => "C1",
            Self::C2 => "C2",
            Self::C3 => "C3",
            Self::C4 => "C4",
            Self::C5 => "C5",
            Self::C6 => "C6",
            Self::C7 => "C7",
        })
    }
}
