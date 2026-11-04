use pulldown_cmark::{html, Options, Parser};
fn main() {
    let cases = [
        ("[[Wiki]]", Options::ENABLE_WIKILINKS),
        ("![[dog.png|a dog]]", Options::ENABLE_WIKILINKS),
        ("[[Wiki|]]", Options::ENABLE_WIKILINKS),
        ("```rust\tcustom\ncode\n```", Options::empty()),
        ("`a\r\nb`", Options::empty()),
        ("> `a\r\n> b`", Options::empty()),
        ("a -- ---", Options::ENABLE_SMART_PUNCTUATION),
        (
            "ref [^FOO] [^foo]\n\n[^foo]: Note",
            Options::ENABLE_FOOTNOTES,
        ),
        (
            "ref [^Straße] [^STRASSE]\n\n[^strasse]: Note",
            Options::ENABLE_FOOTNOTES,
        ),
        ("~~abc~~", Options::ENABLE_SUBSCRIPT),
        ("^^abc^^", Options::ENABLE_SUPERSCRIPT),
    ];
    for (input, opts) in cases {
        let mut output = String::new();
        html::push_html(&mut output, Parser::new_ext(input, opts));
        println!("INPUT {input:?}\nOUTPUT {output:?}");
        for (ev, range) in Parser::new_ext(input, opts).into_offset_iter() {
            println!("{ev:?} {range:?} {:?}", &input[range.clone()]);
        }
    }
}
