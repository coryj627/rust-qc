use simd_json::prelude::*;
fn main(){
 let mut src=br#"{"a":[1,2,3],"o":{"x":1,"y":[5,6]},"e":[],"n":1}"#.to_vec(); let t=simd_json::to_tape(&mut src).unwrap(); let l=simd_json::value::lazy::Value::from_tape(t.as_value());
 let a=l.get_array("a").unwrap(); println!("lazy get_array len={} iter={:?}", a.len(), a.iter().map(|v|v.as_u64()).collect::<Vec<_>>());
 let mut l2=l.clone(); l2.insert("n",1).unwrap(); println!("lazy eq tape vs upgraded {}", l==l2);
 let mut src2=br#"{"a":[1]}"#.to_vec(); let t2=simd_json::to_tape(&mut src2).unwrap(); let l3=simd_json::value::lazy::Value::from_tape(t2.as_value()); let l4=simd_json::value::lazy::Value::Value(std::borrow::Cow::Owned(l3.clone().into_value())); println!("lazy eq tape vs value same content {}", l3==l4);
 let mut q=b"1".to_vec(); let t3=simd_json::to_tape(&mut q).unwrap(); println!("empty tape encode panics? {:?}", std::panic::catch_unwind(||{ simd_json::Tape::null().reset().as_value().encode() }).map_err(|_|"panic")); let _=t3;
 let mut bad=b"[1,2,[3".to_vec(); let mut buf=simd_json::Buffers::default(); let mut tp=simd_json::Tape::null(); let r=simd_json::fill_tape(&mut bad,&mut buf,&mut tp); println!("partial tape {:?} {:?} encode={:?}", r.map_err(|e|e.to_string()), tp.0, std::panic::catch_unwind(std::panic::AssertUnwindSafe(||tp.as_value().encode())).map_err(|_|"panic"));
 #[derive(serde::Serialize)] struct P{a:u8,b:u8} println!("pretty struct {:?}", simd_json::to_string_pretty(&P{a:1,b:2}).unwrap());
 #[derive(serde::Serialize)] enum V{T(u8,u8)} println!("pretty nested {}", simd_json::to_string_pretty(&vec![V::T(1,2),V::T(3,4)]).unwrap());
 let s=simd_json::to_string_pretty(&vec![V::T(1,2)]).unwrap(); let mut b=s.into_bytes(); println!("  reparse {:?}", simd_json::to_owned_value(&mut b).map(|_|()).map_err(|e|e.to_string()));
}
