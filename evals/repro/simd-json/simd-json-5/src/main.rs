// Tape shortcut accessors must expose every element of a nested container.
use simd_json::prelude::*;
fn main() {
    let mut fail = 0;
    let mut i = b"{\"snot\": [1, 2, 3]}".to_vec();
    let t = simd_json::to_tape(i.as_mut_slice()).unwrap();
    let v = t.as_value();
    let a = v.get_array("snot").expect("array");
    let via_get: Vec<_> = a.iter().map(|e| e.as_u64()).collect();
    println!("get_array iter={via_get:?} len={} get(2)={:?}", a.len(), a.get(2).and_then(|e| e.as_u64()));
    if via_get.len() != 3 || a.get(2).and_then(|e| e.as_u64()) != Some(3) { fail += 1; }
    let a = v.try_get_array("snot").unwrap().unwrap();
    let via: Vec<_> = a.iter().map(|e| e.as_u64()).collect();
    println!("try_get_array iter={via:?} get(2)={:?}", a.get(2).and_then(|e| e.as_u64()));
    if via.len() != 3 || a.get(2).and_then(|e| e.as_u64()) != Some(3) { fail += 1; }
    // Control: the path that is not a shortcut sees all three.
    let full = v.get("snot").unwrap().as_array().unwrap();
    let ctl: Vec<_> = full.iter().map(|e| e.as_u64()).collect();
    println!("get(..).as_array() iter={ctl:?}");

    let mut i = b"{\"snot\": {\"x\": 1, \"y\": 2}}".to_vec();
    let t = simd_json::to_tape(i.as_mut_slice()).unwrap();
    let v = t.as_value();
    let o = v.get_object("snot").expect("object");
    let n = o.iter().count();
    let y = o.get("y").and_then(|e| e.as_u64());
    println!("get_object count={n} len={} y={y:?}", o.len());
    if n != 2 || y != Some(2) { fail += 1; }
    let o = v.try_get_object("snot").unwrap().unwrap();
    let n = o.iter().count();
    let y = o.get("y").and_then(|e| e.as_u64());
    println!("try_get_object count={n} y={y:?}");
    if n != 2 || y != Some(2) { fail += 1; }
    // Same off-by-one on an empty container: the slice is empty, so the header is gone too.
    std::panic::set_hook(Box::new(|i| println!("  panic: {i}")));
    let mut panics = 0;
    let mut i = b"{\"a\": [], \"o\": {}}".to_vec();
    let t = simd_json::to_tape(i.as_mut_slice()).unwrap();
    let v = t.as_value();
    let a = v.get_array("a").expect("array");
    if std::panic::catch_unwind(|| a.len()).is_err() { panics += 1; }
    if std::panic::catch_unwind(|| a.iter().count()).is_err() { panics += 1; }
    let o = v.get_object("o").expect("object");
    let o = std::panic::AssertUnwindSafe(o);
    if std::panic::catch_unwind(|| o.len()).is_err() { panics += 1; }
    println!("panics on empty containers from valid JSON: {panics}");
    println!("wrong_accessors={fail}");
    std::process::exit(if fail > 0 { 42 } else { 0 });
}
