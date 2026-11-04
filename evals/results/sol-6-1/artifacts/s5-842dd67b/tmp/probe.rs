use pulldown_cmark::{Parser, Options, html};
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    for input in std::env::args().skip(1) {
        let result = std::panic::catch_unwind(|| {
            let mut out = String::new();
            html::push_html(&mut out, Parser::new_ext(&input, Options::ENABLE_WIKILINKS));
            out
        });
        println!("{input:?}: {result:?}");
    }
}
