// Safe hand-written Visitor: next_value without next_key on an empty object.
use serde::de::{self, MapAccess, Visitor};
use std::fmt;
#[derive(Debug)]
struct Evil;
impl<'de> de::Deserialize<'de> for Evil {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Evil;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("a map") }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Evil, A::Error> {
                for _ in 0..8 {
                    let n = map.next_value::<u64>()?;
                    println!("read past the object: {n}");
                }
                Ok(Evil)
            }
        }
        deserializer.deserialize_map(V)
    }
}
fn main() {
    let mut input = b"{}".to_vec();
    let r = simd_json::from_slice::<Evil>(&mut input);
    println!("result = {r:?}");
    assert!(r.is_err());
    println!("no bug: returned Err");
}
