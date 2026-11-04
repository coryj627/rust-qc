//! Duration conversion helpers.

/// Converts whole minutes to whole seconds, saturating at the maximum value of [`u64`].
///
/// The input `minutes` is a count of minutes; the return value is a count of
/// seconds, using 60 seconds per minute. If the conversion would exceed
/// [`u64::MAX`], returns [`u64::MAX`] without wrapping or panicking.
///
/// # Examples
///
/// Convert minutes to seconds:
///
/// ```
/// use discovery_fixture::saturating_seconds;
///
/// assert_eq!(saturating_seconds(0), 0);
/// assert_eq!(saturating_seconds(2), 120);
/// ```
///
/// The largest input whose conversion fits is converted exactly; larger inputs
/// saturate:
///
/// ```
/// use discovery_fixture::saturating_seconds;
///
/// let largest_exact_minutes = u64::MAX / 60;
/// assert_eq!(
///     saturating_seconds(largest_exact_minutes),
///     largest_exact_minutes * 60,
/// );
/// assert_eq!(saturating_seconds(largest_exact_minutes + 1), u64::MAX);
/// assert_eq!(saturating_seconds(u64::MAX), u64::MAX);
/// ```
pub fn saturating_seconds(minutes: u64) -> u64 {
    minutes.saturating_mul(60)
}
