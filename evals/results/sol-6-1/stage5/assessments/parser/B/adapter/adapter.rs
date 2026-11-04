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

// Reviewed against this candidate's public ParseError::kind() and
// documented ParseErrorKind variants. No message or parser input is inspected.
pub fn classify(error: &ParseError) -> Option<Category> {
    use small_config::ParseErrorKind;
    match error.kind() {
        ParseErrorKind::MissingEquals => Some(Category::MissingEquals),
        ParseErrorKind::InvalidKey => Some(Category::InvalidKey),
        ParseErrorKind::DuplicateKey => Some(Category::DuplicateKey),
        ParseErrorKind::UnterminatedQuote => Some(Category::UnterminatedQuote),
        ParseErrorKind::UnknownEscape => Some(Category::UnknownEscape),
        ParseErrorKind::TrailingEscape => Some(Category::TrailingEscape),
        ParseErrorKind::TrailingCharacters => Some(Category::TrailingCharacters),
        // The candidate enum is non_exhaustive. Future unknown variants must
        // remain unclassified instead of receiving an invented category.
        _ => None,
    }
}
