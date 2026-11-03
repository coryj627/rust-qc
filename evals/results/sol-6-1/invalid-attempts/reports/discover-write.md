The workspace is read-only, so I couldn’t apply changes. This replacement for [src/lib.rs](/work/crate/src/lib.rs) includes the implementation and tests. Formatting passed; compilation, clippy, rustdoc, and tests remain unverified.

```rust
//! Small network configuration helpers.

use std::fmt;

/// The reason a port string could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParsePortError {
    /// The input is empty.
    Empty,
    /// The input contains a character other than an ASCII decimal digit.
    InvalidCharacter {
        /// The zero-based byte offset of the character.
        index: usize,
        /// The offending character.
        character: char,
    },
    /// The input represents zero.
    Zero,
    /// The numeric value exceeds 65535.
    Overflow,
}

impl fmt::Display for ParsePortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("port is empty"),
            Self::InvalidCharacter { index, character } => {
                write!(f, "invalid port character {character:?} at byte {index}")
            }
            Self::Zero => f.write_str("port must be greater than zero"),
            Self::Overflow => f.write_str("port exceeds 65535"),
        }
    }
}

impl std::error::Error for ParsePortError {}

/// Parses an ASCII decimal port in the inclusive range 1 through 65535.
///
/// Leading zeros are accepted. Whitespace and signs are rejected.
///
/// # Examples
///
/// ```
/// use discovery_fixture::{parse_port, ParsePortError};
///
/// assert_eq!(parse_port("00443")?, 443);
/// assert_eq!(parse_port("0"), Err(ParsePortError::Zero));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns [`ParsePortError`] for empty input, non-ASCII-decimal characters,
/// zero, or values greater than 65535.
pub fn parse_port(input: &str) -> Result<u16, ParsePortError> {
    if input.is_empty() {
        return Err(ParsePortError::Empty);
    }

    let mut port = 0_u16;
    for (index, character) in input.char_indices() {
        if !character.is_ascii_digit() {
            return Err(ParsePortError::InvalidCharacter { index, character });
        }
        let digit = character as u16 - u16::from(b'0');
        port = port
            .checked_mul(10)
            .and_then(|value| value.checked_add(digit))
            .ok_or(ParsePortError::Overflow)?;
    }

    if port == 0 {
        return Err(ParsePortError::Zero);
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_every_valid_port_and_leading_zeros() {
        for port in 1..=u16::MAX {
            assert_eq!(parse_port(&port.to_string()), Ok(port));
            assert_eq!(parse_port(&format!("00000{port}")), Ok(port));
        }
        assert_eq!(parse_port(&format!("{}1", "0".repeat(1000))), Ok(1));
    }

    #[test]
    fn distinguishes_empty_zero_and_overflow() {
        for (input, error) in [
            ("", ParsePortError::Empty),
            ("0", ParsePortError::Zero),
            ("00000", ParsePortError::Zero),
            ("65536", ParsePortError::Overflow),
            ("00065536", ParsePortError::Overflow),
            ("18446744073709551616", ParsePortError::Overflow),
        ] {
            assert_eq!(parse_port(input), Err(error), "input: {input:?}");
        }
        assert_eq!(parse_port(&"9".repeat(1000)), Err(ParsePortError::Overflow));
    }

    #[test]
    fn rejects_invalid_characters_with_byte_positions() {
        for (input, index, character) in [
            (" 80", 0, ' '),
            ("80 ", 2, ' '),
            ("8 0", 1, ' '),
            ("\t80", 0, '\t'),
            ("80\n", 2, '\n'),
            ("+80", 0, '+'),
            ("-80", 0, '-'),
            ("8.0", 1, '.'),
            ("0x50", 1, 'x'),
            ("8_0", 1, '_'),
            ("1e2", 1, 'e'),
            ("80\0", 2, '\0'),
            ("12é3", 2, 'é'),
            ("８０", 0, '８'),
            ("١", 0, '١'),
        ] {
            assert_eq!(
                parse_port(input),
                Err(ParsePortError::InvalidCharacter { index, character }),
                "input: {input:?}"
            );
        }
    }
}
```