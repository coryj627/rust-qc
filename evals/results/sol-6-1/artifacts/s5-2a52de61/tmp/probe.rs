use pulldown_cmark::{html, Options, Parser};
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    for input in [
        "![[x]y](z)]]", "![[x](y)]]", "![[x](y)z]]", "![[x](y)z]w]", "![[x]y](z)a]]", "![[x]y](z)a[b]]", "![[x]y](z)a[b]c]]", "![[x]y](z)a b c]]", "![[x]]", "![[x|y]]", "![[x][y]]", "![[x]y][r]]]\n\n[r]: /z", "![[x]y][]]]\n\n[[x]y]: /z", "![[^x]]\n\n[^x]: footnote", "![[a]] [[b]]",
    ] {
        for options in [Options::ENABLE_WIKILINKS, Options::all(), Options::empty()] {
            let result = std::panic::catch_unwind(|| {
                let mut output = String::new();
                html::push_html(&mut output, Parser::new_ext(input, options));
                output
            });
            println!("{input:?} {options:?}: {result:?}");
        }
    }
}
