fn main(){ let mut b=br#"[1,2,3]"#.to_vec(); let v=simd_json::to_owned_value(&mut b); println!("{:?}",v); }
