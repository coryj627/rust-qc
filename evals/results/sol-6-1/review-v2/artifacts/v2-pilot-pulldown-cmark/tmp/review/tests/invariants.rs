use pulldown_cmark::{html, Event, Options, Parser, TextMergeStream, TextMergeWithOffset};
#[test]
fn ordinary_inline_block_combinations_preserve_events_and_balance() {
    let inlines = [
        "plain",
        "café 中文 🦀",
        "*em* **strong**",
        "`code`",
        "[name](page \"title\")",
        "![alt](photo.png)",
        "a &amp; b",
        "a  \nb",
        "a\\\nb",
        "[^note]",
        "[[Page|alias]]",
        "![[photo.png|alt]]",
        "$x+1$ $$y$$",
        "^sup^ ~sub~ ~~strike~~",
        "<em>html</em>",
    ];
    let options = [
        Options::empty(),
        Options::all(),
        Options::all() & !Options::ENABLE_OLD_FOOTNOTES,
    ];
    let mut count = 0;
    for inline in inlines {
        for input in [
            format!("{inline}\n"),
            format!("# {inline}\n"),
            format!("> {inline}\n"),
            format!("- {inline}\n"),
            format!("| col |\n| --- |\n| {inline} |\n"),
            format!("{inline}\n\n[^note]: note\n"),
        ] {
            for opts in options {
                let events: Vec<_> = Parser::new_ext(&input, opts).collect();
                let offset_events: Vec<_> =
                    Parser::new_ext(&input, opts).into_offset_iter().collect();
                assert_eq!(
                    events,
                    offset_events
                        .iter()
                        .map(|(e, _)| e.clone())
                        .collect::<Vec<_>>(),
                    "{input:?} {opts:?}"
                );
                let mut stack = Vec::new();
                for (event, range) in &offset_events {
                    assert!(input.get(range.clone()).is_some(), "{input:?} {range:?}");
                    match event {
                        Event::Start(tag) => stack.push(tag.to_end()),
                        Event::End(end) => assert_eq!(stack.pop(), Some(*end), "{input:?}"),
                        _ => {}
                    }
                }
                assert!(stack.is_empty(), "{input:?}");
                let merged: Vec<_> = TextMergeStream::new(events.into_iter()).collect();
                let merged_offsets: Vec<_> = TextMergeWithOffset::new(offset_events.into_iter())
                    .map(|(e, _)| e)
                    .collect();
                assert_eq!(merged, merged_offsets, "{input:?}");
                let mut out = String::new();
                html::push_html(&mut out, Parser::new_ext(&input, opts));
                let mut bytes = Vec::new();
                html::write_html_io(&mut bytes, Parser::new_ext(&input, opts))
                    .expect("Vec write is infallible");
                assert_eq!(out.as_bytes(), bytes, "{input:?}");
                count += 1;
            }
        }
    }
    eprintln!("checked {count} inline/block/option combinations");
}
