use pulldown_cmark::{Options, Parser};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let tokens = ["![", "[", "]", "(u)", "[r]", "|", "a", "é", "\\]", "`", "\n"];
    let mut state = 42u64;
    let mut failures = 0;
    for _ in 0..100_000 {
        let mut s = String::from("![[");
        for _ in 0..10 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            s.push_str(tokens[state as usize % tokens.len()]);
        }
        s.push_str("]] tail");
        let result = catch_unwind(AssertUnwindSafe(|| {
            Parser::new_ext(&s, Options::ENABLE_WIKILINKS).count()
        }));
        if result.is_err() {
            failures += 1;
            if failures <= 30 { println!("{s:?}"); }
        }
    }
    println!("failures: {failures}");
}
