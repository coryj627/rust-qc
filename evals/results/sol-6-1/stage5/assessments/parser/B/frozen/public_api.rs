use small_config::{parse, Entry, ParseError};
use std::error::Error;

fn values(input: &str) -> Vec<(String, String)> {
    match parse(input) {
        Ok(entries) => entries.into_iter().map(|e| (e.key, e.value)).collect(),
        Err(error) => panic!("expected valid configuration, got {error:?}"),
    }
}

fn expected(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn rejects(input: &str) {
    assert!(parse(input).is_err(), "accepted malformed input: {input:?}");
}

fn error_on_line(input: &str, line: usize) {
    let error = match parse(input) {
        Ok(_) => panic!("accepted malformed input: {input:?}"),
        Err(error) => error,
    };
    let message = error.to_string();
    // Accept any message wording, punctuation, field order or padding. Match a
    // complete decimal number so that line 1 cannot accidentally match line 10.
    assert!(
        message
            .split(|c: char| !c.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .any(|part| part.parse::<usize>() == Ok(line)),
        "display should identify line {line}; got {message:?}"
    );
}

#[test]
fn public_signature_fields_and_error_traits() {
    let parser: fn(&str) -> Result<Vec<Entry>, ParseError> = parse;
    let entry = Entry {
        key: String::from("manual"),
        value: String::from("value"),
    };
    assert_eq!(entry.key, "manual");
    assert_eq!(entry.value, "value");
    fn accepts_error<E: Error + std::fmt::Debug + std::fmt::Display>() {}
    accepts_error::<ParseError>();
    assert!(parser("").is_ok());
}

#[test]
fn empty_input_empty_lines_and_comments() {
    assert!(values("").is_empty());
    assert!(values("\n\n\r\n").is_empty());
    assert!(values("# header\n \t# invalid = \"unterminated\r\n\t# trailing").is_empty());
}

#[test]
fn ascii_layout_and_key_alphabet() {
    assert_eq!(
        values(" \t aZ09_- \t= \t useful text \t\n0=zero\n_=under\n-=dash"),
        expected(&[
            ("aZ09_-", "useful text"),
            ("0", "zero"),
            ("_", "under"),
            ("-", "dash")
        ])
    );
}

#[test]
fn unquoted_empty_values_and_literal_separators() {
    assert_eq!(
        values("a=\nb = \t\nc=hello # world = again\nd==\ne=#\nf=two  internal\t gaps"),
        expected(&[
            ("a", ""),
            ("b", ""),
            ("c", "hello # world = again"),
            ("d", "="),
            ("e", "#"),
            ("f", "two  internal\t gaps")
        ])
    );
}

#[test]
fn quote_only_has_special_meaning_at_value_start() {
    assert_eq!(
        values("a=word\"tail\"\nb=path\\n\\q\\\nc='single'"),
        expected(&[
            ("a", "word\"tail\""),
            ("b", "path\\n\\q\\"),
            ("c", "'single'")
        ])
    );
}

#[test]
fn quoted_empty_whitespace_and_literals() {
    assert_eq!(
        values("a=\"\"\nb=\" \t = # \" \t\nc=\"'single'\""),
        expected(&[("a", ""), ("b", " \t = # "), ("c", "'single'")])
    );
}

#[test]
fn all_four_supported_escapes() {
    assert_eq!(
        values(r#"a="backslash:\\ quote:\" newline:\n tab:\t""#),
        expected(&[("a", "backslash:\\ quote:\" newline:\n tab:\t")])
    );
}

#[test]
fn escape_sequences_are_decoded_once() {
    assert_eq!(
        values(r#"a="\\n\\t\\\"""#),
        expected(&[("a", "\\n\\t\\\"")])
    );
}

#[test]
fn escaped_quote_does_not_end_value() {
    assert_eq!(values(r#"a="x\"= # y""#), expected(&[("a", "x\"= # y")]));
}

#[test]
fn preserve_order_and_case_sensitive_keys() {
    assert_eq!(
        values("z=first\nA=second\na=third\nZ=fourth"),
        expected(&[
            ("z", "first"),
            ("A", "second"),
            ("a", "third"),
            ("Z", "fourth")
        ])
    );
}

#[test]
fn duplicate_keys_are_rejected_after_layout_normalization() {
    for input in [
        "a=1\na=2",
        "a=1\n \t a \t=1",
        "x=1\n# c\ny=2\nx=3",
        "a=\na=\"\"",
    ] {
        rejects(input);
    }
}

#[test]
fn reject_missing_equals_and_empty_key() {
    for input in ["a", "=x", " \t= value", "\"a\"=b"] {
        rejects(input);
    }
}

#[test]
fn reject_non_ascii_or_punctuated_keys() {
    for key in [
        "has space",
        "has\ttab",
        "a.b",
        "a/b",
        "a#b",
        "é",
        "键",
        "🙂",
        "a\rb",
        "a\u{a0}",
        "\u{b}a",
    ] {
        rejects(&format!("{key}=value"));
    }
}

#[test]
fn reject_unterminated_quotes_and_escapes() {
    for input in [
        "a=\"",
        "a=\"abc",
        "a=\"abc\\",
        "a=\"abc\\\"",
        "a=\"x\nnext=value",
    ] {
        rejects(input);
    }
}

#[test]
fn reject_unknown_quoted_escapes() {
    for suffix in ["r", "0", "x41", "u{41}", "q", "/", " ", "\t", "é", "🙂"] {
        rejects(&format!("a=\"\\{suffix}\""));
    }
}

#[test]
fn reject_text_after_closing_quote_including_comment() {
    for suffix in [
        "x",
        "=",
        "# comment",
        " \t # comment",
        "\"",
        "\\n",
        "\u{a0}",
        "\u{b}",
    ] {
        rejects(&format!("a=\"ok\"{suffix}"));
    }
}

#[test]
fn lf_crlf_mixed_endings_and_unterminated_final_line() {
    assert_eq!(
        values("a=one\r\nb=two\nc=\"three\"\r\nd=four"),
        expected(&[("a", "one"), ("b", "two"), ("c", "three"), ("d", "four")])
    );
    assert_eq!(values("a=one\n"), expected(&[("a", "one")]));
    assert_eq!(values("a=one\r\n"), expected(&[("a", "one")]));
}

#[test]
fn cr_is_only_a_line_ending_when_immediately_before_lf() {
    assert_eq!(values("a=x\r"), expected(&[("a", "x\r")]));
    assert_eq!(values("a=x\r\r\n"), expected(&[("a", "x\r")]));
    assert_eq!(values("a=x\ry"), expected(&[("a", "x\ry")]));
    assert_eq!(values("a=\"x\ry\""), expected(&[("a", "x\ry")]));
    rejects("a=\"x\"\r");
}

#[test]
fn unicode_values_are_preserved() {
    assert_eq!(
        values("a= café 世界 🙂 e\u{301} \nb=\"é\\n世界\\t🙂\\\"e\u{301}\""),
        expected(&[
            ("a", "café 世界 🙂 e\u{301}"),
            ("b", "é\n世界\t🙂\"e\u{301}")
        ])
    );
}

#[test]
fn trim_only_ascii_space_and_tab() {
    assert_eq!(
        values("a= \t\u{a0}value\u{2003}\t \nb=\u{b}v\u{c}\nc=\u{a0}\"literal\""),
        expected(&[
            ("a", "\u{a0}value\u{2003}"),
            ("b", "\u{b}v\u{c}"),
            ("c", "\u{a0}\"literal\"")
        ])
    );
    rejects("\u{a0}# not an ASCII-indented comment");
}

#[test]
fn errors_report_physical_one_based_line_numbers() {
    error_on_line("bad", 1);
    error_on_line("# heading\n\n\t# comment\r\na=ok\r\nbad", 5);
    error_on_line("\n\n\n\n\n\n\n\n\nmissing", 10);
    error_on_line("a=1\n# c\na=2", 3);
    error_on_line("ok=1\nq=\"\\q\"", 2);
    error_on_line("# c\na=\"ok\"tail", 2);
}

#[test]
fn repeated_calls_have_independent_state() {
    for _ in 0..3 {
        assert_eq!(values("a=x"), expected(&[("a", "x")]));
        rejects("a=x\na=y");
        assert_eq!(values("a=y"), expected(&[("a", "y")]));
    }
}

#[test]
fn supported_escape_and_unicode_combinations() {
    let fragments = [
        ("x", "x"),
        ("é", "é"),
        ("世界", "世界"),
        ("🙂", "🙂"),
        (r"\n", "\n"),
        (r"\t", "\t"),
        (r"\\", "\\"),
        (r#"\""#, "\""),
    ];
    for (left_source, left_value) in fragments {
        for (right_source, right_value) in fragments {
            let input = format!("k=\"{left_source}{right_source}\"\t");
            let want = format!("{left_value}{right_value}");
            assert_eq!(
                values(&input),
                vec![(String::from("k"), want)],
                "input {input:?}"
            );
        }
    }
}

#[test]
fn malformed_utf8_safe_inputs_never_panic() {
    // All strings are valid UTF-8. No byte offsets or parser internals are assumed.
    let fragments = [
        "", "a", "=", "\"", "\\", "\n", "\r", "\t", "#", "é", "🙂", "\0",
    ];
    for first in fragments {
        for second in fragments {
            for third in fragments {
                let input = format!("{first}{second}{third}");
                assert!(
                    std::panic::catch_unwind(|| {
                        let _ = parse(&input);
                    })
                    .is_ok(),
                    "parser panicked on {input:?}"
                );
            }
        }
    }
}
