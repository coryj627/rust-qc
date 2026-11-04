use small_config::{parse, Entry, ParseError};

fn pairs(input: &str) -> Vec<(String, String)> {
    parse(input)
        .unwrap()
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect()
}

#[test]
fn ignores_blank_lines_and_comments() {
    for input in [
        "",
        "\n",
        " \t\n\t \r\n",
        "# comment",
        " \t# x = \"bad\\q\n#\r\n",
    ] {
        assert!(parse(input).unwrap().is_empty(), "{input:?}");
    }
}

#[test]
fn trims_only_spaces_and_tabs_and_preserves_order() {
    assert_eq!(
        pairs(" \tZ-9_ \t= \tfirst \t\r\na= # literal = value\nA=\n0 = last"),
        [
            ("Z-9_", "first"),
            ("a", "# literal = value"),
            ("A", ""),
            ("0", "last")
        ]
        .map(|(key, value)| (key.into(), value.into()))
    );
    assert_eq!(
        pairs("x=\u{a0}text\u{a0}\ny=\u{b}text\u{c}\nz=an \"embedded\" quote"),
        [
            ("x", "\u{a0}text\u{a0}"),
            ("y", "\u{b}text\u{c}"),
            ("z", "an \"embedded\" quote")
        ]
        .map(|(key, value)| (key.into(), value.into()))
    );
}

#[test]
fn decodes_all_supported_escapes_and_unicode() {
    let input = "text = \"  café 世界 🦀 \\ \\\\ \\\" \\n \\t # =  \" \t";
    // A single backslash followed by a space is not an escape.
    assert!(parse(input).is_err());
    assert_eq!(
        pairs(r#"text = "  café 世界 🦀 \\ \" \n \t # =  " "#),
        vec![("text".into(), "  café 世界 🦀 \\ \" \n \t # =  ".into())]
    );
    assert_eq!(
        pairs("empty=\"\"\nblank=\" \t \""),
        vec![("empty".into(), "".into()), ("blank".into(), " \t ".into()),]
    );
}

#[test]
fn removes_cr_only_as_part_of_crlf() {
    assert_eq!(
        pairs("a=one\r\nb=two\r\r\nc=three\r"),
        [("a", "one"), ("b", "two\r"), ("c", "three\r")]
            .map(|(key, value)| (key.into(), value.into()))
    );
    assert_eq!(pairs("x=\"a\rb\"\r\n"), vec![("x".into(), "a\rb".into())]);
    assert!(parse("x=\"a\"\r").is_err());
    assert!(parse("\r").is_err());
}

#[test]
fn rejects_malformed_lines_with_their_physical_line_number() {
    let malformed = [
        "key",
        "=value",
        " \t=",
        "bad key=value",
        "kéy=value",
        "a.b=value",
        "+=value",
        "\u{a0}key=value",
        "key\r=value",
        "key=\"",
        "key=\"open",
        "key=\"trailing\\",
        r#"key="\q""#,
        r#"key="\r""#,
        r#"key="\u1234""#,
        r#"key="\0""#,
        r#"key="\🦀""#,
        r#"key="ok"text"#,
        r#"key="ok" # comment"#,
        "key=\"ok\"\u{a0}",
        "key=\"ok\"\u{b}",
        "key=\"ok\"=",
        "key=\"ok\"\"",
    ];
    for input in malformed {
        let input = format!("# first\r\n\n \t\n{input}");
        let error = parse(&input).unwrap_err();
        assert_eq!(error.line(), 4, "{input:?}");
        assert!(error.to_string().starts_with("line 4:"), "{error}");
    }
}

#[test]
fn rejects_duplicates_but_keys_are_case_sensitive() {
    assert!(parse("Key=1\nkey=2").is_ok());
    for input in ["key=1\nkey=2", " key \t=1\n# comment\n\n\tkey=2"] {
        let error = parse(input).unwrap_err();
        assert_eq!(error.line(), input.split('\n').count());
        assert!(error.to_string().contains("duplicate key"));
    }
}

#[test]
fn quoted_values_cannot_span_lines() {
    for input in [
        "x=\"first\nsecond\"",
        "x=\"first\\\nsecond\"",
        "x=\"first\r\nsecond\"",
    ] {
        assert_eq!(parse(input).unwrap_err().line(), 1);
    }
}

#[test]
fn error_implements_standard_error_and_entry_fields_are_public() {
    fn standard_error(error: &dyn std::error::Error) {
        assert!(!format!("{error:?}").is_empty());
        assert!(error.source().is_none());
    }
    let error: ParseError = parse("invalid").unwrap_err();
    standard_error(&error);
    let entry = Entry {
        key: "a".into(),
        value: "b".into(),
    };
    assert_eq!(parse("a=b").unwrap(), vec![entry]);
}
