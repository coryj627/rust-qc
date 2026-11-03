use pulldown_cmark::{html, Event, Options, Parser, Tag};
fn main() {
    for (name, text, opts) in [
        (
            "fence-tab",
            "```rust\tfoo\nlet x = 1;\n```\n",
            Options::empty(),
        ),
        (
            "wiki-offset",
            "Before [[Page]] after ![[a.png|alt]].",
            Options::ENABLE_WIKILINKS,
        ),
        (
            "wiki-multiline",
            "[[Page\nName]]",
            Options::ENABLE_WIKILINKS,
        ),
        ("wiki-empty-pipe", "[[Page|]]", Options::ENABLE_WIKILINKS),
        (
            "footnote-case",
            "a[^A] b[^a]\n\n[^A]: note\n",
            Options::ENABLE_FOOTNOTES,
        ),
        ("code-nul", "```\na\0b\n```", Options::empty()),
        (
            "metadata-nul",
            "---\na\0b\n---\nafter",
            Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,
        ),
    ] {
        let mut out = String::new();
        html::push_html(&mut out, Parser::new_ext(text, opts));
        println!("{name}: {text:?} => {out:?}");
        for (event, range) in Parser::new_ext(text, opts).into_offset_iter() {
            println!("  {range:?}: {event:?}");
        }
    }
    let text = format!(
        "[r]: https://example.com/{}\n\n{}",
        "a".repeat(180),
        "[r] ".repeat(600)
    );
    let links = Parser::new(&text)
        .filter(|e| matches!(e, Event::Start(Tag::Link { .. })))
        .count();
    println!(
        "repeated-ref: input_bytes={} links={links} expected=600",
        text.len()
    );
}
