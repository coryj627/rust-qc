The workspace is read-only, so I couldn’t apply the change. Add this rustdoc above [saturating_seconds](/work/crate/src/lib.rs:3), preserving its signature and body:

````rust
/// Converts whole minutes to seconds, saturating at the largest representable number of seconds.
///
/// Each input minute represents 60 seconds. If the mathematical result exceeds
/// [`u64::MAX`], returns [`u64::MAX`] seconds without wrapping or panicking.
///
/// # Examples
///
/// ```
/// use discovery_fixture::saturating_seconds;
///
/// assert_eq!(saturating_seconds(0), 0);
/// assert_eq!(saturating_seconds(2), 120);
/// ```
///
/// The largest input whose conversion fits is exact; larger inputs saturate:
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
````

The existing formatting check passed. Clippy, documentation generation, and tests were skipped because they require filesystem writes; the proposed examples remain unverified.