use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
fn render(text: &str, opts: Options) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(text, opts));
    out
}
fn links(text: &str) -> usize {
    Parser::new(text)
        .filter(|e| matches!(e, Event::Start(Tag::Link { .. })))
        .count()
}
#[test]
fn all_defined_references_remain_links() {
    let text = format!(
        "[r]: https://example.com/{}\n\n{}",
        "a".repeat(180),
        "[r] ".repeat(600)
    );
    assert_eq!(links(&text), 600);
}
#[test]
fn footnote_case_variants_reach_the_definition() {
    let out = render("a[^A] b[^a]\n\n[^A]: note\n", Options::ENABLE_FOOTNOTES);
    assert!(!out.contains("href=\"#a\""), "{out}");
    assert_eq!(out.matches(">1</a>").count(), 2, "{out}");
}
#[test]
fn wiki_ranges_cover_both_closing_brackets() {
    for text in ["[[Page]]", "[[Page|alias]]", "![[a.png]]", "![[a.png|alt]]"] {
        for (ev, r) in Parser::new_ext(text, Options::ENABLE_WIKILINKS).into_offset_iter() {
            if matches!(
                ev,
                Event::Start(Tag::Link { .. } | Tag::Image { .. })
                    | Event::End(TagEnd::Link | TagEnd::Image)
            ) {
                assert_eq!(r, 0..text.len(), "{text}: {ev:?}");
            }
        }
    }
}
#[test]
fn empty_wiki_alias_has_no_delimiter_text() {
    assert_eq!(
        render("[[Page|]]", Options::ENABLE_WIKILINKS),
        "<p><a href=\"Page\"></a></p>\n"
    );
}
#[test]
fn unicode_label_limit_counts_characters() {
    let label = "é".repeat(500);
    let text = format!("[{label}]: /ok\n\n[x][{label}]\n");
    assert_eq!(links(&text), 1);
}
#[test]
fn ascii_label_limit_is_999() {
    for n in [998, 999, 1000, 1001] {
        let label = "a".repeat(n);
        let text = format!("[{label}]: /ok\n\n[x][{label}]\n");
        assert_eq!(links(&text), usize::from(n <= 999), "label length {n}");
    }
}
#[test]
fn null_character_is_replaced() {
    assert_eq!(render("a\0b", Options::empty()), "<p>a\u{fffd}b</p>\n");
}
#[test]
fn code_cr_line_endings_normalized() {
    assert_eq!(
        render("```\ra\rb\r```", Options::empty()),
        "<pre><code>a\nb\n</code></pre>\n"
    );
}
#[test]
fn io_failure_is_returned() {
    struct Failure;
    impl std::io::Write for Failure {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("write failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(html::write_html_io(Failure, Parser::new("hello")).is_err());
}
#[test]
fn fmt_failure_is_returned() {
    struct Failure;
    impl std::fmt::Write for Failure {
        fn write_str(&mut self, _: &str) -> std::fmt::Result {
            Err(std::fmt::Error)
        }
    }
    assert!(html::write_html_fmt(Failure, Parser::new("hello")).is_err());
}
#[test]
fn tabs_do_not_create_hard_line_breaks() {
    for suffix in ["\t\t", " \t", "\t "] {
        let text = format!("one{suffix}\ntwo");
        assert!(
            !Parser::new(&text).any(|event| event == Event::HardBreak),
            "{text:?}: {}",
            render(&text, Options::empty())
        );
    }
}
#[test]
fn tab_after_closing_fence_is_ignored() {
    for fence in ["```", "~~~"] {
        let text = format!("{fence}\nx\n{fence}\t\nafter\n");
        assert_eq!(
            render(&text, Options::empty()),
            "<pre><code>x\n</code></pre>\n<p>after</p>\n"
        );
    }
}
#[test]
fn html_block_end_tags_are_case_insensitive() {
    assert_eq!(
        render("<pre>\nx\n</PRE>\n*after*\n", Options::empty()),
        "<pre>\nx\n</PRE>\n<p><em>after</em></p>\n"
    );
}
#[test]
fn html_block_end_tag_need_not_match_opening_tag() {
    assert_eq!(
        render("<pre>\nx\n</style>\n*after*\n", Options::empty()),
        "<pre>\nx\n</style>\n<p><em>after</em></p>\n"
    );
}
#[test]
fn inline_cdata_may_contain_isolated_closing_bracket() {
    assert_eq!(
        render("foo <![CDATA[a]b]]> bar", Options::empty()),
        "<p>foo <![CDATA[a]b]]> bar</p>\n"
    );
}
#[test]
fn unmatched_other_delimiter_does_not_suppress_subscript() {
    let text = "~a^ b~";
    assert_eq!(
        render(text, Options::ENABLE_SUBSCRIPT),
        "<p><sub>a^ b</sub></p>\n"
    );
    assert_eq!(
        render(
            text,
            Options::ENABLE_SUBSCRIPT | Options::ENABLE_SUPERSCRIPT
        ),
        "<p><sub>a^ b</sub></p>\n"
    );
}
#[test]
fn unmatched_other_delimiter_does_not_suppress_superscript() {
    let text = "^a~ b^";
    assert_eq!(
        render(text, Options::ENABLE_SUPERSCRIPT),
        "<p><sup>a~ b</sup></p>\n"
    );
    assert_eq!(
        render(
            text,
            Options::ENABLE_SUBSCRIPT | Options::ENABLE_SUPERSCRIPT
        ),
        "<p><sup>a~ b</sup></p>\n"
    );
}
