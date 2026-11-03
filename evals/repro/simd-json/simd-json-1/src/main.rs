// Deeply nested JSON through the public DOM APIs. No depth limit at the baseline.
// usage: repro <owned|borrowed|tape> <depth>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a[1].as_str();
    let depth: usize = a[2].parse().unwrap();
    let mut d = Vec::with_capacity(depth * 2);
    d.extend(std::iter::repeat(b'[').take(depth));
    d.extend(std::iter::repeat(b']').take(depth));
    match mode {
        "owned" => {
            let r = simd_json::to_owned_value(&mut d);
            println!("owned depth={depth} ok={}", r.is_ok());
            if let Err(e) = r { println!("err={e:?}"); }
        }
        "borrowed" => {
            let r = simd_json::to_borrowed_value(&mut d);
            println!("borrowed depth={depth} ok={}", r.is_ok());
            if let Err(e) = r { println!("err={e:?}"); }
        }
        "tape" => {
            let r = simd_json::to_tape(&mut d);
            println!("tape depth={depth} ok={}", r.is_ok());
        }
        _ => panic!("mode"),
    }
}
