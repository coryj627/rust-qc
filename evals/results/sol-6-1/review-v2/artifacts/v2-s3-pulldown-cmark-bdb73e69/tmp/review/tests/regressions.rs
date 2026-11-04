use pulldown_cmark::{html, Event, Options, Parser, Tag};

fn render(input: &str, options: Options) -> String {
    let mut output = String::new();
    html::push_html(&mut output, Parser::new_ext(input, options));
    output
}

#[test]
fn crlf_is_one_space_inside_code_spans() {
    for input in ["`a\r\nb`", "` a\r\nb `"] {
        assert_eq!(render(input, Options::empty()), "<p><code>a b</code></p>\n");
    }
}

#[test]
fn crlf_code_spans_work_in_containers() {
    assert_eq!(
        render("> `a\r\n> b`", Options::empty()),
        "<blockquote>\n<p><code>a b</code></p>\n</blockquote>\n"
    );
}

#[test]
fn carriage_returns_delimit_fenced_code_lines() {
    assert_eq!(
        render("```rust\ra\rb\r```\r", Options::empty()),
        "<pre><code class=\"language-rust\">a\nb\n</code></pre>\n"
    );
}

#[test]
fn carriage_returns_delimit_indented_code_lines() {
    assert_eq!(
        render("    a\r    b\r", Options::empty()),
        "<pre><code>a\nb\n</code></pre>\n"
    );
}

#[test]
fn tabs_can_follow_closing_code_fences() {
    for fence in ["```", "~~~"] {
        let input = format!("{fence}\nx\n{fence}\t\nafter\n");
        assert_eq!(
            render(&input, Options::empty()),
            "<pre><code>x\n</code></pre>\n<p>after</p>\n"
        );
    }
}

#[test]
fn case_variant_footnotes_keep_matching_anchors_and_numbers() {
    let output = render("[^A]\n\n[^a]: note", Options::ENABLE_FOOTNOTES);
    assert!(output.contains("href=\"#a\">1</a>"), "{output}");
    assert!(
        output.contains("id=\"a\"><sup class=\"footnote-definition-label\">1</sup>"),
        "{output}"
    );
}

#[test]
fn empty_wikilink_captions_do_not_consume_following_text() {
    let output = render("[[p|]] tail", Options::ENABLE_WIKILINKS);
    assert_eq!(output.matches("tail").count(), 1, "{output}");
}

#[test]
fn wikilink_offsets_cover_both_closing_brackets() {
    for input in [
        "[[page]]",
        "[[page|caption]]",
        "![[page]]",
        "![[page|caption]]",
    ] {
        let span = Parser::new_ext(input, Options::ENABLE_WIKILINKS)
            .into_offset_iter()
            .find_map(|(event, span)| match event {
                Event::Start(Tag::Link { .. } | Tag::Image { .. }) => Some(span),
                _ => None,
            });
        assert_eq!(span, Some(0..input.len()), "{input}");
    }
}

#[test]
fn unicode_labels_count_characters() {
    for label in ["é".repeat(500), "界".repeat(334), "🦀".repeat(250)] {
        let input = format!("[x][{label}]\n\n[{label}]: /url\n");
        assert_eq!(
            render(&input, Options::empty()),
            "<p><a href=\"/url\">x</a></p>\n"
        );
    }
}

#[test]
fn ascii_labels_stop_at_999_characters() {
    let label = "a".repeat(1000);
    let input = format!("[x][{label}]\n\n[{label}]: /url\n");
    assert!(!Parser::new(&input).any(|event| matches!(event, Event::Start(Tag::Link { .. }))));
}

#[test]
fn single_letter_custom_heading_attributes_are_preserved() {
    assert_eq!(
        render("# heading {x}", Options::ENABLE_HEADING_ATTRIBUTES),
        "<h1 x=\"\">heading</h1>\n"
    );
}

#[test]
fn link_titles_require_a_separator_after_the_destination() {
    assert!(!Parser::new("[x](<url>\"title\")")
        .any(|event| matches!(event, Event::Start(Tag::Link { .. }))));
}

#[test]
fn tabs_do_not_count_as_two_trailing_spaces() {
    for input in ["a \t\nb", "a\t \nb"] {
        assert_eq!(render(input, Options::empty()), "<p>a\nb</p>\n");
    }
}

#[test]
fn null_text_uses_the_replacement_character() {
    assert_eq!(render("a\0b", Options::empty()), "<p>a�b</p>\n");
}

#[test]
fn image_alternative_text_omits_inline_html_tags() {
    assert_eq!(
        render("![a <b>b</b> c](img)", Options::empty()),
        "<p><img src=\"img\" alt=\"a b c\" /></p>\n"
    );
}

#[test]
fn ordinary_controls() {
    assert_eq!(
        render("`a\nb`", Options::empty()),
        "<p><code>a b</code></p>\n"
    );
    assert_eq!(render("a  \nb", Options::empty()), "<p>a<br />\nb</p>\n");
    assert_eq!(
        render("[[p|caption]] tail", Options::ENABLE_WIKILINKS),
        "<p><a href=\"p\">caption</a> tail</p>\n"
    );
}

#[test]
fn renderer_propagates_io_errors() {
    struct FailedWriter;
    impl std::io::Write for FailedWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("probe writer error"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(html::write_html_io(FailedWriter, Parser::new("text")).is_err());
}
