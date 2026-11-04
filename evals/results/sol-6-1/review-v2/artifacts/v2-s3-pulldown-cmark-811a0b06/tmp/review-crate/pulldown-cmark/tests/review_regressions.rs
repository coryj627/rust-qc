use pulldown_cmark::{html, Event, Options, Parser, Tag};
fn render(s: &str, opts: Options) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(s, opts));
    out
}
#[test]
fn code_span_crlf_is_one_space() {
    assert_eq!(render("`a\r\nb`", Options::empty()), "<p><code>a b</code></p>\n");
}
#[test]
fn cr_line_endings_equivalent_to_lf() {
    for source in ["a\nb", "# a\n\nb", "```rust\nx\n```", "    x\n    y\n", "> x\n> y", "<div>\nx\n\ny"] {
        assert_eq!(render(&source.replace('\n', "\r"), Options::empty()), render(source, Options::empty()), "input={source:?}");
    }
}
#[test]
fn unicode_label_within_999_characters_resolves() {
    let label = "é".repeat(500);
    let input = format!("[text][{label}]\n\n[{label}]: /url\n");
    assert!(Parser::new(&input).any(|e| matches!(e, Event::Start(Tag::Link {..}))));
}
#[test]
fn ascii_label_above_999_characters_does_not_resolve() {
    let label = "a".repeat(1000);
    let input = format!("[text][{label}]\n\n[{label}]: /url\n");
    assert!(!Parser::new(&input).any(|e| matches!(e, Event::Start(Tag::Link {..}))));
}
#[test]
fn footnote_case_variants_share_anchor_and_number() {
    let output = render("a[^NOTE] b[^note]\n\n[^Note]: text\n", Options::ENABLE_FOOTNOTES);
    println!("{output}");
    assert_eq!(output.matches("href=\"#Note\"").count(), 2);
    assert_eq!(output.matches(">1</a>").count(), 2);
}
#[test]
fn wikilink_offset_contains_both_closing_brackets() {
    let input = "[[page]]";
    let (_, span) = Parser::new_ext(input, Options::ENABLE_WIKILINKS).into_offset_iter()
        .find(|(e, _)| matches!(e, Event::Start(Tag::Link {..}))).unwrap();
    assert_eq!(&input[span], input);
}
#[test]
fn heading_closer_accepts_tabs() {
    assert_eq!(render("# foo\t###\t\n", Options::empty()), "<h1>foo</h1>\n");
}
#[test]
fn closing_fence_accepts_tabs() {
    assert_eq!(render("```\nx\n```\t\nafter", Options::empty()), "<pre><code>x\n</code></pre>\n<p>after</p>\n");
}
#[test]
fn link_title_requires_whitespace_after_destination() {
    assert!(!Parser::new("[a](<url>\"title\")").any(|e| matches!(e, Event::Start(Tag::Link {..}))));
}
#[test]
fn html_block_close_is_case_insensitive() {
    assert_eq!(render("<script>\nx\n</SCRIPT>\n\nafter", Options::empty()), "<script>\nx\n</SCRIPT>\n<p>after</p>\n");
}
#[test]
fn nested_superscript_does_not_hide_table_context() {
    let source = "a | b\n---|---\n^`x\\|y`^ | z\n";
    assert!(render(source, Options::ENABLE_TABLES | Options::ENABLE_SUPERSCRIPT).contains("<code>x|y</code>"));
}
#[test]
fn tilde_closer_does_not_hide_superscript_opener() {
    assert_eq!(render("^a b~ c^", Options::ENABLE_SUPERSCRIPT | Options::ENABLE_SUBSCRIPT), "<p><sup>a b~ c</sup></p>\n");
}
#[test]
fn controls() {
    for label in ["a".repeat(999), "é".repeat(499)] {
        let input = format!("[text][{label}]\n\n[{label}]: /url\n");
        assert_eq!(render(&input, Options::empty()), "<p><a href=\"/url\">text</a></p>\n");
    }
    assert_eq!(render("`a\nb`", Options::empty()), "<p><code>a b</code></p>\n");
    assert_eq!(render("[a](<url> \"title\")", Options::empty()), "<p><a href=\"url\" title=\"title\">a</a></p>\n");
    assert_eq!(render("# foo ###\n", Options::empty()), "<h1>foo</h1>\n");
    assert_eq!(render("<script>\nx\n</script>\n\nafter", Options::empty()), "<script>\nx\n</script>\n<p>after</p>\n");
    assert_eq!(render("^a b~ c^", Options::ENABLE_SUPERSCRIPT), "<p><sup>a b~ c</sup></p>\n");
}
#[test]
fn html_block_close_need_not_match_start_tag() {
    assert_eq!(render("<pre>\nx\n</style>\n\nafter", Options::empty()), "<pre>\nx\n</style>\n<p>after</p>\n");
}
#[test]
fn one_character_heading_attribute_is_preserved() {
    let event = Parser::new_ext("# hi {x}\n", Options::ENABLE_HEADING_ATTRIBUTES).next().unwrap();
    assert!(matches!(event, Event::Start(Tag::Heading {attrs, ..}) if attrs == [("x".into(), None)]));
}
