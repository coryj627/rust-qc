// Invalid UTF-8 inside a JSON string, through public safe API.
fn main() {
    let mut d = b"\"\xff\"".to_vec();
    let v = simd_json::to_borrowed_value(&mut d);
    match v {
        Err(e) => println!("rejected: {e:?}"),
        Ok(v) => {
            use simd_json::prelude::*;
            let s: &str = v.as_str().expect("a string");
            println!("ACCEPTED bytes={:?} from_utf8_ok={}", s.as_bytes(), std::str::from_utf8(s.as_bytes()).is_ok());
            // A safe str method on a &str that breaks the UTF-8 invariant.
            let n = s.chars().count();
            let c = s.chars().next();
            println!("chars count={n} first={c:?}");
        }
    }
}
