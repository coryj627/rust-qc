//! Half-open byte spans with checked construction and UTF-8 substring access.
//!
//! [`ByteSpan`] represents `[start, end)` in bytes, independently of any text.
//! Its endpoints always satisfy `start <= end`; empty spans are valid. Text
//! bounds and UTF-8 boundaries are checked only by [`ByteSpan::slice`].
//! [`SpanError`] identifies validation failures without parsing error messages.
//!
//! # Examples
//!
//! ```
//! use byte_span::ByteSpan;
//!
//! let text = "a🦀z";
//! let crab = ByteSpan::new(1, 5)?;
//! assert_eq!(crab.len(), 4);
//! assert_eq!(crab.slice(text)?, "🦀");
//! assert_eq!(crab.intersection(&ByteSpan::new(3, 6)?), Some(ByteSpan::new(3, 5)?));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::fmt;

/// A half-open byte interval whose start never exceeds its end.
///
/// The endpoints are byte offsets, not character indices. A span is independent
/// of any input and may use the full `usize` range. Its private fields preserve
/// `start <= end`, so its length is always representable without overflow.
/// Use [`Self::slice`] to validate a span against UTF-8 text.
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
/// Callers cannot bypass validation by constructing a span directly:
///
/// ```compile_fail
/// use byte_span::ByteSpan;
///
/// let reversed = ByteSpan { start: 5, end: 2 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    start: usize,
    end: usize,
}

impl ByteSpan {
    /// Creates a span from an inclusive start and an exclusive end, in bytes.
    ///
    /// Equal endpoints create an empty span, including at `usize::MAX`.
    /// Construction does not validate bounds or UTF-8 boundaries against text.
    ///
    /// # Examples
    ///
    /// ```
    /// use byte_span::ByteSpan;
    ///
    /// assert!(ByteSpan::new(3, 3)?.is_empty());
    /// assert!(ByteSpan::new(4, 3).is_err());
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

    /// Returns the inclusive start offset, measured in bytes.
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

    /// Returns the exclusive end offset, measured in bytes.
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

    /// Returns the number of bytes in the span without overflow.
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

    /// Returns whether a byte offset is inside this half-open span.
    ///
    /// Includes the start and excludes the end. Empty spans contain no offsets.
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

    /// Returns the overlapping byte interval only when it has positive length.
    ///
    /// Touching spans and intersections involving an empty span return `None`.
    /// The result need not align with UTF-8 boundaries in any particular text.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::ByteSpan;
    /// let span = ByteSpan::new(2, 5)?;
    /// assert_eq!(span.intersection(&ByteSpan::new(4, 7)?), Some(ByteSpan::new(4, 5)?));
    /// assert_eq!(span.intersection(&ByteSpan::new(5, 7)?), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
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

    /// Borrows the UTF-8 substring between this span's byte endpoints.
    ///
    /// An empty span at `text.len()` returns an empty slice. Empty spans inside
    /// a UTF-8 code point are invalid, just like nonempty spans at that offset.
    /// The returned slice borrows from `text`, independently of this span.
    ///
    /// # Examples
    ///
    /// ```
    /// # use byte_span::{ByteSpan, SpanError};
    /// let text = "aéz";
    /// assert_eq!(ByteSpan::new(1, 3)?.slice(text)?, "é");
    /// assert_eq!(ByteSpan::new(4, 4)?.slice(text)?, "");
    /// assert_eq!(ByteSpan::new(2, 3)?.slice(text),
    ///            Err(SpanError::InvalidUtf8Boundary { offset: 2 }));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::OutOfBounds`] if the end exceeds `text.len()`, before
    /// checking either UTF-8 boundary. Otherwise, returns
    /// [`SpanError::InvalidUtf8Boundary`] if an endpoint lies inside a code point.
    /// If both endpoints are invalid boundaries, reports the start first.
    pub fn slice<'a>(&self, text: &'a str) -> Result<&'a str, SpanError> {
        if self.end > text.len() {
            return Err(SpanError::OutOfBounds {
                end: self.end,
                text_len: text.len(),
            });
        }
        for offset in [self.start, self.end] {
            if !text.is_char_boundary(offset) {
                return Err(SpanError::InvalidUtf8Boundary { offset });
            }
        }
        // Construction guarantees ordering; the checks above guarantee bounds
        // and UTF-8 alignment, so indexing cannot panic on caller input.
        Ok(&text[self.start..self.end])
    }
}

/// A construction or slicing failure with offsets and lengths measured in bytes.
///
/// Match the variants to distinguish failures without parsing display text.
/// These errors contain positions only and never retain the input text.
///
/// # Examples
///
/// ```
/// use byte_span::{ByteSpan, SpanError};
///
/// match ByteSpan::new(5, 2) {
///     Err(SpanError::Reversed { start, end }) => assert_eq!((start, end), (5, 2)),
///     result => panic!("expected reversed endpoints, got {result:?}"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpanError {
    /// The start byte offset exceeds the end byte offset during construction.
    Reversed {
        /// The rejected start byte offset.
        start: usize,
        /// The rejected end byte offset.
        end: usize,
    },
    /// The end byte offset exceeds the text's length during slicing.
    OutOfBounds {
        /// The span's end byte offset.
        end: usize,
        /// The input text's length in bytes.
        text_len: usize,
    },
    /// An endpoint lies inside a UTF-8 code point during slicing.
    InvalidUtf8Boundary {
        /// The first invalid byte offset, checking the start before the end.
        offset: usize,
    },
}

impl fmt::Display for SpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reversed { start, end } => {
                write!(f, "start byte offset {start} exceeds end byte offset {end}")
            }
            Self::OutOfBounds { end, text_len } => {
                write!(
                    f,
                    "end byte offset {end} exceeds text length {text_len} bytes"
                )
            }
            Self::InvalidUtf8Boundary { offset } => {
                write!(f, "byte offset {offset} is not a UTF-8 boundary")
            }
        }
    }
}

impl std::error::Error for SpanError {}
