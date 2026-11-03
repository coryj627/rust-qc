#![cfg(feature = "html")]

use pulldown_cmark::{html, Options, Parser};

fn render(input: &str, options: Options) -> String {
    let mut output = String::new();
    html::push_html(&mut output, Parser::new_ext(input, options));
    output
}

#[test]
fn malformed_delimiter_neighbors_finish() {
    // Only completion and retained output are specified for malformed input.
    // Unicode prefixes also exercise byte-index handling on untrusted text.
    let inputs = [
        "![[] ]()]]",
        " é ![[] ]()]]",
        "前 ![[] ]()]] 後",
        "![[] ]()]]\n\n[[Page]]",
        "![[] ]()]]\n![[] ]()]]",
        "]] [[]] [[|]] [[|Symbol]] [[",
    ];
    for input in inputs {
        for options in [Options::ENABLE_WIKILINKS, Options::all()] {
            assert!(!render(input, options).is_empty(), "input={input:?}, options={options:?}");
        }
    }
}

#[test]
fn valid_wikilinks_and_images_keep_rendering() {
    for (input, expected) in [
        ("[[Page]]", "<p><a href=\"Page\">Page</a></p>\n"),
        ("[[Page|a **label**]]", "<p><a href=\"Page\">a <strong>label</strong></a></p>\n"),
        ("![[dog.png|a cute dog]]", "<p><img src=\"dog.png\" alt=\"a cute dog\" /></p>\n"),
    ] {
        assert_eq!(render(input, Options::ENABLE_WIKILINKS), expected, "input={input:?}");
    }
}

#[test]
fn ordinary_markdown_keeps_rendering_with_wikilinks_on_or_off() {
    let input = "[label](https://example.org/) ![alt](dog.png) **bold** `[[code]]`";
    let expected = "<p><a href=\"https://example.org/\">label</a> <img src=\"dog.png\" alt=\"alt\" /> <strong>bold</strong> <code>[[code]]</code></p>\n";
    for options in [Options::empty(), Options::ENABLE_WIKILINKS] {
        assert_eq!(render(input, options), expected);
    }
}

#[test]
fn wikilinks_remain_opt_in() {
    assert_eq!(render("[[Page|label]]", Options::empty()), "<p>[[Page|label]]</p>\n");
}
