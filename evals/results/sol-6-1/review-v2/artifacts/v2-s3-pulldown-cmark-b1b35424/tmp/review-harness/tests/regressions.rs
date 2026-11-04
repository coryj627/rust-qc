use pulldown_cmark::{html, Event, Options, Parser, Tag};

fn render(input: &str, options: Options) -> String {
    let mut output = String::new();
    html::push_html(&mut output, Parser::new_ext(input, options));
    output
}

#[test]
fn code_span_line_endings_are_single_spaces() {
    for ending in ["\n", "\r", "\r\n"] {
        let input = format!("`a{ending}b`");
        assert_eq!(
            render(&input, Options::empty()),
            "<p><code>a b</code></p>\n",
            "{input:?}"
        );
    }
}

#[test]
fn fenced_language_is_first_whitespace_separated_word() {
    for separator in [" ", "\t"] {
        let input = format!("```rust{separator}custom\ncode\n```");
        assert_eq!(
            render(&input, Options::empty()),
            "<pre><code class=\"language-rust\">code\n</code></pre>\n",
            "{input:?}"
        );
    }
}

#[test]
fn wikilink_offsets_cover_both_closing_brackets() {
    for input in [
        "[[Wiki]]",
        "[[Wiki|label]]",
        "![[dog.png]]",
        "![[dog.png|a dog]]",
    ] {
        for (event, range) in Parser::new_ext(input, Options::ENABLE_WIKILINKS).into_offset_iter() {
            if matches!(event, Event::Start(Tag::Link { .. } | Tag::Image { .. })) {
                assert_eq!(range, 0..input.len(), "{input:?}");
            }
        }
    }
}

#[test]
fn empty_wikilink_alias_does_not_render_delimiters() {
    let output = render("[[Wiki|]]", Options::ENABLE_WIKILINKS);
    assert!(!output.contains(">]]</a>"), "{output:?}");
}

#[test]
fn footnote_reference_case_variants_share_destination_and_number() {
    let output = render(
        "ref [^FOO] [^foo]\n\n[^foo]: Note",
        Options::ENABLE_FOOTNOTES,
    );
    assert_eq!(
        output.matches("href=\"#foo\">1</a>").count(),
        2,
        "{output:?}"
    );
}

#[test]
fn unicode_label_length_counts_characters() {
    for (unit, count) in [("é", 500), ("🦀", 250), ("a", 999)] {
        let label = unit.repeat(count);
        let input = format!("[text][{label}]\n\n[{label}]: /url\n");
        assert_eq!(
            render(&input, Options::empty()),
            "<p><a href=\"/url\">text</a></p>\n",
            "unit={unit:?}, count={count}"
        );
    }
}

#[test]
fn ascii_label_over_limit_is_literal() {
    let label = "a".repeat(1000);
    let input = format!("[text][{label}]\n\n[{label}]: /url\n");
    assert!(!render(&input, Options::empty()).contains("href="));
}

#[test]
fn baseline_feature_combinations_are_balanced() {
    let fixtures = [
        "Hello *world*",
        "# hello {#id .class}",
        "a|b\n---|---\nx|y",
        "- [x] done",
        "> [!NOTE]\n> Note",
        "term\n: definition",
        "---\ntitle: test\n---",
        "$x$ and $$y$$",
        "~sub~ ^super^ ~~strike~~",
        "ref [^x]\n\n[^x]: note",
        "[[Wiki|a *label*]]",
    ];
    for options in [
        Options::empty(),
        Options::all(),
        Options::all() - Options::ENABLE_OLD_FOOTNOTES,
    ] {
        for input in fixtures {
            let mut stack = Vec::new();
            for (event, range) in Parser::new_ext(input, options).into_offset_iter() {
                assert!(range.start <= range.end && range.end <= input.len());
                assert!(input.is_char_boundary(range.start) && input.is_char_boundary(range.end));
                match event {
                    Event::Start(tag) => stack.push(tag.to_end()),
                    Event::End(end) => assert_eq!(stack.pop(), Some(end)),
                    _ => {}
                }
            }
            assert!(stack.is_empty());
        }
    }
}

#[test]
fn one_character_heading_attributes_are_retained() {
    for input in ["# heading {x}", "heading {x}\n==="] {
        assert_eq!(
            render(input, Options::ENABLE_HEADING_ATTRIBUTES),
            "<h1 x=\"\">heading</h1>\n"
        );
    }
}

#[test]
fn standalone_carriage_returns_terminate_block_lines() {
    assert_eq!(
        render("# heading\r\rpara\r", Options::ENABLE_HEADING_ATTRIBUTES),
        "<h1>heading</h1>\n<p>para</p>\n"
    );
}

#[test]
fn closing_fence_allows_a_trailing_tab() {
    assert_eq!(
        render("```\ncode\n```\t\nafter\n", Options::empty()),
        "<pre><code>code\n</code></pre>\n<p>after</p>\n"
    );
}

#[test]
fn literal_null_is_replaced_as_required_by_commonmark() {
    assert_eq!(render("a\0b", Options::empty()), "<p>a\u{fffd}b</p>\n");
}
