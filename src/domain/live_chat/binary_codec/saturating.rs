pub(super) fn saturating_u8<T>(value: T) -> u8
where
    T: TryInto<u8>,
{
    value.try_into().map_or(u8::MAX, |value| value)
}

pub(super) fn saturating_u16<T>(value: T) -> u16
where
    T: TryInto<u16>,
{
    value.try_into().map_or(u16::MAX, |value| value)
}

pub(super) fn saturating_u32<T>(value: T) -> u32
where
    T: TryInto<u32>,
{
    value.try_into().map_or(u32::MAX, |value| value)
}
