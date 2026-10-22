//! Shape extraction, with caps checked before typed reservation or child conversion.
use super::*;
pub(super) fn next(values: &mut impl Iterator<Item = C>) -> DResult<C> {
    values.next().ok_or(E::InvalidShape)
}
pub(super) fn text(value: C) -> DResult<String> {
    match value {
        C::Text(v) => Ok(v),
        _ => Err(E::InvalidShape),
    }
}
pub(super) fn unsigned(value: C) -> DResult<u64> {
    match value {
        C::Unsigned(v) => Ok(v),
        _ => Err(E::InvalidShape),
    }
}
pub(super) fn signed(value: C) -> DResult<i64> {
    match value {
        C::Unsigned(v) => v.try_into().map_err(|_| E::InvalidShape),
        C::Negative(v) => Ok(v),
        _ => Err(E::InvalidShape),
    }
}
pub(super) fn fixed<const N: usize>(value: C) -> DResult<[u8; N]> {
    match value {
        C::Bytes(v) => v.try_into().map_err(|_| E::InvalidShape),
        _ => Err(E::InvalidShape),
    }
}
pub(super) fn array(value: C) -> DResult<Vec<C>> {
    match value {
        C::Array(v) => Ok(v),
        _ => Err(E::InvalidShape),
    }
}
pub(super) fn list<T>(
    value: C,
    cap: usize,
    mut mapper: impl FnMut(C) -> DResult<T>,
) -> DResult<Vec<T>> {
    let values = array(value)?;
    if values.len() > cap {
        return Err(CompleteEpisodeError::CollectionLimit.into());
    }
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        result.push(mapper(value)?);
    }
    Ok(result)
}
pub(super) fn token<T>(
    value: C,
    constructor: impl FnOnce(&str) -> super::super::Result<T>,
) -> DResult<T> {
    let value = text(value)?;
    if value.len() > 128 {
        return Err(CompleteEpisodeError::InvalidIdentifier.into());
    }
    Ok(constructor(&value)?)
}
pub(super) fn ppm(value: C) -> DResult<UnitPpm> {
    Ok(UnitPpm::new(
        unsigned(value)?.try_into().map_err(|_| E::InvalidShape)?,
    )?)
}
pub(super) fn digest(value: C) -> DResult<Digest32> {
    Ok(Digest32::new(fixed(value)?))
}
pub(super) fn utc(value: C) -> DResult<UtcMillis> {
    Ok(UtcMillis::new(unsigned(value)?))
}
pub(super) fn estimate<T>(value: C, mapper: impl FnOnce(C) -> DResult<T>) -> DResult<Estimate<T>> {
    let values = array(value)?;
    let len = values.len();
    let mut values = values.into_iter();
    match (unsigned(next(&mut values)?)?, len) {
        (0, 1) => Ok(Estimate::Unknown),
        (1, 2) => Ok(Estimate::Known(mapper(next(&mut values)?)?)),
        _ => Err(E::InvalidShape),
    }
}
pub(super) struct Fields(Vec<(String, C)>);
impl Fields {
    pub(super) fn new(value: C, required: &[&str], optional: &[&str]) -> DResult<Self> {
        let C::Map(values) = value else {
            return Err(E::InvalidShape);
        };
        if values.len() < required.len() || values.len() > required.len() + optional.len() {
            return Err(E::InvalidShape);
        }
        let mut fields = Vec::with_capacity(values.len());
        for (key, value) in values {
            let key = text(key)?;
            if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
                return Err(E::InvalidShape);
            }
            fields.push((key, value));
        }
        if required
            .iter()
            .any(|key| !fields.iter().any(|(k, _)| k == key))
        {
            return Err(E::InvalidShape);
        }
        Ok(Self(fields))
    }
    pub(super) fn optional<T>(
        &mut self,
        key: &str,
        mapper: impl FnOnce(C) -> DResult<T>,
    ) -> DResult<Option<T>> {
        match self.0.iter().position(|(k, _)| k == key) {
            Some(index) => mapper(self.0.swap_remove(index).1).map(Some),
            None => Ok(None),
        }
    }
    pub(super) fn take(&mut self, key: &str) -> DResult<C> {
        self.optional(key, Ok)?.ok_or(E::InvalidShape)
    }
}
pub(super) fn unit(value: C) -> DResult<u64> {
    let values = array(value)?;
    if values.len() != 1 {
        return Err(E::InvalidShape);
    }
    unsigned(next(&mut values.into_iter())?)
}
pub(super) fn variant(value: C) -> DResult<(u64, C)> {
    let values = array(value)?;
    if values.len() != 2 {
        return Err(E::InvalidShape);
    }
    let mut values = values.into_iter();
    let tag = unsigned(next(&mut values)?)?;
    let fields = next(&mut values)?;
    if !matches!(fields, C::Map(_)) {
        return Err(E::InvalidShape);
    }
    Ok((tag, fields))
}
