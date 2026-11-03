use pulldown_cmark::{html, Parser};
fn main() {
    let s = "<script>\nalert(1)\n</SCRIPT>\n\n# Title\n";
    let want = "<script>\nalert(1)\n</SCRIPT>\n<h1>Title</h1>\n";
    let mut got = String::new();
    html::push_html(&mut got, Parser::new(s));
    println!("input {s:?}\ngot  {got:?}\nwant {want:?}");
    std::process::exit(if got != want { 0 } else { 1 });
}
