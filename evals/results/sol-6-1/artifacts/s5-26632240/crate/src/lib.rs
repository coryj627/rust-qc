//! Parses a small, line-oriented configuration format into ordered [`Entry`] values.
//!
//! Each data line contains `key = value`. Keys contain only ASCII letters,
//! digits, `_`, and `-`, and are case-sensitive. Spaces and tabs around keys
//! and values are ignored. Empty or space/tab-only lines and lines whose first
//! non-space/tab character is `#` are ignored. Within values, `#` and `=` are
//! literal characters.
//!
//! Values starting with `"` are quoted and support only `\\`, `\"`, `\n`, and
//! `\t` escapes. Other characters, including Unicode, are preserved. Only spaces
//! and tabs may follow a closing quote. Lines end with LF or CRLF; a final line
//! need not have an ending. A carriage return outside CRLF is ordinary text.
//!
//! [`parse`] rejects malformed lines and duplicate keys with a [`ParseError`].
//!
//! # Examples
//!
//! ```
//! let entries = small_config::parse("# preferences\nname = café\nmessage = \"hi\\nthere\"\n")?;
//! assert_eq!(entries[0].key, "name");
//! assert_eq!(entries[0].value, "café");
//! assert_eq!(entries[1].value, "hi\nthere");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]

use std::collections::HashSet;
use std::fmt;

/// A configuration key and its decoded value, as returned by [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The case-sensitive key, without surrounding spaces or tabs.
    pub key: String,
    /// The value after trimming or quoted escape decoding.
    pub value: String,
}

/// A malformed configuration line, with its location and failure kind.
///
/// # Examples
///
/// ```
/// use small_config::{parse, ParseErrorKind};
///
/// let error = parse("# comment\nmissing equals").expect_err("line 2 is malformed");
/// assert_eq!(error.line(), 2);
/// assert_eq!(error.kind(), ParseErrorKind::MissingEquals);
/// assert_eq!(error.to_string(), "line 2: missing '='");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    kind: ParseErrorKind,
}

impl ParseError {
    /// Returns the one-based physical line number of the malformed input.
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the failure kind for handling errors without parsing their messages.
    #[must_use]
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

/// The reason a configuration line could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// A data line has no equals sign separating its key and value.
    MissingEquals,
    /// A key is empty or contains characters outside ASCII letters, digits, `_`, and `-`.
    InvalidKey,
    /// A key has already appeared on an earlier data line.
    DuplicateKey,
    /// A quoted value has no closing quote on the same physical line.
    UnterminatedQuote,
    /// A quoted value contains an escape other than `\\`, `\"`, `\n`, or `\t`.
    UnknownEscape,
    /// A quoted value ends with a backslash instead of an escape character.
    TrailingEscape,
    /// A closing quote is followed by characters other than spaces or tabs.
    TrailingCharacters,
}

/// Parses configuration text, preserving entry order and decoding quoted values.
///
/// See the [crate documentation](crate) for the complete format.
///
/// # Examples
///
/// ```
/// use small_config::{parse, Entry};
///
/// assert_eq!(parse("count = 3\nempty =")?, vec![
///     Entry { key: "count".into(), value: "3".into() },
///     Entry { key: "empty".into(), value: "".into() },
/// ]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns a [`ParseError`] for the first malformed line or duplicate key.
/// The error identifies its one-based physical line number and failure kind.
pub fn parse(input: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    let mut keys = HashSet::new();

    for (index, line) in input.split_inclusive('\n').enumerate() {
        let line = match line.strip_suffix('\n') {
            Some(line) => line.strip_suffix('\r').unwrap_or(line),
            None => line,
        };
        let line = trim_space_tab(line);
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
        let key = trim_space_tab(key);
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

        let value = trim_space_tab(value);
        let value = match value.strip_prefix('"') {
            Some(quoted) => parse_quoted(quoted).map_err(error)?,
            None => value.to_owned(),
        };
        entries.push(Entry {
            key: key.to_owned(),
            value,
        });
    }
    Ok(entries)
}

