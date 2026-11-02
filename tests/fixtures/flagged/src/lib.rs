//! Fixture: each marked line must produce the named rule.
pub use inner::*; // EXPECT ORG-12
mod inner {
    pub struct ConfigManager; // EXPECT NAME-5
}
// We decided to use a Vec here. EXPECT DOC-11
// TODO fix later EXPECT CMT-3
// +-----+ EXPECT CMT-5
/// # Arguments EXPECT DOC-4
pub fn f(x: Option<u8>) -> u8 {
    println!("hi"); // EXPECT PANIC-6
    x.expect("") // EXPECT PANIC-2
}
