//! A small, dependency-free parser for line-based `key = value` configuration.
//!
//! Lines end with LF or CRLF; the final line may omit its ending. Blank lines
//! and lines beginning with `#` after optional ASCII spaces/tabs are ignored.
//! Keys contain only ASCII letters, digits, `_`, and `-`, and must be unique.
//! Spaces and tabs around keys and values are discarded. Unquoted values keep
//! all other characters literally, including `#` and `=`. Double-quoted values
//! support only `\\`, `\"`, `\n`, and `\t` escapes and preserve whitespace inside
//! the quotes. A closing quote may be followed only by spaces or tabs.
//!
//! ```
//! use small_config::{parse, Entry};
//!
//! let entries = parse("# settings\nname = Ada\ngreeting = \"Hello,\\t世界\"\n")?;
//! assert_eq!(entries, vec![
//!     Entry { key: "name".into(), value: "Ada".into() },
//!     Entry { key: "greeting".into(), value: "Hello,\t世界".into() },
//! ]);
//! # Ok::<(), small_config::ParseError>(())
//! ```

use std::collections::HashSet;
use std::fmt;

/// A key and its decoded value, in their original input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The case-sensitive ASCII key.
    pub key: String,
    /// The value after trimming or decoding quoted escapes.
    pub value: String,
}

/// A malformed configuration line.
///
/// Its display text includes the one-based line number and a concise reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    message: &'static str,
}

impl ParseError {
    /// Returns the one-based number of the line that could not be parsed.
    pub fn line(&self) -> usize {
        self.line
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

fn trim(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

/// Parses a configuration, preserving entry order and decoded values.
///
/// Empty values are allowed. Keys are case-sensitive, and repeated keys are
/// rejected. Only ASCII spaces and tabs count as surrounding whitespace.
/// A CR is removed only when immediately followed by an LF; a bare CR is
/// ordinary content (and therefore invalid in a key or after a closing quote).
///
/// # Errors
///
/// Returns a [`ParseError`] for the first malformed line or duplicate key.
/// Quoted values cannot span input lines. Unknown or unfinished escapes,
/// missing closing quotes, and text after a closing quote are errors.
pub fn parse(input: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    let mut keys = HashSet::new();

    for (index, raw_line) in input.split_inclusive('\n').enumerate() {
        let line = match raw_line.strip_suffix('\n') {
            Some(line) => line.strip_suffix('\r').unwrap_or(line),
            None => raw_line,
        };
        let line = trim(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let error = |message| ParseError {
            line: index + 1,
            message,
        };
        let (key, value) = line.split_once('=').ok_or_else(|| error("expected '='"))?;
        let key = trim(key);
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(error("invalid key"));
        }
        if !keys.insert(key) {
            return Err(error("duplicate key"));
        }

        let value = trim(value);
        let value = match value.strip_prefix('"') {
            Some(quoted) => decode_quoted(quoted).map_err(error)?,
            None => value.to_owned(),
        };
        entries.push(Entry {
            key: key.to_owned(),
            value,
        });
    }
    Ok(entries)
}

fn decode_quoted(input: &str) -> Result<String, &'static str> {
    let mut value = String::new();
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if !trim(chars.as_str()).is_empty() {
                    return Err("text after closing quote");
                }
                return Ok(value);
            }
            '\\' => value.push(match chars.next() {
                Some('\\') => '\\',
                Some('"') => '"',
                Some('n') => '\n',
                Some('t') => '\t',
                Some(_) => return Err("unknown escape"),
                None => return Err("unfinished escape"),
            }),
            _ => value.push(ch),
        }
    }
    Err("missing closing quote")
}
