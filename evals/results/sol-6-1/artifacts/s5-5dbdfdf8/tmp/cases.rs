use pulldown_cmark::{html, Options, Parser};
fn main() {
    for input in ["![[]a]()]]", "![[]a](image.png)]]", "![[]a](image.png)tail *text*]]", "![[]a](image.png)tail |alias]]", "![[]a](image.png)tail [label]]", "![[foo]]", "![[foo|bar]]", "![[[x](url)]]", "![[foo](/img)]]", "![[]a][ref]]]\n\n[ref]: /img", "![[]a][ref]tail *text*]]\n\n[ref]: /img"] {
        print!("{input:?} => ");
        let result = std::panic::catch_unwind(|| {
            let mut output = String::new();
            html::push_html(&mut output, Parser::new_ext(input, Options::ENABLE_WIKILINKS));
            output
        });
        println!("{result:?}");
    }
}
