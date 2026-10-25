//! Integer ranges in messages.

/// The Ints from `min` to `max`, the bounds of a Rust integer type, as words
/// to follow "Int" or "to be": as much of that range as an Int covers.
pub(crate) fn int_range<T: TryInto<i128>>(min: T, max: T) -> String {
    // Of the integer types' bounds, only `u128::MAX` is beyond an `i128`; it is
    // beyond an Int too, so `i128::MAX` stands in for it.
    let min = min.try_into().unwrap_or(i128::MIN).max(i64::MIN.into());
    let max = max.try_into().unwrap_or(i128::MAX).min(i64::MAX.into());
    if max == i64::MAX.into() {
        format!("at least {min}")
    } else if min == i64::MIN.into() {
        format!("at most {max}")
    } else {
        format!("from {min} to {max}")
    }
}
