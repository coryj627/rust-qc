//! Small network configuration helpers.

use std::fmt;

/// A failure to parse an ASCII decimal port in the range 1 through 65535.
///
/// Use [`Self::kind`] to distinguish the cause without inspecting the error message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParsePortError {
    kind: ParsePortErrorKind,
}

impl ParsePortError {
    /// Returns the cause of the parsing failure, including any offending character's byte position.
    pub fn kind(&self) -> ParsePortErrorKind {
        self.kind
    }
}

/// The cause of a port parsing failure, with positions measured in zero-based bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ParsePortErrorKind {
    /// The input contains no characters.
    Empty,
    /// The input contains a character other than an ASCII decimal digit.
    InvalidCharacter {
        /// The first character that is not an ASCII decimal digit.
        character: char,
        /// The zero-based byte position of the character in the input.
        position: usize,
    },
    /// The input represents zero, which is not an accepted port.
    Zero,
    /// The decimal value exceeds 65535.
    Overflow {
        /// The zero-based byte position of the first digit that exceeds the range.
        position: usize,
    },
}

impl fmt::Display for ParsePortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ParsePortErrorKind::Empty => f.write_str("port is empty"),
            ParsePortErrorKind::InvalidCharacter {
                character,
                position,
            } => write!(f, "invalid port character {character:?} at byte {position}"),
            ParsePortErrorKind::Zero => f.write_str("port must be at least 1"),
            ParsePortErrorKind::Overflow { position } => {
                write!(f, "port exceeds 65535 at byte {position}")
            }
        }
    }
}

impl std::error::Error for ParsePortError {}

/// Parses a nonempty ASCII decimal string as a port from 1 through 65535.
///
/// Leading zeros are accepted. Whitespace, signs, and non-ASCII digits are rejected.
///
/// # Examples
///
/// ```
/// use discovery_fixture::{parse_port, ParsePortErrorKind};
///
/// assert_eq!(parse_port("00443")?, 443);
/// assert_eq!(parse_port("0").unwrap_err().kind(), ParsePortErrorKind::Zero);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns [`ParsePortError`] for empty input, a non-ASCII-decimal character, zero,
/// or a value exceeding 65535. Invalid characters are reported before range errors.
pub fn parse_port(input: &str) -> Result<u16, ParsePortError> {
    if input.is_empty() {
        return Err(ParsePortError {
            kind: ParsePortErrorKind::Empty,
        });
    }

    if let Some((position, character)) = input
        .char_indices()
        .find(|(_, character)| !character.is_ascii_digit())
    {
        return Err(ParsePortError {
            kind: ParsePortErrorKind::InvalidCharacter {
                character,
                position,
            },
        });
    }

    let mut port = 0_u16;
    for (position, byte) in input.bytes().enumerate() {
        port = port
            .checked_mul(10)
            .and_then(|value| value.checked_add(u16::from(byte - b'0')))
            .ok_or(ParsePortError {
                kind: ParsePortErrorKind::Overflow { position },
            })?;
    }

    if port == 0 {
        return Err(ParsePortError {
            kind: ParsePortErrorKind::Zero,
        });
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::{parse_port, ParsePortErrorKind};

    #[test]
    fn every_valid_port_round_trips_with_optional_leading_zeros() {
        for port in 1..=u16::MAX {
            assert_eq!(parse_port(&port.to_string()), Ok(port));
            assert_eq!(parse_port(&format!("000{port}")), Ok(port));
        }
    }

    #[test]
    fn rejects_empty_input_and_zero() {
        assert_eq!(
            parse_port("").unwrap_err().kind(),
            ParsePortErrorKind::Empty
        );
        for input in ["0", "00", "00000", &"0".repeat(1000)] {
            assert_eq!(
                parse_port(input).unwrap_err().kind(),
                ParsePortErrorKind::Zero,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn rejects_whitespace_signs_and_nondecimal_characters() {
        for character in [
            ' ', '\t', '\n', '\r', '\u{a0}', '+', '-', '.', '_', 'a', '/', ':', '\0', '١', '１',
            '💡',
        ] {
            for position in 0..=3 {
                let mut input = String::from("123");
                input.insert(position, character);
                assert_eq!(
                    parse_port(&input).unwrap_err().kind(),
                    ParsePortErrorKind::InvalidCharacter {
                        character,
                        position,
                    },
                    "input: {input:?}"
                );
            }
        }
    }

    #[test]
    fn rejects_overflow_including_values_larger_than_machine_integers() {
        for input in [
            "65536",
            "655350",
            "99999",
            "18446744073709551616",
            &"9".repeat(1000),
            "00065536",
        ] {
            assert!(
                matches!(
                    parse_port(input).unwrap_err().kind(),
                    ParsePortErrorKind::Overflow { .. }
                ),
                "input: {input:?}"
            );
        }
        assert_eq!(
            parse_port("65536").unwrap_err().kind(),
            ParsePortErrorKind::Overflow { position: 4 }
        );
    }

    #[test]
    fn invalid_characters_take_precedence_over_range_errors() {
        assert_eq!(
            parse_port("999999x").unwrap_err().kind(),
            ParsePortErrorKind::InvalidCharacter {
                character: 'x',
                position: 6,
            }
        );
    }

    #[test]
    fn errors_provide_standard_error_and_diagnostic_context() {
        let error = parse_port("12\n").unwrap_err();
        let standard_error: &dyn std::error::Error = &error;
        assert_eq!(
            standard_error.to_string(),
            "invalid port character '\\n' at byte 2"
        );
        assert!(standard_error.source().is_none(), "no underlying error");
    }
}
