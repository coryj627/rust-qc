//! Fixture: test data inside `src/` stays skipped, so nothing here is flagged.
pub struct InputManager;
pub fn f(x: Option<u8>) -> u8 {
    println!("hi");
    x.expect("")
}
