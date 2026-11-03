//! Fixture: a `tests` folder inside `src/` stays non-production, so nothing here is flagged.
pub fn f(x: Option<u8>) -> u8 {
    println!("tests may print");
    x.expect("")
}
