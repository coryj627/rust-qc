from pathlib import Path
import shutil
root = Path('/work/results')
for letter in 'ABCD':
    d = root / letter
    d.mkdir(exist_ok=True)
    (d / 'frozen').mkdir(exist_ok=True)
    (d / 'gate').mkdir(exist_ok=True)
    (d / 'own').mkdir(exist_ok=True)
    (d / 'adapter').mkdir(exist_ok=True)
    shutil.copyfile('/work/tmp/setup_assessment.py', d / 'setup_assessment.py')
    available = letter in 'BD'
    code = '''use small_config::ParseError;

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

'''
    if available:
        code += '''// Reviewed against this candidate's public ParseError::kind() and
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
'''
    else:
        code += '''// This candidate has no public machine-readable reason/category API.
// Keep that limitation explicit; do not inspect Display/Debug or call context.
pub fn classify(_error: &ParseError) -> Option<Category> {
    None
}
'''
    (d / 'adapter/adapter.rs').write_text(code)
    test = r'''use error_adapter::{classify, line, Category};
use small_config::parse;

fn check(input: &str, category: Category, expected_line: usize) {
    let error = parse(input).expect_err("test fixture must be malformed");
    assert_eq!(classify(&error), Some(category));
    assert_eq!(line(&error), expected_line);
}

#[test]
fn missing_equals() { check("# heading\nmissing", Category::MissingEquals, 2); }
#[test]
fn invalid_key() { check("# heading\ninvalid.key=value", Category::InvalidKey, 2); }
#[test]
fn duplicate_key() { check("a=1\n# heading\na=2", Category::DuplicateKey, 3); }
#[test]
fn unterminated_quote() { check("# heading\na=\"open", Category::UnterminatedQuote, 2); }
#[test]
fn unknown_escape() { check("# heading\na=\"\\q\"", Category::UnknownEscape, 2); }
#[test]
fn trailing_escape() { check("# heading\na=\"open\\", Category::TrailingEscape, 2); }
#[test]
fn trailing_characters() { check("# heading\na=\"closed\"text", Category::TrailingCharacters, 2); }
'''
    (d / 'adapter/tests.rs').write_text(test)
    (d / 'adapter/Cargo.toml').write_text(f'''[package]
name = "error_adapter"
version = "0.0.0"
edition = "2021"

[lib]
path = "adapter.rs"

[[test]]
name = "categories"
path = "tests.rs"

[dependencies]
small_config = {{ path = "/work/candidates/{letter}" }}

[workspace]
''')
    notes = f'''Candidate {letter} adapter review

Reviewed /work/candidates/{letter}/src/lib.rs independently.
The frozen scoring.md contains no category adapter runner or category contract.
The frozen 24 public API tests are run unchanged regardless of adapter availability.
The preserved seven category tests are supplemental checks of optional public APIs;
they do not add requirements to the case and are excluded from frozen hidden counts.

'''
    if available:
        notes += '''Public ParseError::kind() returns the public non_exhaustive ParseErrorKind.
Each mapping is an exact match on that typed API:
- MissingEquals: documented missing separator.
- InvalidKey: documented empty or invalid key alphabet.
- DuplicateKey: documented repeated key.
- UnterminatedQuote: documented missing closing quote.
- UnknownEscape: documented unsupported escape.
- TrailingEscape: documented final backslash without an escape character.
- TrailingCharacters: documented non-space/tab following the closing quote.
The adapter returns None for future unknown enum variants. The line mapping calls
public ParseError::line(), documented as one-based physical input line.
Classification accepts only an error: it cannot inspect call input or expected test
category, and does not read Display or Debug strings or private representation.
No submission files are injected or changed; the adapter is a separate consumer.
'''
    else:
        notes += '''Public ParseError::line() supplies one-based lines, but the reason/message
field is private and no public typed category accessor or downcastable source is
provided. The adapter records None for category. Reading Display/Debug text or
inferring category from the caller's input would violate the assessment rules.
All seven category adapter tests are deliberately UNRUN (not compile/test failures).
The missing optional category API is not a failure of the requested case API.
'''
    (d / 'adapter/review.md').write_text(notes)
