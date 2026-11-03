use pulldown_cmark::{Options, Parser};
fn main() {
    let alphabet = b"[]!|()a\\";
    let mut count = 0_u64;
    for len in 0..=7 {
        for mut n in 0..alphabet.len().pow(len) {
            let mut input = String::new();
            for _ in 0..len {
                input.push(alphabet[n % alphabet.len()] as char);
                n /= alphabet.len();
            }
            for (prefix, suffix) in [("", ""), ("![[", "]]"), ("![[]a]()", ""), ("![[]a][ref]", "\n\n[ref]: image.png")] {
                let text = format!("{prefix}{input}{suffix}");
                for options in [Options::ENABLE_WIKILINKS, Options::all()] {
                    for (_, range) in Parser::new_ext(&text, options).into_offset_iter() {
                        assert!(text.get(range).is_some(), "invalid range for {text:?}");
                    }
                    count += 1;
                }
            }
        }
    }
    println!("Parsed {count} delimiter cases without panics or invalid source ranges");
}
