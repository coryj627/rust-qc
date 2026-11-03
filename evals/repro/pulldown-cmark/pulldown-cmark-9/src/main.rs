use pulldown_cmark::Parser;
fn main() {
    // A list item holding only a link reference definition, then a whitespace-only line.
    let input = "- [o]:u\n    \t";
    println!("input {input:?}, Parser::new (default options), plain iteration");
    let n = Parser::new(input).count();
    println!("no panic, {n} events");
}
