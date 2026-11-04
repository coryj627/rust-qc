//! Parses a small, line-oriented configuration format into ordered key/value entries.
//!
//! Lines are separated by LF or CRLF; the final line may omit its ending.
//! Blank lines and lines beginning with `#` after ASCII spaces or tabs are ignored.
//! Data lines contain `key = value`. Keys use ASCII letters, digits, `_`, and `-`.
//! Only ASCII spaces and tabs are trimmed around keys and complete values.
//!
//! Unquoted values may be empty, and `#`, `=`, quotes, and backslashes are literal.
//! A value beginning with `"` is quoted and supports only `\\`, `\"`, `\n`, and
//! `\t` escapes. Unescaped Unicode is preserved. After the closing quote, only
//! spaces and tabs are allowed. Quotes cannot span lines, and a lone CR is content.
//! Keys are case-sensitive and must be unique. [`parse`] preserves input order.
//!
//! # Examples
//!
//! ```
//! use small_config::{parse, Entry};
//!
//! let entries = parse("# Settings\nname = café\nmessage = \"hello\\nworld\"\n")?;
//! assert_eq!(entries, vec![
//!     Entry { key: "name".into(), value: "café".into() },
//!     Entry { key: "message".into(), value: "hello\nworld".into() },
//! ]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]

use std::collections::HashSet;
use std::fmt;

/// An owned key and decoded value from one configuration data line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The case-sensitive key, without surrounding ASCII spaces or tabs.
    pub key: String,
    /// The decoded value, without whitespace outside a quoted value.
    pub value: String,
}

/// A configuration syntax error with its one-based input line number.
///
/// Use [`Self::line`] and [`Self::kind`] to inspect the failure without parsing
/// its display text. Messages do not include configuration values.
///
/// # Examples
///
/// ```
/// use small_config::{parse, ParseErrorKind};
///
/// if let Err(error) = parse("# Settings\ninvalid") {
///     assert_eq!(error.line(), 2);
///     assert_eq!(error.kind(), ParseErrorKind::MissingEquals);
///     assert_eq!(error.to_string(), "line 2: missing '='");
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    kind: ParseErrorKind,
}

/// The reason a configuration line could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// A data line has no equals sign separating its key and value.
    MissingEquals,
    /// A key is empty or contains a character outside ASCII letters, digits, `_`, and `-`.
    InvalidKey,
    /// A key has already appeared on an earlier data line.
    DuplicateKey,
    /// A quoted value has no closing quote on its line.
    UnterminatedQuote,
    /// A quoted value contains an escape other than `\\`, `\"`, `\n`, or `\t`.
    UnknownEscape,
    /// A quoted value ends immediately after a backslash.
    TrailingEscape,
    /// A closing quote is followed by something other than ASCII spaces or tabs.
    TrailingCharacters,
}

impl ParseError {
    /// Returns the one-based input line number where parsing failed.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the syntax error category for programmatic error handling.
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            ParseErrorKind::MissingEquals => "missing '='",
            ParseErrorKind::InvalidKey => "invalid key",
            ParseErrorKind::DuplicateKey => "duplicate key",
            ParseErrorKind::UnterminatedQuote => "missing closing quote",
            ParseErrorKind::UnknownEscape => "unknown escape",
            ParseErrorKind::TrailingEscape => "trailing escape",
            ParseErrorKind::TrailingCharacters => "unexpected text after closing quote",
        };
        write!(f, "line {}: {message}", self.line)
    }
}

impl std::error::Error for ParseError {}

