use pulldown_cmark::{Options, Parser};
fn main() {
    let input = "![[] ]()]]";
    println!("input {input:?}, Options::ENABLE_WIKILINKS only");
    let n = Parser::new_ext(input, Options::ENABLE_WIKILINKS).count();
    println!("no panic, {n} events");
}
