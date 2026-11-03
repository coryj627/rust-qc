//! Half-open byte spans with checked construction and UTF-8 substring access.
//!
//! [`ByteSpan`] stores byte offsets, not character indices. It always satisfies
//! `start <= end`, but need not fit a particular string until it is sliced.
//! [`SpanError`] distinguishes reversed bounds, out-of-bounds slices, and
//! endpoints inside UTF-8 code points. No operation allocates or panics on
//! invalid offsets.
//!
//! # Examples
//!
//! ```
//! use byte_span::ByteSpan;
//!
//! let text = "aé🙂z";
//! let span = ByteSpan::new(1, 7)?;
//! assert_eq!(span.len(), 6);
//! assert_eq!(span.slice(text)?, "é🙂");
//! assert!(span.contains(1));
//! assert!(!span.contains(7));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::fmt;

/// A half-open byte interval whose start never exceeds its end.
///
/// Offsets and lengths are measured in bytes. Empty spans are valid, including
/// at [`usize::MAX`]. Construction does not check any particular string's
/// length or UTF-8 boundaries; [`Self::slice`] performs those checks.
///
/// # Examples
///
/// ```
/// use byte_span::ByteSpan;
///
/// let span = ByteSpan::new(2, 5)?;
/// assert_eq!((span.start(), span.end(), span.len()), (2, 5, 3));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Fields are private, so callers cannot bypass validation:
///
/// ```compile_fail
/// use byte_span::ByteSpan;
/// let reversed = ByteSpan { start: 5, end: 2 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    start: usize,
    end: usize,
}

impl ByteSpan {
    /// Creates a half-open span from byte offsets, accepting equal endpoints.
    ///
    /// # Examples
    ///
    /// ```
    /// use byte_span::{ByteSpan, SpanError};
    ///
    /// assert!(ByteSpan::new(4, 4)?.is_empty());
    /// assert!(matches!(ByteSpan::new(5, 2), Err(SpanError::Reversed { .. })));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::Reversed`] if `start > end`.
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start > end {
            return Err(SpanError::Reversed { start, end });
        }
        Ok(Self { start, end })
    }

    /// Returns the inclusive starting byte offset.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// assert_eq!(ByteSpan::new(2, 5)?.start(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the exclusive ending byte offset.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// assert_eq!(ByteSpan::new(2, 5)?.end(), 5);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the span's length in bytes, including zero for empty spans.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// assert_eq!(ByteSpan::new(0, usize::MAX)?.len(), usize::MAX);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns whether the span contains zero bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// assert!(ByteSpan::new(5, 5)?.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Tests whether a byte offset lies within the half-open interval.
    ///
    /// Includes the start, excludes the end, and is always false for empty spans.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// let span = ByteSpan::new(2, 5)?;
    /// assert!(span.contains(2));
    /// assert!(!span.contains(5));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Returns the positive-length overlap, or none for touching or empty spans.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// let span = ByteSpan::new(2, 6)?;
    /// assert_eq!(span.intersection(&ByteSpan::new(4, 8)?), Some(ByteSpan::new(4, 6)?));
    /// assert_eq!(span.intersection(&ByteSpan::new(6, 8)?), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Borrows the UTF-8 substring at this span's byte offsets.
    ///
    /// An empty span at `text.len()` returns an empty slice. The returned
    /// reference borrows `text` and may outlive the span.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::{ByteSpan, SpanError};
    /// let text = "aé🙂z";
    /// assert_eq!(ByteSpan::new(1, 3)?.slice(text)?, "é");
    /// assert_eq!(ByteSpan::new(text.len(), text.len())?.slice(text)?, "");
    /// assert!(matches!(
    ///     ByteSpan::new(2, 3)?.slice(text),
    ///     Err(SpanError::InvalidUtf8Boundary { offset: 2 })
    /// ));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::OutOfBounds`] if the end exceeds `text.len()`,
    /// before checking either UTF-8 boundary. Otherwise returns
    /// [`SpanError::InvalidUtf8Boundary`] if an endpoint lies inside a code
    /// point, reporting the start first when both endpoints are invalid.
    pub fn slice<'a>(&self, text: &'a str) -> Result<&'a str, SpanError> {
        if self.end > text.len() {
            return Err(SpanError::OutOfBounds {
                start: self.start,
                end: self.end,
                len: text.len(),
            });
        }
        if !text.is_char_boundary(self.start) {
            return Err(SpanError::InvalidUtf8Boundary { offset: self.start });
        }
        // Ordering and bounds are validated, so only the end boundary can fail.
        text.get(self.start..self.end)
            .ok_or(SpanError::InvalidUtf8Boundary { offset: self.end })
    }
}

