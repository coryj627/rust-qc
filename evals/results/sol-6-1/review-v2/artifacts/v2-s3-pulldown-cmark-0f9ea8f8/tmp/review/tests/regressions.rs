use pulldown_cmark::{Event, Options, Parser, Tag};
use pulldown_review::render;
#[test]
fn reference_label_character_limit() {
    for (ch, len, expected) in [('a', 999, 1), ('a', 1000, 0), ('é', 499, 1), ('é', 500, 1), ('界', 334, 1), ('界', 999, 1)] {
        let label = ch.to_string().repeat(len);
        let md = format!("[text][{label}]\n\n[{label}]: /url\n");
        let links = Parser::new(&md).filter(|e| matches!(e, Event::Start(Tag::Link { .. }))).count();
        println!("{ch} x {len}: expected {expected} link(s), got {links}");
    }
    let label = "é".repeat(500);
    let md = format!("[text][{label}]\n\n[{label}]: /url\n");
    assert_eq!(render(&md, Options::empty()), "<p><a href=\"/url\">text</a></p>\n");
}
#[test]
fn wikilink_source_range() {
    let mut bad_ranges = Vec::new();
    for md in ["[[page]]", "[[page|label]]", "![[image.png]]"] {
        let events: Vec<_> = Parser::new_ext(md, Options::ENABLE_WIKILINKS).into_offset_iter().collect();
        println!("{md:?}: {events:?}");
        let (_, range) = events.iter().find(|(e,_)| matches!(e, Event::Start(Tag::Link {..} | Tag::Image {..}))).unwrap();
        if &md[range.clone()] != md { bad_ranges.push((md, range.clone())); }
    }
    assert!(bad_ranges.is_empty(), "{bad_ranges:?}");
}
#[test]
fn footnote_case_matching() {
    let md = "ref[^A] and again[^a]\n\n[^a]: definition\n";
    let out = render(md, Options::ENABLE_FOOTNOTES);
    println!("{out}");
    assert!(!out.contains("href=\"#A\""));
    assert!(!out.contains("footnote-definition-label\">2"));
}
#[test]
fn fenced_code_tab_info() {
    assert_eq!(render("```rust\tmore\ncode\n```\n", Options::empty()), "<pre><code class=\"language-rust\">code\n</code></pre>\n");
}
#[test]
fn heading_closing_tabs() {
    let inputs = ["# heading\t\n", "# heading\t###\n", "# heading ###\t\n"];
    let actual: Vec<_> = inputs.iter().map(|md| render(md, Options::empty())).collect();
    println!("{inputs:?} -> {actual:?}");
    assert!(actual.iter().all(|html| html == "<h1>heading</h1>\n"));
}
#[test]
fn carriage_return_lines() {
    for lf in ["```rust\nx\n```\nafter\n", "    x\n\nafter\n", "<div>\nx\n\nafter\n", "<!--x-->\nafter\n"] {
        let cr = lf.replace('\n', "\r");
        let crlf = lf.replace('\n', "\r\n");
        println!("LF {lf:?} -> {:?}", render(lf, Options::empty()));
        println!("CR {cr:?} -> {:?}", render(&cr, Options::empty()));
        assert_eq!(render(lf, Options::empty()), render(&crlf, Options::empty()));
    }
    let lf = "```rust\nx\n```\nafter\n";
    assert_eq!(render(lf, Options::empty()), render(&lf.replace('\n', "\r"), Options::empty()));
}
#[test]
fn closing_fence_tab() {
    let md = "```\nx\n```\t\nafter\n";
    assert_eq!(render(md, Options::empty()), "<pre><code>x\n</code></pre>\n<p>after</p>\n");
}
#[test]
fn tabs_do_not_make_hard_breaks() {
    let whitespace = ["\t\t", " \t", "\t "];
    let actual: Vec<_> = whitespace.iter().map(|ws| render(&format!("foo{ws}\nbar\n"), Options::empty())).collect();
    println!("{whitespace:?} -> {actual:?}");
    assert!(actual.iter().all(|html| html == "<p>foo\nbar</p>\n"));
}
#[test]
fn single_character_heading_attribute() {
    let md = "# heading {a bb c=d}\n";
    assert_eq!(render(md, Options::ENABLE_HEADING_ATTRIBUTES), "<h1 a=\"\" bb=\"\" c=\"d\">heading</h1>\n");
}
#[test]
fn subscript_only_preserves_inline_parsing() {
    for md in ["~~*a*~~", "~~[a](b)~~", "~~`a`~~"] {
        println!("{md:?}: {:?}", render(md, Options::ENABLE_SUBSCRIPT));
    }
    assert_eq!(render("~~*a*~~", Options::ENABLE_SUBSCRIPT), "<p>~~<em>a</em>~~</p>\n");
}
#[test]
fn linked_wiki_image() {
    let md = "[![[foo.png]]](bar)";
    println!("{:?}", render(md, Options::ENABLE_WIKILINKS));
    assert_eq!(render(md, Options::ENABLE_WIKILINKS), "<p><a href=\"bar\"><img src=\"foo.png\" alt=\"foo.png\" /></a></p>\n");
}
#[test]
fn headings_with_attributes_keep_trailing_backslash() {
    for opts in [Options::empty(), Options::ENABLE_HEADING_ATTRIBUTES] {
        println!("opts {opts:?}: {:?}", render("# heading \\\nnext\n", opts));
    }
    assert_eq!(render("# heading \\\nnext\n", Options::ENABLE_HEADING_ATTRIBUTES), "<h1>heading \\</h1>\n<p>next</p>\n");
}
#[test]
fn uppercase_html_closing_tag_ends_block() {
    let md = "<pre>\nx\n</PRE>\n*after*\n";
    assert_eq!(render(md, Options::empty()), "<pre>\nx\n</PRE>\n<p><em>after</em></p>\n");
}
#[test]
fn tab_between_unquoted_html_attributes() {
    let md = "a <b x=one\ty=two>z</b>\n";
    assert_eq!(render(md, Options::empty()), "<p>a <b x=one\ty=two>z</b></p>\n");
}
#[test]
fn crlf_is_one_code_span_line_ending() {
    for line_ending in ["\n", "\r", "\r\n"] {
        let md = format!("`a{line_ending}b`");
        println!("{md:?}: {:?}", render(&md, Options::empty()));
        assert_eq!(render(&md, Options::empty()), "<p><code>a b</code></p>\n");
    }
}
#[test]
fn table_colon_must_be_at_edge() {
    let md = "| header |\n| -:- |\n| value |\n";
    assert!(!render(md, Options::ENABLE_TABLES).contains("<table>"), "{}", render(md, Options::ENABLE_TABLES));
}
#[test]
fn unmatched_caret_does_not_cancel_strikethrough() {
    let md = "~~a b^ c~~";
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_SUPERSCRIPT;
    assert_eq!(render(md, opts), "<p><del>a b^ c</del></p>\n");
}
#[test]
fn html_writers_propagate_errors() {
    struct IoFail(usize);
    impl std::io::Write for IoFail {
        fn write(&mut self, s: &[u8]) -> std::io::Result<usize> {
            if self.0 == 0 { return Err(std::io::ErrorKind::Other.into()); }
            let n = self.0.min(s.len()); self.0 -= n; Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    struct FmtFail;
    impl std::fmt::Write for FmtFail {
        fn write_str(&mut self, _: &str) -> std::fmt::Result { Err(std::fmt::Error) }
    }
    let md = "# title\n\n*body* [link](dest) ![alt](img)\n";
    for limit in 0..40 {
        assert!(pulldown_cmark::html::write_html_io(IoFail(limit), Parser::new(md)).is_err());
    }
    assert!(pulldown_cmark::html::write_html_fmt(FmtFail, Parser::new(md)).is_err());
}
