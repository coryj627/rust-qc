use pulldown_cmark::{html, Parser};
fn test_markdown_html(input: &str, expected: &str, _: bool, _: bool, _: bool, _: bool, _: bool) {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new(input));
    fn normalize(s: &str) -> String {
        s.replace("<br>", "<br />")
            .replace("<br/>", "<br />")
            .replace("<hr>", "<hr />")
            .replace("<hr/>", "<hr />")
            .replace(">\n<", "><")
    }
    assert_eq!(normalize(&out), normalize(expected), "{input:?}");
}
#[path = "/work/crate/pulldown-cmark/tests/suite/spec.rs"]
mod commonmark;