fn trim_space_tab(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

fn parse_quoted(text: &str) -> Result<String, ParseErrorKind> {
    let mut value = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                return if trim_space_tab(chars.as_str()).is_empty() {
                    Ok(value)
                } else {
                    Err(ParseErrorKind::TrailingCharacters)
                };
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

    fn pairs(input: &str) -> Vec<(String, String)> {
        parse(input)
            .unwrap()
            .into_iter()
            .map(|entry| (entry.key, entry.value))
            .collect()
    }

    #[test]
    fn ignores_blank_lines_and_indented_comments() {
        for input in [
            "",
            "\n",
            " \t\n\t\r\n",
            "# comment",
            " \t# invalid = \"\\q\n\n",
        ] {
            assert!(parse(input).unwrap().is_empty(), "input: {input:?}");
        }
    }

    #[test]
    fn preserves_order_case_and_literal_unquoted_text() {
        assert_eq!(
            pairs(" \tZ-9_ = \t café 😀 # literal = text \t\r\nz-9_=second\n0 = 0"),
            [
                ("Z-9_".into(), "café 😀 # literal = text".into()),
                ("z-9_".into(), "second".into()),
                ("0".into(), "0".into()),
            ]
        );
        assert_eq!(pairs("x = a\"b\\n")[0].1, "a\"b\\n");
    }

    #[test]
    fn accepts_empty_quoted_and_unquoted_values() {
        assert_eq!(
            pairs("a=\nb= \t\r\nc=\"\" \t\nd=\" \t \""),
            [
                ("a".into(), "".into()),
                ("b".into(), "".into()),
                ("c".into(), "".into()),
                ("d".into(), " \t ".into()),
            ]
        );
    }

    #[test]
    fn decodes_only_quoted_escapes_and_preserves_unicode() {
        assert_eq!(
            pairs(r#"text = "雪😀 \\ \" \n \t # =" "#)[0].1,
            "雪😀 \\ \" \n \t # ="
        );
        assert_eq!(pairs(r"text = \n\t\q")[0].1, r"\n\t\q");
    }

    #[test]
    fn distinguishes_crlf_from_bare_carriage_returns() {
        assert_eq!(pairs("a=x\r\nb=y\n")[0].1, "x");
        assert_eq!(pairs("a=x\rb=y\r")[0].1, "x\rb=y\r");
        assert_eq!(pairs("a=x\r\r\n")[0].1, "x\r");
        assert_eq!(pairs("a=\"x\ry\"")[0].1, "x\ry");
        assert_eq!(
            parse("a=\"x\"\r").unwrap_err().kind(),
            ParseErrorKind::TrailingCharacters
        );
        assert_eq!(
            parse("\r").unwrap_err().kind(),
            ParseErrorKind::MissingEquals
        );
    }

    #[test]
    fn trims_only_ascii_spaces_and_tabs() {
        assert_eq!(
            pairs("x=\u{a0} hello \u{2003}")[0].1,
            "\u{a0} hello \u{2003}"
        );
        assert_eq!(pairs("x=\u{b}\u{c}\0")[0].1, "\u{b}\u{c}\0");
        for input in [
            "\u{a0}x=v",
            "\u{b}x=v",
            "\u{2003}# comment",
            "x=\"v\"\u{a0}",
        ] {
            assert!(parse(input).is_err(), "input: {input:?}");
        }
    }

    #[test]
    fn rejects_invalid_keys_and_missing_separators() {
        for input in [
            "=value",
            " \t =value",
            "two words=x",
            "a.b=x",
            "é=x",
            "a\0=x",
            "a#=x",
        ] {
            assert_eq!(
                parse(input).unwrap_err().kind(),
                ParseErrorKind::InvalidKey,
                "input: {input:?}"
            );
        }
        for input in ["no separator", "a", "\"a=b\""] {
            assert!(parse(input).is_err(), "input: {input:?}");
        }
    }

    #[test]
    fn rejects_malformed_quoted_values() {
        for (input, kind) in [
            ("x=\"", ParseErrorKind::UnterminatedQuote),
            ("x=\"abc", ParseErrorKind::UnterminatedQuote),
            ("x=\"abc\\\"", ParseErrorKind::UnterminatedQuote),
            ("x=\"abc\\", ParseErrorKind::TrailingEscape),
            ("x=\"\\q\"", ParseErrorKind::UnknownEscape),
            ("x=\"\\r\"", ParseErrorKind::UnknownEscape),
            ("x=\"\\雪\"", ParseErrorKind::UnknownEscape),
            ("x=\"ok\"# comment", ParseErrorKind::TrailingCharacters),
            ("x=\"ok\" \t =", ParseErrorKind::TrailingCharacters),
            ("x=\"ok\"\"", ParseErrorKind::TrailingCharacters),
            ("x=\"first\nsecond\"", ParseErrorKind::UnterminatedQuote),
        ] {
            let error = parse(input).unwrap_err();
            assert_eq!(error.kind(), kind, "input: {input:?}");
            assert_eq!(error.line(), 1, "input: {input:?}");
        }
    }

    #[test]
    fn rejects_duplicate_keys_after_trimming() {
        let error = parse("x = first\n# comment\n\r\n \tx\t=second").unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::DuplicateKey);
        assert_eq!(error.line(), 4);
        assert_eq!(pairs("x=1\nX=2").len(), 2);
    }

    #[test]
    fn reports_physical_line_numbers_and_implements_error() {
        let error = parse("\r\n# comment\r\nokay=yes\nbad").unwrap_err();
        assert_eq!(error.line(), 4);
        assert_eq!(error.kind(), ParseErrorKind::MissingEquals);
        assert_eq!(error.to_string(), "line 4: missing '='");
        let error: &dyn std::error::Error = &error;
        assert!(
            error.source().is_none(),
            "syntax errors have no underlying error"
        );
        assert!(
            !format!("{error:?}").is_empty(),
            "errors must support Debug"
        );
    }

    #[test]
    fn quoted_values_round_trip_generated_unicode_and_control_characters() {
        let alphabet = [
            'a', '雪', '😀', '\\', '"', '\n', '\t', '\r', ' ', '#', '=', '\0',
        ];
        for number in 0..alphabet.len().pow(3) {
            let mut remaining = number;
            let mut value = String::new();
            for _ in 0..3 {
                value.push(alphabet[remaining % alphabet.len()]);
                remaining /= alphabet.len();
            }
            let encoded = value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t");
            let input = format!("key = \"{encoded}\" \t\r\n");
            assert_eq!(pairs(&input), [("key".into(), value)], "input: {input:?}");
        }
    }

    #[test]
    fn arbitrary_short_inputs_never_panic() {
        let alphabet = [
            'a', '0', '-', ' ', '\t', '=', '"', '\\', '#', '\n', '\r', 'é', '😀', '\0',
        ];
        for length in 0..=4 {
            for number in 0..alphabet.len().pow(length) {
                let mut remaining = number;
                let mut input = String::new();
                for _ in 0..length {
                    input.push(alphabet[remaining % alphabet.len()]);
                    remaining /= alphabet.len();
                }
                // Both acceptance and rejection are valid; a panic fails this test.
                drop(parse(&input));
            }
        }
    }
}
