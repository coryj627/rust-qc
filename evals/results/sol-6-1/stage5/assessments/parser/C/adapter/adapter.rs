use small_config::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    MissingEquals,
    InvalidKey,
    DuplicateKey,
    UnterminatedQuote,
    UnknownEscape,
    TrailingEscape,
    TrailingCharacters,
}

pub fn line(error: &ParseError) -> usize {
    error.line()
}

// This candidate has no public machine-readable reason/category API.
// Keep that limitation explicit; do not inspect Display/Debug or call context.
pub fn classify(_error: &ParseError) -> Option<Category> {
    None
}
