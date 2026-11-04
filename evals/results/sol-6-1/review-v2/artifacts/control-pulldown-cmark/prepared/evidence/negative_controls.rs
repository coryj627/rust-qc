use pulldown_cmark::{html, BrokenLink, Event, LinkType, Options, Parser, Tag, TagEnd};
use std::io::{self, Write};

fn render(input: &str, options: Options) -> String {
    let mut output = String::new();
    html::push_html(&mut output, Parser::new_ext(input, options));
    output
}

#[test]
fn decoy_1_first_reference_definition_wins() {
    let input = "[ref]: /first \"first title\"\n[REF]: /second \"second title\"\n\n[shown][ref] [ref][] [ref]\n";
    let parser = Parser::new(input);
    let def = parser.reference_definitions().get("REF").unwrap();
    assert_eq!(def.dest.as_ref(), "/first");
    assert_eq!(def.title.as_ref().unwrap().as_ref(), "first title");
    let mut output = String::new();
    html::push_html(&mut output, parser);
    assert_eq!(output, "<p><a href=\"/first\" title=\"first title\">shown</a> <a href=\"/first\" title=\"first title\">ref</a> <a href=\"/first\" title=\"first title\">ref</a></p>\n");
    println!("input={input:?}\noutput={output:?}\nFirst definition's destination and title preserved for full, collapsed, and shortcut references.");
}

#[test]
fn decoy_2_all_space_code_spans_preserved() {
    for spaces in [1, 2, 3, 5] {
        let payload = " ".repeat(spaces);
        let input = format!("`{payload}`");
        let codes: Vec<_> = Parser::new(&input).filter_map(|event| match event {
            Event::Code(code) => Some(code.to_string()),
            _ => None,
        }).collect();
        assert_eq!(codes, vec![payload.clone()]);
        let output = render(&input, Options::empty());
        assert_eq!(output, format!("<p><code>{payload}</code></p>\n"));
        println!("input={input:?} code={codes:?} output={output:?}");
    }
    assert_eq!(render("` a `", Options::empty()), "<p><code>a</code></p>\n");
    assert_eq!(render("`  a  `", Options::empty()), "<p><code> a </code></p>\n");
    println!("Non-space spans still trim exactly one surrounding space.");
}

#[derive(Default)]
struct ShortWriter {
    bytes: Vec<u8>,
    short_writes: usize,
    calls: usize,
}
impl Write for ShortWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        let count = bytes.len().min(2);
        if count < bytes.len() { self.short_writes += 1; }
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[test]
fn false_1_io_short_writes_are_retried() {
    let input = "3. café & tea\n";
    let mut writer = ShortWriter::default();
    html::write_html_io(&mut writer, Parser::new(input)).unwrap();
    let expected = "<ol start=\"3\">\n<li>café &amp; tea</li>\n</ol>\n";
    assert_eq!(writer.bytes, expected.as_bytes());
    assert!(writer.short_writes > 0);
    println!("input={input:?}\noutput={:?}\nwrite calls={} short writes={}", String::from_utf8(writer.bytes).unwrap(), writer.calls, writer.short_writes);
}

#[test]
fn false_2_nested_image_alt_keeps_outer_tail() {
    let input = "![before *em* ![inner](inner.png) after](outer.png) tail\n";
    let events: Vec<_> = Parser::new(input).collect();
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Start(Tag::Image { .. }))).count(), 2);
    let mut output = String::new();
    html::push_html(&mut output, events.clone().into_iter());
    assert_eq!(output, "<p><img src=\"outer.png\" alt=\"before em inner after\" /> tail</p>\n");
    println!("input={input:?}\nevents={events:?}\noutput={output:?}");
}

#[test]
fn false_3_short_table_rows_are_padded() {
    let input = "| a | b | c |\n| --- | --- | --- |\n| one |\n| two | three |\n";
    let options = Options::ENABLE_TABLES;
    let mut in_body_row = false;
    let mut row_cells = Vec::new();
    for event in Parser::new_ext(input, options) {
        match event {
            Event::Start(Tag::TableRow) => { in_body_row = true; row_cells.push(0); }
            Event::Start(Tag::TableCell) if in_body_row => *row_cells.last_mut().unwrap() += 1,
            Event::End(TagEnd::TableRow) => in_body_row = false,
            _ => {}
        }
    }
    assert_eq!(row_cells, vec![3, 3]);
    let output = render(input, options);
    assert_eq!(output, "<table><thead><tr><th>a</th><th>b</th><th>c</th></tr></thead><tbody>\n<tr><td>one</td><td></td><td></td></tr>\n<tr><td>two</td><td>three</td><td></td></tr>\n</tbody></table>\n");
    println!("input={input:?}\nbody-row cell counts={row_cells:?}\noutput={output:?}");
}

#[test]
fn false_4_defined_references_bypass_broken_link_callback() {
    let input = "[known]: /defined \"Defined\"\n\n[ok][known] [fallback][missing]\n";
    let mut seen = Vec::new();
    let mut callback = |link: BrokenLink<'_>| {
        seen.push(link.reference.to_string());
        Some(("/fallback".into(), "Fallback".into()))
    };
    let events: Vec<_> = Parser::new_with_broken_link_callback(input, Options::empty(), Some(&mut callback)).collect();
    assert_eq!(seen, vec!["missing"]);
    let links: Vec<_> = events.iter().filter_map(|event| match event {
        Event::Start(Tag::Link { link_type, dest_url, title, .. }) => Some((*link_type, dest_url.to_string(), title.to_string())),
        _ => None,
    }).collect();
    assert_eq!(links, vec![
        (LinkType::Reference, "/defined".to_owned(), "Defined".to_owned()),
        (LinkType::ReferenceUnknown, "/fallback".to_owned(), "Fallback".to_owned()),
    ]);
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    assert_eq!(output, "<p><a href=\"/defined\" title=\"Defined\">ok</a> <a href=\"/fallback\" title=\"Fallback\">fallback</a></p>\n");
    println!("input={input:?}\ncallback labels={seen:?}\nlinks={links:?}\noutput={output:?}");
}

#[test]
fn false_5_repeated_footnotes_reuse_number() {
    let input = "first[^a] second[^a] third[^b]\n\n[^a]: Alpha\n[^b]: Beta\n";
    let output = render(input, Options::ENABLE_FOOTNOTES);
    assert_eq!(output, "<p>first<sup class=\"footnote-reference\"><a href=\"#a\">1</a></sup> second<sup class=\"footnote-reference\"><a href=\"#a\">1</a></sup> third<sup class=\"footnote-reference\"><a href=\"#b\">2</a></sup></p>\n<div class=\"footnote-definition\" id=\"a\"><sup class=\"footnote-definition-label\">1</sup>\n<p>Alpha</p>\n</div>\n<div class=\"footnote-definition\" id=\"b\"><sup class=\"footnote-definition-label\">2</sup>\n<p>Beta</p>\n</div>\n");
    println!("input={input:?}\noutput={output:?}\nReference numbers are 1, 1, 2; definition labels are 1, 2.");
}
