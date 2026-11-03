use simd_json::prelude::*;
fn t(s:&str){ let mut b=s.as_bytes().to_vec(); println!("{:?} -> {:?}",s,simd_json::to_owned_value(&mut b).map_err(|e|format!("{:?}@{}",e.error(),e.index()))); }
fn main(){
  for s in ["1e0","0e0","12e0","1.0","10e-1","1.5e1","18446744073709551619.5","18446744073709551615","18446744073709551616","28446744073709551619.5","99999999999999999999","99999999999999999999x","123456789012345678901234567890x","-99999999999999999999","-170141183460469231731687303715884105728","170141183460469231731687303715884105727","340282366920938463463374607431768211455","340282366920938463463374607431768211456","-170141183460469231731687303715884105729","1e400","-1e400","1e-400","5e-324","1e-323","1.7976931348623157e308","0.000000000000000000000000000000000000000000001","1.23456789012345678901234567890e5","1234567890123456789012.5","-0","-0.0","-0e0","0.0", "1e4294967296","123456789012345678x","1234567890123456789x"] { t(s); }
  let big="170141183460469231731687303715884105727";
  let mut b=big.as_bytes().to_vec(); if let Ok(tp)=simd_json::to_tape(&mut b){ let v=tp.as_value(); println!("tape as_i128 {:?} as_u128 {:?} cast_f64 {:?} is_i128 {:?} as_i64 {:?}",v.as_i128(),v.as_u128(),v.cast_f64(),v.is_i128(),v.as_i64()); println!("tape dbg {:?} encode {}",tp.0,v.encode()); }
  let mut b=big.as_bytes().to_vec(); if let Ok(v)=simd_json::to_owned_value(&mut b){ println!("owned as_i128 {:?} as_u128 {:?} cast_f64 {:?}",v.as_i128(),v.as_u128(),v.cast_f64()); }
  let mut b=big.as_bytes().to_vec(); println!("serde f64 from big {:?}",simd_json::from_slice::<f64>(&mut b));
  let mut b=big.as_bytes().to_vec(); println!("serde i128 from big {:?}",simd_json::from_slice::<i128>(&mut b));
  let mut b=big.as_bytes().to_vec(); println!("serde u128 from big {:?}",simd_json::from_slice::<u128>(&mut b));
  let mut b=big.as_bytes().to_vec(); println!("serde any->sj {:?}",simd_json::from_slice::<serde_json::Value>(&mut b).map_err(|e|e.to_string()));
  let mut b=big.as_bytes().to_vec(); println!("serde any->owned {:?}",simd_json::from_slice::<simd_json::OwnedValue>(&mut b).map_err(|e|e.to_string()));
  let mut b=big.as_bytes().to_vec(); let v=simd_json::to_owned_value(&mut b); println!("from_owned u128 {:?}", v.and_then(|v|simd_json::serde::from_owned_value::<u128>(v)).map_err(|e|e.to_string()));
  let mut m=std::collections::HashMap::new(); m.insert(1u128,1u8);
  println!("to_string u128 key {:?}",simd_json::to_string(&m).map_err(|e|e.to_string()));
  println!("to_owned u128 key {:?}",simd_json::serde::to_owned_value(&m).map_err(|e|e.to_string()));
  let s=r#"{"1":1}"#; let mut b=s.as_bytes().to_vec(); println!("from_slice u128 key {:?}",simd_json::from_slice::<std::collections::HashMap<u128,u8>>(&mut b).map_err(|e|e.to_string()));
  println!("to_owned i128 {:?}",simd_json::serde::to_owned_value(1i128).map_err(|e|e.to_string()));
  println!("to_string i128 {:?}",simd_json::to_string(&1i128).map_err(|e|e.to_string()));
  // ordering of numbers in eq
  println!("eq i64/u64 {}",simd_json::OwnedValue::from(1i64)==simd_json::OwnedValue::from(1u64));
  println!("try_from sj {:?}",simd_json::OwnedValue::try_from(serde_json::json!({"a":1,"b":-1,"c":1.5,"d":u64::MAX})).map(|v|v.encode()));
  let r:Result<serde_json::Value,_>=simd_json::OwnedValue::from(f64::NAN).try_into(); println!("nan->sj {:?}",r.map_err(|e|e.to_string()));
}
#[allow(dead_code)] fn _unused(){}
