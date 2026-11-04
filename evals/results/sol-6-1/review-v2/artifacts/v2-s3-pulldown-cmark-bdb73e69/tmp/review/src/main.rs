use pulldown_cmark::{html, Options, Parser};
fn main() {
    for s in [
        "```\nx\n```\t\nafter\n",
        "~~~\nx\n~~~\t\nafter\n",
        "# heading {x}",
        "![a <b>b</b> c](img)",
        "![a <!-- comment --> b](img)",
        "![a <br> b](img)",
        "a\tb\nc\n",
        "a \t\nb\n",
        "a\t \nb\n",
        "[x](url\"title\")",
        "[x](url'title')",
        "[x](<url>\"title\")",
    ] {
        let mut out = String::new();
        html::push_html(
            &mut out,
            Parser::new_ext(s, Options::ENABLE_HEADING_ATTRIBUTES),
        );
        println!("INPUT {s:?} => {out:?}");
    }
}
