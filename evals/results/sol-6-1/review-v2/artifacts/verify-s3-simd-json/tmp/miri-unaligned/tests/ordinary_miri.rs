#[test]
fn ordinary_parse() {
 let mut b=br#"{"name":"caf\u00e9","items":[1,null,true,{"x":"line\nbreak"}]}"#.to_vec();
 let v=simd_json::to_owned_value(&mut b).unwrap();
 assert_eq!(v["name"], "café");
}
