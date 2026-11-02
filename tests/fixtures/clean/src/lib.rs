//! Fixture: nothing here should be flagged.
/*!
Block doc with an example that prints:
println!("{x}");
*/
pub(crate) use imp::*;
mod imp {
    pub(crate) struct Manager;
}
// TODO(#12): owner noted.
/// A service-free name.
pub struct Inventory;
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        println!("tests may print");
        let _ = Some(1).expect("");
    }
}
