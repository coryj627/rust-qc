//! Fixture: `data` skips a top-level folder, but inside `src/` it is a module name.
pub struct RowManager; // EXPECT NAME-5
pub fn f(x: Option<u8>) -> u8 {
    println!("hi"); // EXPECT PANIC-6
    x.expect("") // EXPECT PANIC-2
}
