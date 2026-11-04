//! Half-open spans measured in **bytes**, with checked UTF-8 slicing.
//!
//! A [`ByteSpan`] represents `[start, end)` and always satisfies `start <= end`.
//! Empty spans are valid, but intersections must have positive length.
//! Offsets are independent of any particular string; [`ByteSpan::slice`] checks
//! them against the supplied string before borrowing a substring.
//!
//! ```
//! use byte_span::{ByteSpan, SpanError};
//!
//! let text = "aé🦀z";
//! let span = ByteSpan::new(1, 7)?;
//! assert_eq!(span.len(), 6); // Two bytes for é, four for 🦀.
//! assert_eq!(span.slice(text)?, "é🦀");
//! assert!(span.contains(1));
//! assert!(!span.contains(7));
//!
//! let other = ByteSpan::new(3, 8)?;
//! assert_eq!(span.intersection(&other), Some(ByteSpan::new(3, 7)?));
//! assert_eq!(ByteSpan::new(8, 8)?.slice(text)?, "");
//!
//! // Errors can be matched without inspecting their display text.
//! assert_eq!(
//!     ByteSpan::new(2, 3)?.slice(text),
//!     Err(SpanError::InvalidCharBoundary { offset: 2 }),
//! );
//! # Ok::<(), SpanError>(())
//! ```

use std::error::Error;
use std::fmt;

/// A half-open byte interval `[start, end)` with `start <= end`.
///
/// The endpoints are private so all publicly constructible spans uphold this
/// invariant. An empty span has equal endpoints. A span need not fit a string
/// or align with UTF-8 code points until it is passed to [`Self::slice`].
///
/// ```compile_fail
/// use byte_span::ByteSpan;
///
/// // Private endpoints prevent bypassing the checked constructor.
/// let reversed = ByteSpan { start: 5, end: 2 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    start: usize,
    end: usize,
}

impl ByteSpan {
    /// Creates a span whose endpoints are byte offsets.
    ///
    /// Equal endpoints are allowed, including `usize::MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::Reversed`] if `start > end`.
    ///
    /// ```
    /// use byte_span::{ByteSpan, SpanError};
    ///
    /// assert!(ByteSpan::new(4, 4)?.is_empty());
    /// assert_eq!(
    ///     ByteSpan::new(4, 2),
    ///     Err(SpanError::Reversed { start: 4, end: 2 }),
    /// );
    /// # Ok::<(), SpanError>(())
    /// ```
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start > end {
            Err(SpanError::Reversed { start, end })
        } else {
            Ok(Self { start, end })
        }
    }

    /// Returns the inclusive start byte offset.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the exclusive end byte offset.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the length in bytes, which is zero for an empty span.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns whether the start and end byte offsets are equal.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns whether `offset` lies in `[start, end)`.
    ///
    /// An empty span contains no offsets. This checks only byte offsets, not
    /// UTF-8 boundaries.
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Returns the overlap only when it has positive byte length.
    ///
    /// Disjoint spans, touching spans, and intersections involving an empty
    /// span return `None`.
    ///
    /// ```
    /// use byte_span::ByteSpan;
    ///
    /// let span = ByteSpan::new(1, 4)?;
    /// assert_eq!(span.intersection(&ByteSpan::new(3, 5)?),
    ///            Some(ByteSpan::new(3, 4)?));
    /// assert_eq!(span.intersection(&ByteSpan::new(4, 5)?), None);
    /// # Ok::<(), byte_span::SpanError>(())
    /// ```
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
    /// Empty spans return an empty slice when their offset is a character
    /// boundary, including at `text.len()`.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::OutOfBounds`] if the end exceeds `text.len()`.
    /// This check takes precedence over UTF-8 boundary checks. Otherwise,
    /// returns [`SpanError::InvalidCharBoundary`] if either endpoint lies
    /// inside a UTF-8 code point, checking the start before the end.
    ///
    /// ```
    /// use byte_span::{ByteSpan, SpanError};
    ///
    /// assert_eq!(ByteSpan::new(0, 2)?.slice("é")?, "é");
    /// assert_eq!(ByteSpan::new(2, 2)?.slice("é")?, "");
    /// // The bounds error wins even though byte 1 is inside é.
    /// assert_eq!(ByteSpan::new(1, 3)?.slice("é"),
    ///            Err(SpanError::OutOfBounds { end: 3, len: 2 }));
    /// # Ok::<(), SpanError>(())
    /// ```
    pub fn slice<'a>(&self, text: &'a str) -> Result<&'a str, SpanError> {
        if self.end > text.len() {
            return Err(SpanError::OutOfBounds {
                end: self.end,
                len: text.len(),
            });
        }
        for offset in [self.start, self.end] {
            if !text.is_char_boundary(offset) {
                return Err(SpanError::InvalidCharBoundary { offset });
            }
        }
        // The span invariant and the checks above make indexing valid.
        Ok(&text[self.start..self.end])
    }
}

/// A structured error from span construction or UTF-8 slicing.
///
/// Every offset and length is measured in bytes. [`ByteSpan::slice`] checks
/// bounds before boundaries, and reports the start boundary before the end
/// boundary if both are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpanError {
    /// Construction failed because `start > end`.
    Reversed {
        /// The requested inclusive start byte offset.
        start: usize,
        /// The requested exclusive end byte offset.
        end: usize,
    },
    /// Slicing failed because `end > len`.
    OutOfBounds {
        /// The span's exclusive end byte offset.
        end: usize,
        /// The input string's length in bytes.
        len: usize,
    },
    /// An in-bounds endpoint lies inside a UTF-8 code point.
    InvalidCharBoundary {
        /// The offending byte offset.
        offset: usize,
    },
}

impl fmt::Display for SpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reversed { start, end } => {
                write!(f, "start byte offset {start} exceeds end byte offset {end}")
            }
            Self::OutOfBounds { end, len } => {
                write!(f, "end byte offset {end} exceeds string byte length {len}")
            }
            Self::InvalidCharBoundary { offset } => {
                write!(f, "byte offset {offset} is not a UTF-8 character boundary")
            }
        }
    }
}

impl Error for SpanError {}
