use small_config::{parse, Entry, ParseError};

fn entry(key: &str, value: &str) -> Entry {
    Entry {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

#[test]
fn ignores_empty_and_comment_lines() {
    for input in ["", "\n", " \t\n\t \r\n", "# comment", " \t# a = \"\\q\n\n"] {
        assert_eq!(parse(input).unwrap(), vec![], "input: {input:?}");
    }
}

#[test]
fn preserves_order_and_case_sensitive_keys() {
    assert_eq!(
        parse("z=last\nA=upper\na=lower\n0_-9=digits").unwrap(),
        vec![
            entry("z", "last"),
            entry("A", "upper"),
            entry("a", "lower"),
            entry("0_-9", "digits"),
        ]
    );
}

#[test]
fn trims_ascii_spaces_and_tabs_around_unquoted_values_and_keys() {
    assert_eq!(
        parse(" \t key \t = \t a b\tc \t\nempty= \t\nblank=").unwrap(),
        vec![
            entry("key", "a b\tc"),
            entry("empty", ""),
            entry("blank", "")
        ]
    );
}

#[test]
fn keeps_literal_characters_in_unquoted_values() {
    assert_eq!(
        parse("symbols = #hash = equals\ntext = café 🦀\nraw = a\"b\\q").unwrap(),
        vec![
            entry("symbols", "#hash = equals"),
            entry("text", "café 🦀"),
            entry("raw", "a\"b\\q"),
        ]
    );
}

#[test]
fn decodes_all_supported_escapes() {
    assert_eq!(
        parse(r#"key = "a\\b\"c\nd\te""#).unwrap(),
        vec![entry("key", "a\\b\"c\nd\te")]
    );
}

#[test]
fn quoted_values_preserve_spaces_unicode_and_literals() {
    assert_eq!(
        parse("a = \" \tcafé 🦀 # = \t \" \t\nb=\"\"").unwrap(),
        vec![entry("a", " \tcafé 🦀 # = \t "), entry("b", "")]
    );
}

#[test]
fn supports_lf_crlf_and_unterminated_final_lines() {
    for input in ["a=1\nb=2", "a=1\nb=2\n", "a=1\r\nb=2\r\n", "a=1\r\nb=2\n"] {
        assert_eq!(
            parse(input).unwrap(),
            vec![entry("a", "1"), entry("b", "2")]
        );
    }
}

#[test]
fn treats_lone_cr_as_content() {
    assert_eq!(parse("a=x\ry\r").unwrap(), vec![entry("a", "x\ry\r")]);
    assert_eq!(parse("a=\"x\ry\"\n").unwrap(), vec![entry("a", "x\ry")]);
    assert_eq!(parse("a=x\r\r\n").unwrap(), vec![entry("a", "x\r")]);
    assert!(parse("a=\"x\"\r").is_err());
    assert!(parse("\r").is_err());
}

#[test]
fn does_not_trim_other_unicode_or_ascii_whitespace() {
    assert_eq!(
        parse("a=\u{a0}x\u{a0}\nb=\u{b}x\u{c}").unwrap(),
        vec![entry("a", "\u{a0}x\u{a0}"), entry("b", "\u{b}x\u{c}")]
    );
    // Unicode line separators are content, not line endings.
    assert_eq!(
        parse("a=x\u{2028}y").unwrap(),
        vec![entry("a", "x\u{2028}y")]
    );
    for input in ["\u{a0}# comment", "\u{b}# comment", "a=\"x\"\u{a0}"] {
        assert!(parse(input).is_err(), "input: {input:?}");
    }
}

#[test]
fn rejects_missing_equals_and_invalid_keys() {
    for input in [
        "key",
        "=value",
        " \t=value",
        "two words=x",
        "a.b=x",
        "a/b=x",
        "a#=x",
        "é=x",
        "🦀=x",
        "a\u{a0}=x",
        "\u{b}a=x",
        "a\r=x",
        "\0=x",
    ] {
        let error = parse(input).unwrap_err();
        assert_eq!(error.line(), 1, "input: {input:?}");
    }
}

#[test]
fn rejects_malformed_quoted_values_without_panicking() {
    let cases = [
        ("a=\"", "missing closing quote"),
        ("a=\"text", "missing closing quote"),
        ("a=\"\\\"", "missing closing quote"),
        ("a=\"text\\", "trailing escape"),
        (r#"a="\q""#, "unknown escape"),
        (r#"a="\r""#, "unknown escape"),
        (r#"a="\u1234""#, "unknown escape"),
        (r#"a="\🦀""#, "unknown escape"),
        (r#"a="ok"x"#, "text after closing quote"),
        (r#"a="ok" # comment"#, "text after closing quote"),
        (r#"a="ok"="#, "text after closing quote"),
        (r#"a="ok"""#, "text after closing quote"),
        ("a=\"multi\nline\"", "missing closing quote"),
    ];
    for (input, reason) in cases {
        let error = parse(input).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("line 1: {reason}"),
            "input: {input:?}"
        );
    }
}

#[test]
fn rejects_duplicate_keys_after_trimming() {
    let error = parse("a=1\n# comment\n \ta \t=2").unwrap_err();
    assert_eq!(error.to_string(), "line 3: duplicate key");
}

#[test]
fn counts_all_physical_lines_in_errors() {
    let error = parse("\r\n \t# comment\r\ngood=1\n\ninvalid").unwrap_err();
    assert_eq!(error.line(), 5);
    assert_eq!(error.to_string(), "line 5: expected '='");
    let error = parse("a=1\nb=\"bad\\q\"\ninvalid").unwrap_err();
    assert_eq!(error.line(), 2);
}

#[test]
fn error_implements_standard_error_traits() {
    fn as_error(error: &ParseError) -> &dyn std::error::Error {
        error
    }
    let error = parse("bad").unwrap_err();
    assert_eq!(as_error(&error).to_string(), "line 1: expected '='");
    assert!(as_error(&error).source().is_none());
    assert!(!format!("{error:?}").is_empty());
}