/// Parses configuration text, preserving the order of entries and decoding quoted values.
///
/// See the [crate documentation](crate) for the format and a complete example.
/// Parsing stops at the first invalid line; no partial result is returned.
///
/// # Errors
///
/// Returns [`ParseError`] for a missing equals sign, an invalid or duplicate key,
/// or an invalid quoted value. The error identifies the one-based input line.
pub fn parse(input: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    let mut keys = HashSet::new();

    for (index, segment) in input.split_inclusive('\n').enumerate() {
        let line = if let Some(line) = segment.strip_suffix('\n') {
            line.strip_suffix('\r').unwrap_or(line)
        } else {
            segment
        };
        let line = line.trim_matches([' ', '\t']);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let error = |kind| ParseError {
            line: index + 1,
            kind,
        };
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| error(ParseErrorKind::MissingEquals))?;
        let key = key.trim_end_matches([' ', '\t']);
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(error(ParseErrorKind::InvalidKey));
        }
        if !keys.insert(key) {
            return Err(error(ParseErrorKind::DuplicateKey));
        }

        let value = value.trim_matches([' ', '\t']);
        let value = if let Some(quoted) = value.strip_prefix('"') {
            parse_quoted(quoted).map_err(error)?
        } else {
            value.to_owned()
        };
        entries.push(Entry {
            key: key.to_owned(),
            value,
        });
    }
    Ok(entries)
}

