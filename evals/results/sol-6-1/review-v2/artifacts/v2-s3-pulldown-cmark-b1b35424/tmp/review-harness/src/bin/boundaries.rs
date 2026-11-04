use pulldown_cmark::{html, Options, Parser};
fn render(input: &str, options: Options) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(input, options));
    out
}
fn main() {
    for (unit, count) in [
        ("é", 499),
        ("é", 500),
        ("🦀", 249),
        ("🦀", 250),
        ("a", 999),
        ("a", 1000),
    ] {
        let label = unit.repeat(count);
        let input = format!("[text][{label}]\n\n[{label}]: /url\n");
        println!(
            "label {unit:?} * {count}: link={}",
            render(&input, Options::empty()).contains("href=")
        );
    }
    for lf in ["\n", "\r\n", "\r"] {
        for base in [
            "```rust\ncode\n```\n",
            "    one\n    two\n",
            "<div>\none\n</div>\n\nafter\n",
            "# heading\n\npara\n",
            "```\ncode\n```\t\nafter\n",
            "# heading {x}",
        ] {
            let input = base.replace('\n', lf);
            println!(
                "input {input:?}: {:?}",
                render(&input, Options::ENABLE_HEADING_ATTRIBUTES)
            );
        }
    }
}
