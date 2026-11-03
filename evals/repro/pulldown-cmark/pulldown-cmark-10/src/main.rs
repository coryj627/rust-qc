use pulldown_cmark::{Options, Parser};
fn main() {
    let input = "\u{fffd}\u{fffd}  2\n:<\n  {}\n -\n";
    let o = Options::ENABLE_HEADING_ATTRIBUTES | Options::ENABLE_DEFINITION_LIST;
    println!("input {input:?}, ENABLE_HEADING_ATTRIBUTES | ENABLE_DEFINITION_LIST");
    let n = Parser::new_ext(input, o).count();
    println!("no panic, {n} events");
}
