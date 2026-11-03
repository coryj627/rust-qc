//! Half-open spans measured in UTF-8 bytes.
//!
//! ```
//! use byte_span::ByteSpan;
//! let span = ByteSpan::new(1, 3)?;
//! assert_eq!(span.slice("aéz")?, "é");
//! # Ok::<(), byte_span::SpanError>(())
//! ```
use std::fmt;

/// A validated byte interval `[start, end)`; endpoints always satisfy start <= end.
#[derive(Debug)]
pub struct ByteSpan {
    start: usize,
    end: usize,
}

/// A construction or slicing failure, distinguishable without parsing text.
#[derive(Debug)]
pub enum SpanError {
    /// Construction supplied a start greater than the end.
    InvalidOrder,
    /// The span's end exceeded the input byte length.
    OutsideText,
    /// An in-bounds endpoint fell inside a UTF-8 code point.
    NotCharBoundary,
}

impl fmt::Display for SpanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidOrder => "start exceeds end",
            Self::OutsideText => "span exceeds input length",
            Self::NotCharBoundary => "endpoint is inside a UTF-8 code point",
        })
    }
}

impl std::error::Error for SpanError {}

impl ByteSpan {
    /// Construct a byte span, rejecting reversed endpoints with InvalidOrder.
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start > end {
            Err(SpanError::InvalidOrder)
        } else {
            Ok(Self { start, end })
        }
    }

    /// Inclusive start in bytes.
    pub fn start(&self) -> usize {
        self.start
    }
    /// Exclusive end in bytes.
    pub fn end(&self) -> usize {
        self.end
    }
    /// Length in bytes; the construction invariant makes subtraction safe.
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    /// Whether this interval contains no bytes.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
    /// Whether the byte offset is inside the half-open interval.
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset <= self.end
    }
    /// Positive-length overlap; empty or touching intervals have no intersection.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }
    /// Borrow the designated substring. OutsideText takes precedence over
    /// NotCharBoundary. Empty spans at the byte length are valid.
    pub fn slice<'a>(&self, text: &'a str) -> Result<&'a str, SpanError> {
        if self.end > text.len() {
            return Err(SpanError::OutsideText);
        }
        text.get(self.start..self.end)
            .ok_or(SpanError::NotCharBoundary)
    }
}
