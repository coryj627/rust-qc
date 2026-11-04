use error_adapter::{classify, line, Category};
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
