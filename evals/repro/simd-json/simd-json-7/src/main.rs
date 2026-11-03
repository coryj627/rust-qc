// A fixed-length target must not let surplus array elements leak into later fields.
fn t<T: for<'a> serde::Deserialize<'a> + std::fmt::Debug>(s: &str) -> (String, String) {
    let mut d = s.as_bytes().to_vec();
    let a = format!("{:?}", simd_json::from_slice::<T>(&mut d).map_err(|e| e.to_string()));
    let b = format!("{:?}", serde_json::from_str::<T>(s).map_err(|e| e.to_string()));
    (a, b)
}
fn main() {
    let (a, b) = t::<((u8, u8), u8)>("[[1,2,3],4]");
    println!("((u8,u8),u8) [[1,2,3],4]: simd-json={a}\n  serde_json={b}");
    let wrong = a == "Ok(((1, 2), 3))";
    let (a2, b2) = t::<Vec<(u8, u8)>>("[[1,2,[9,9]],[4,5]]");
    println!("Vec<(u8,u8)> [[1,2,[9,9]],[4,5]]: simd-json={a2}\n  serde_json={b2}");
    let wrong2 = a2.starts_with("Ok(");
    std::process::exit(if wrong && wrong2 { 42 } else { 0 });
}
