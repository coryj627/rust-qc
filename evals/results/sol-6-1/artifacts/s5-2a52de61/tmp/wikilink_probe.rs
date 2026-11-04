use pulldown_cmark::{Options, Parser};

fn check(input: &str) {
    for options in [Options::ENABLE_WIKILINKS, Options::all()] {
        let result = std::panic::catch_unwind(|| {
            for (_, range) in Parser::new_ext(input, options).into_offset_iter() {
                assert!(input.get(range).is_some());
            }
        });
        assert!(result.is_ok(), "failed to parse {input:?} with {options:?}");
    }
}

fn enumerate(prefix: &str, alphabet: &[u8], max_len: usize) -> usize {
    let mut count = 0;
    for len in 0..=max_len {
        for mut n in 0..alphabet.len().pow(len as u32) {
            let mut input = prefix.to_owned();
            for _ in 0..len {
                input.push(alphabet[n % alphabet.len()] as char);
                n /= alphabet.len();
            }
            check(&input);
            count += 1;
        }
    }
    count
}

fn main() {
    let mut count = enumerate("", b"![]()|a\\", 7);
    for prefix in ["![[x]y](z)", "![[é]文](画像)", "[[![[x]y](z)", "![[x](y)"] {
        count += enumerate(prefix, b"[]|a*", 6);
    }
    println!("Parsed {count} inputs with wikilinks alone and all options; no panics or invalid event ranges.");
}