fn parse_quoted(input: &str) -> Result<String, ParseErrorKind> {
    let mut value = String::new();
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if !chars.as_str().trim_matches([' ', '\t']).is_empty() {
                    return Err(ParseErrorKind::TrailingCharacters);
                }
                return Ok(value);
            }
            '\\' => value.push(match chars.next() {
                Some('\\') => '\\',
                Some('"') => '"',
                Some('n') => '\n',
                Some('t') => '\t',
                Some(_) => return Err(ParseErrorKind::UnknownEscape),
                None => return Err(ParseErrorKind::TrailingEscape),
            }),
            _ => value.push(ch),
        }
    }
    Err(ParseErrorKind::UnterminatedQuote)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value: &str) -> Entry {
        Entry {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn ignores_blank_lines_and_indented_comments() {
        for input in ["", "\n", "\r\n", " \t", " \t\n\t# ignored = \"\\q\n# last"] {
            assert_eq!(parse(input).unwrap(), Vec::new(), "input: {input:?}");
        }
    }

    #[test]
    fn preserves_order_and_distinguishes_key_case() {
        let input = " z-9_ = first\r\n\tA\t=\tsecond\n a = third";
        assert_eq!(
            parse(input).unwrap(),
            vec![
                entry("z-9_", "first"),
                entry("A", "second"),
                entry("a", "third")
            ]
        );
    }

    #[test]
    fn trims_only_horizontal_ascii_whitespace() {
        assert_eq!(
            parse("\tk \t= \t\u{a0}café\u{2003} \t\nv=\u{b}\u{c}\0").unwrap(),
            vec![entry("k", "\u{a0}café\u{2003}"), entry("v", "\u{b}\u{c}\0")]
        );
    }

    #[test]
    fn unquoted_values_keep_punctuation_and_allow_empty_text() {
        assert_eq!(
            parse("empty=\nblank= \t\nraw= #tag = \\\"literal\"\n9=last").unwrap(),
            vec![
                entry("empty", ""),
                entry("blank", ""),
                entry("raw", "#tag = \\\"literal\""),
                entry("9", "last"),
            ]
        );
    }

    #[test]
    fn quoted_values_decode_escapes_and_preserve_inner_whitespace() {
        assert_eq!(
            parse("text = \" \\t雪🦀\\n\\\"\\\\#= \"\t \nempty=\"\"").unwrap(),
            vec![entry("text", " \t雪🦀\n\"\\#= "), entry("empty", "")]
        );
    }

    #[test]
    fn only_a_cr_immediately_before_lf_is_a_line_ending() {
        assert_eq!(
            parse("a=one\r\nb=two\r\r\nc=three\rd=four\r").unwrap(),
            vec![
                entry("a", "one"),
                entry("b", "two\r"),
                entry("c", "three\rd=four\r")
            ]
        );
        assert_eq!(parse("x=\"a\rb\"\r\n").unwrap(), vec![entry("x", "a\rb")]);
        assert_eq!(
            parse("x=\"a\"\r").unwrap_err().kind(),
            ParseErrorKind::TrailingCharacters
        );
    }

    #[test]
    fn rejects_missing_equals_and_invalid_keys() {
        for (input, expected) in [
            ("key", ParseErrorKind::MissingEquals),
            ("=value", ParseErrorKind::InvalidKey),
            (" \t =value", ParseErrorKind::InvalidKey),
            ("two words=value", ParseErrorKind::InvalidKey),
            ("two\twords=value", ParseErrorKind::InvalidKey),
            ("é=value", ParseErrorKind::InvalidKey),
            ("a.b=value", ParseErrorKind::InvalidKey),
            ("\u{a0}key=value", ParseErrorKind::InvalidKey),
            ("\rkey=value", ParseErrorKind::InvalidKey),
        ] {
            let error = parse(input).unwrap_err();
            assert_eq!(error.kind(), expected, "input: {input:?}");
            assert_eq!(error.line(), 1);
        }
    }

    #[test]
    fn rejects_duplicates_after_trimming_keys() {
        let error = parse("key=one\r\n# comment\n\n\tkey \t=two").unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::DuplicateKey);
        assert_eq!(error.line(), 4);
        assert_eq!(error.to_string(), "line 4: duplicate key");
    }

    #[test]
    fn rejects_invalid_quoted_values() {
        for (value, expected) in [
            ("\"", ParseErrorKind::UnterminatedQuote),
            ("\"abc", ParseErrorKind::UnterminatedQuote),
            ("\"abc\\\"", ParseErrorKind::UnterminatedQuote),
            ("\"abc\\", ParseErrorKind::TrailingEscape),
            ("\"\\q\"", ParseErrorKind::UnknownEscape),
            ("\"\\r\"", ParseErrorKind::UnknownEscape),
            ("\"\\雪\"", ParseErrorKind::UnknownEscape),
            ("\"\\ \"", ParseErrorKind::UnknownEscape),
            ("\"abc\"tail", ParseErrorKind::TrailingCharacters),
            ("\"abc\" #comment", ParseErrorKind::TrailingCharacters),
            ("\"abc\"\"", ParseErrorKind::TrailingCharacters),
            ("\"abc\"\u{a0}", ParseErrorKind::TrailingCharacters),
            ("\"abc\nnext=ok", ParseErrorKind::UnterminatedQuote),
        ] {
            let input = format!("# comment\r\n\t\nkey={value}");
            let error = parse(&input).unwrap_err();
            assert_eq!(error.kind(), expected, "input: {input:?}");
            assert_eq!(error.line(), 3);
            assert!(error.to_string().starts_with("line 3: "), "error: {error}");
        }
    }

    #[test]
    fn error_implements_standard_error_without_exposing_values() {
        let error = parse("key=\"secret\"extra").unwrap_err();
        let standard_error: &dyn std::error::Error = &error;
        assert_eq!(
            standard_error.to_string(),
            "line 1: unexpected text after closing quote"
        );
        assert!(
            standard_error.source().is_none(),
            "syntax errors have no underlying cause"
        );
    }

    #[test]
    fn quoted_values_round_trip_arbitrary_short_character_sequences() {
        let alphabet = [
            'a', '雪', '🦀', '"', '\\', '\n', '\t', '\r', ' ', '#', '=', '\0',
        ];
        for first in alphabet {
            for second in alphabet {
                for third in alphabet {
                    let value: String = [first, second, third].into_iter().collect();
                    let encoded = value
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\t', "\\t");
                    let input = format!("key = \"{encoded}\" \t\r\n");
                    assert_eq!(parse(&input).unwrap(), vec![entry("key", &value)]);
                }
            }
        }
    }

    #[test]
    fn short_inputs_never_panic_and_error_lines_stay_in_bounds() {
        let alphabet = ['a', '=', '"', '\\', 'n', ' ', '\t', '\r', '\n', '#', '雪'];
        let mut inputs = vec![String::new()];
        for _ in 0..=4 {
            let mut next = Vec::new();
            for input in inputs {
                if let Err(error) = parse(&input) {
                    let line_count = input.bytes().filter(|&byte| byte == b'\n').count() + 1;
                    assert!((1..=line_count).contains(&error.line()), "input: {input:?}");
                }
                if input.chars().count() < 4 {
                    for ch in alphabet {
                        let mut extended = input.clone();
                        extended.push(ch);
                        next.push(extended);
                    }
                }
            }
            inputs = next;
        }
    }
}