/// A construction or slicing failure, carrying byte offsets for typed inspection.
///
/// Match variants to distinguish failures without parsing the display message.
///
/// # Examples
///
/// ```
/// use byte_span::{ByteSpan, SpanError};
///
/// match ByteSpan::new(0, 4)?.slice("é") {
///     Err(SpanError::OutOfBounds { end, len, .. }) => assert_eq!((end, len), (4, 2)),
///     result => panic!("expected a bounds error, got {result:?}"),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpanError {
    /// The starting byte offset exceeds the ending byte offset.
    Reversed {
        /// The requested starting byte offset.
        start: usize,
        /// The requested ending byte offset.
        end: usize,
    },
    /// The span's end exceeds the input's byte length.
    OutOfBounds {
        /// The span's starting byte offset.
        start: usize,
        /// The span's ending byte offset.
        end: usize,
        /// The input's length in bytes.
        len: usize,
    },
    /// An in-bounds endpoint falls inside a UTF-8 code point.
    InvalidUtf8Boundary {
        /// The invalid byte offset, preferring the start if both are invalid.
        offset: usize,
    },
}

impl fmt::Display for SpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reversed { start, end } => {
                write!(f, "start byte offset {start} exceeds end byte offset {end}")
            }
            Self::OutOfBounds { start, end, len } => {
                write!(
                    f,
                    "byte span [{start}, {end}) exceeds input length {len} bytes"
                )
            }
            Self::InvalidUtf8Boundary { offset } => {
                write!(f, "byte offset {offset} is not a UTF-8 boundary")
            }
        }
    }
}

