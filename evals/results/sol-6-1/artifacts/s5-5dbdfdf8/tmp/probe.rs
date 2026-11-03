use pulldown_cmark::{Options, Parser};
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let alphabet = b"[]!|()a\\";
    let mut failures = 0;
    for len in 0..=8 {
        for mut n in 0..alphabet.len().pow(len) {
            let mut input = String::from("![[");
            for _ in 0..len {
                input.push(alphabet[n % alphabet.len()] as char);
                n /= alphabet.len();
            }
            input.push_str("]]");
            if std::panic::catch_unwind(|| Parser::new_ext(&input, Options::ENABLE_WIKILINKS).count()).is_err() {
                println!("{input:?}");
                failures += 1;
                if failures == 30 { return; }
            }
        }
    }
}
