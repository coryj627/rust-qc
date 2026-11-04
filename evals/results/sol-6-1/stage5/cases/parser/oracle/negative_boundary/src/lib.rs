//! Independent reference for the documented small configuration format.
use std::collections::HashSet;
use std::fmt;

pub struct Entry {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct ParseError {
    line: usize,
    reason: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}
impl std::error::Error for ParseError {}

fn trim_layout(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

fn quoted_value(text: &str) -> Result<String, &'static str> {
    let mut decoded = String::new();
    let mut chars = text[1..].chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                return if trim_layout(chars.as_str()).is_empty() {
                    Ok(decoded)
                } else {
                    Err("text after closing quote")
                };
            }
            '\\' => match chars.next() {
                Some('\\') => decoded.push('\\'),
                Some('"') => decoded.push('"'),
                Some('n') => decoded.push('\n'),
                Some('t') => decoded.push('\t'),
                Some(_) => return Err("unknown escape"),
                None => return Err("trailing escape"),
            },
            _ => decoded.push(ch),
        }
    }
    Err("missing closing quote")
}

pub fn parse(input: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    // split_inclusive preserves whether a CR is actually followed by LF.
    for (offset, raw_line) in input.split_inclusive('\n').enumerate() {
        let line = if let Some(without_lf) = raw_line.strip_suffix('\n') {
            without_lf
        } else {
            raw_line
        };
        let line = trim_layout(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fail = |reason| ParseError {
            line: offset + 1,
            reason,
        };
        let (key, value) = line.split_once('=').ok_or_else(|| fail("missing equals"))?;
        let key = trim_layout(key);
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(fail("invalid key"));
        }
        let value = trim_layout(value);
        let value = if value.starts_with('"') {
            quoted_value(value).map_err(fail)?
        } else {
            value.to_owned()
        };
        if !seen.insert(key.to_owned()) {
            return Err(fail("duplicate key"));
        }
        entries.push(Entry {
            key: key.to_owned(),
            value,
        });
    }
    Ok(entries)
}