impl std::error::Error for SpanError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_preserves_bounds_and_rejects_reversal() {
        let offsets = [0, 1, 2, usize::MAX / 2, usize::MAX - 1, usize::MAX];
        for start in offsets {
            for end in offsets {
                match ByteSpan::new(start, end) {
                    Ok(span) => {
                        assert!(start <= end, "accepted reversed bounds {start}..{end}");
                        assert_eq!((span.start(), span.end()), (start, end));
                        assert_eq!(span.is_empty(), start == end);
                        assert_eq!(span.len().checked_add(start), Some(end));
                    }
                    Err(error) => {
                        assert!(start > end, "rejected ordered bounds {start}..{end}");
                        assert_eq!(error, SpanError::Reversed { start, end });
                    }
                }
            }
        }
    }

    #[test]
    fn membership_includes_only_bytes_before_the_end() {
        let span = ByteSpan::new(2, 5).unwrap();
        let members: Vec<_> = (0..8).filter(|&offset| span.contains(offset)).collect();
        assert_eq!(members, [2, 3, 4]);
        let empty = ByteSpan::new(3, 3).unwrap();
        assert!(
            !(0..8).any(|offset| empty.contains(offset)),
            "empty span contains a byte"
        );
    }

    #[test]
    fn full_usize_range_does_not_overflow() {
        let full = ByteSpan::new(0, usize::MAX).unwrap();
        let last = ByteSpan::new(usize::MAX - 1, usize::MAX).unwrap();
        let empty = ByteSpan::new(usize::MAX, usize::MAX).unwrap();
        assert_eq!(full.len(), usize::MAX);
        assert_eq!(last.len(), 1);
        assert_eq!(empty.len(), 0);
        assert!(full.contains(0), "full span should contain its start");
        assert!(
            full.contains(usize::MAX - 1),
            "full span should contain its final byte"
        );
        assert!(!full.contains(usize::MAX), "end must be excluded");
        assert!(
            !last.contains(usize::MAX - 2),
            "offset before start must be excluded"
        );
        assert!(
            !empty.contains(usize::MAX),
            "empty span must exclude its endpoint"
        );
        assert_eq!(full.intersection(&last), Some(last));
        assert_eq!(last.intersection(&full), Some(last));
        assert_eq!(full.intersection(&empty), None);
        assert_eq!(
            last.intersection(&ByteSpan::new(0, usize::MAX - 1).unwrap()),
            None
        );
    }

    #[test]
    fn intersections_match_shared_byte_membership_for_all_small_spans() {
        let spans: Vec<_> = (0..=6)
            .flat_map(|start| (start..=6).map(move |end| ByteSpan::new(start, end).unwrap()))
            .collect();
        for a in &spans {
            for b in &spans {
                let shared: Vec<_> = (a.start()..a.end())
                    .filter(|offset| (b.start()..b.end()).contains(offset))
                    .collect();
                let intersection = a.intersection(b);
                assert_eq!(
                    intersection,
                    b.intersection(a),
                    "asymmetric overlap: {a:?}, {b:?}"
                );
                assert_eq!(
                    intersection.is_none(),
                    shared.is_empty(),
                    "empty overlap: {a:?}, {b:?}"
                );
                if let Some(overlap) = intersection {
                    assert!(!overlap.is_empty(), "returned empty overlap: {a:?}, {b:?}");
                    assert_eq!((overlap.start()..overlap.end()).collect::<Vec<_>>(), shared);
                }
            }
        }
    }

    #[test]
    fn slicing_matches_checked_str_access_for_all_small_endpoint_pairs() {
        for text in ["", "ascii", "é", "中", "🙂", "aé中🙂z"] {
            for start in 0..=text.len() + 2 {
                for end in start..=text.len() + 2 {
                    let span = ByteSpan::new(start, end).unwrap();
                    assert_eq!(
                        span.slice(text).ok(),
                        text.get(start..end),
                        "slice {span:?} of {text:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn slicing_reports_each_invalid_endpoint_and_prefers_start() {
        for (start, end, offset) in [(1, 3, 1), (0, 2, 2), (1, 2, 1), (1, 1, 1)] {
            assert_eq!(
                ByteSpan::new(start, end).unwrap().slice("中"),
                Err(SpanError::InvalidUtf8Boundary { offset })
            );
        }
    }

    #[test]
    fn bounds_errors_precede_boundary_errors_even_at_usize_max() {
        for (start, end) in [(1, 3), (3, 3), (1, usize::MAX), (usize::MAX, usize::MAX)] {
            assert_eq!(
                ByteSpan::new(start, end).unwrap().slice("é"),
                Err(SpanError::OutOfBounds { start, end, len: 2 })
            );
        }
        assert_eq!(
            ByteSpan::new(0, usize::MAX).unwrap().slice(""),
            Err(SpanError::OutOfBounds {
                start: 0,
                end: usize::MAX,
                len: 0
            })
        );
    }

    #[test]
    fn empty_slices_work_at_every_utf8_boundary_including_input_end() {
        let text = "aé中🙂z";
        for offset in [0, 1, 3, 6, 10, 11] {
            assert_eq!(ByteSpan::new(offset, offset).unwrap().slice(text), Ok(""));
        }
        assert_eq!(ByteSpan::new(0, 0).unwrap().slice(""), Ok(""));
    }

    #[test]
    fn slice_borrows_the_original_text_independently_of_the_span() {
        let text = String::from("aé🙂z");
        let slice = {
            let span = ByteSpan::new(1, 7).unwrap();
            span.slice(&text).unwrap()
        };
        assert_eq!(slice, "é🙂");
        assert_eq!(slice.as_ptr(), text[1..].as_ptr());
    }

    #[test]
    fn errors_implement_display_and_standard_error_without_a_source() {
        let cases = [
            (
                SpanError::Reversed { start: 5, end: 2 },
                "start byte offset 5 exceeds end byte offset 2",
            ),
            (
                SpanError::OutOfBounds {
                    start: 1,
                    end: 9,
                    len: 3,
                },
                "byte span [1, 9) exceeds input length 3 bytes",
            ),
            (
                SpanError::InvalidUtf8Boundary { offset: 2 },
                "byte offset 2 is not a UTF-8 boundary",
            ),
        ];
        for (error, message) in cases {
            let error: &dyn std::error::Error = &error;
            assert_eq!(error.to_string(), message);
            assert!(
                error.source().is_none(),
                "span error has no underlying cause"
            );
        }
    }
}
