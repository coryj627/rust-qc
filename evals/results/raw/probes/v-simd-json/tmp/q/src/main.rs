#![allow(unused)]
use simd_json::prelude::*;
use std::collections::HashMap;
fn o(s:&str)->String{let mut b=s.as_bytes().to_vec(); format!("{:?}",simd_json::to_owned_value(&mut b).map_err(|e|e.to_string()))}
fn main(){
  #[cfg(feature="b128")]{
    for s in ["170141183460469231731687303715884105727","-170141183460469231731687303715884105727"]{
    let mut b=s.as_bytes().to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value();
    println!("tape {:?} ty={:?} i128={:?} u128={:?} isu128={} cast_f64={:?}", t.0, v.value_type(), v.as_i128(), v.as_u128(), v.is_u128(), v.cast_f64());
    let mut b=s.as_bytes().to_vec(); let w=simd_json::to_owned_value(&mut b).unwrap(); println!("owned {:?} {:?} {:?}", w.as_i128(), w.as_u128(), w.cast_f64());
    let mut b=s.as_bytes().to_vec(); println!("f64 {:?}", simd_json::from_slice::<f64>(&mut b).map_err(|e|e.to_string()));
    }
    let mut h=HashMap::new(); h.insert(1u128,1u8); println!("u128 key to_owned {:?} to_string {:?}", simd_json::serde::to_owned_value(&h).map_err(|e|e.to_string()), simd_json::to_string(&h).map_err(|e|e.to_string()));
    let mut x=br#"{"1":1}"#.to_vec(); println!("i128 key from_slice {:?}", simd_json::from_slice::<HashMap<i128,u8>>(&mut x).map_err(|e|e.to_string()));
    println!("{}", o("[99999999999999999999x]"));
  }
  #[cfg(feature="bigf")]{
    for s in ["-9223372036854775809","-18446744073709551616","18446744073709551616","-9223372036854775808","[99999999999999999999x]"]{ println!("{s}: {}", o(s)); }
  }
  #[cfg(feature="approx")]{
    for s in ["5979E306","2e308","1.7976931348623159e308","-0.0","1e0","-0e0","0e0","0.0","18446744073709551619.5","28446744073709551619.5","18446744073709551616.5","18446744073709551617e2","12345678901234567890123.5","464E0263","1e-324","4.9e-324","-6.970748","6.04e-24","0.1","1.5"]{ println!("{s}: {} std={:?}", o(s), s.parse::<f64>()); }
    let mut st=88172645463325252u64; let mut nx=||{st^=st<<13;st^=st>>7;st^=st<<17;st};
    let (mut diff,mut tot,mut big,mut maxulp)=(0,0,0,0u64);
    for _ in 0..500000 { let a=nx()%1000000000; let d=nx()%10; let e=(nx()%40) as i32-20; let s=format!("{}.{}e{}",a,nx()%10u64.pow(d as u32+1),e);
      let want:f64=s.parse().unwrap(); let mut b=s.clone().into_bytes(); if let Ok(v)=simd_json::to_owned_value(&mut b){ if let Some(g)=v.as_f64(){ tot+=1; if g!=want {diff+=1; let u=(g.to_bits() as i64-want.to_bits() as i64).unsigned_abs(); if u>maxulp{maxulp=u}; if u>4 {big+=1}} } } }
    println!("approx diff {diff}/{tot} >4ulp {big} maxulp {maxulp}");
  }
}
