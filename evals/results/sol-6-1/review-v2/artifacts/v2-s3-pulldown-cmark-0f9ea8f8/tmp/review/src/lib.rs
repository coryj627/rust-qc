use pulldown_cmark::{html, Options, Parser};
pub fn render(s: &str, opts: Options) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(s, opts));
    out
}
