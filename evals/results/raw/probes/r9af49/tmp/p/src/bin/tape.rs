use simd_json::prelude::*;
use simd_json::value::lazy;
fn main(){
  let a=std::env::args().nth(1).unwrap_or_default();
  let mut b=br#"{"a":[1,2,3],"o":{"x":1,"y":2},"n":[[1,2]],"z":9}"#.to_vec();
  let t=simd_json::to_tape(&mut b).unwrap();
  let v=t.as_value();
  let arr=v.get_array("a").unwrap();
  println!("get_array a len={} get0={:?} get2={:?} iter={:?}",arr.len(),arr.get(0).and_then(|x|x.as_u64()),arr.get(2).and_then(|x|x.as_u64()),arr.iter().map(|x|x.as_u64()).collect::<Vec<_>>());
  let arr2=v.get("a").unwrap().as_array().unwrap();
  println!("via as_array len={} get2={:?} iter={:?}",arr2.len(),arr2.get(2).and_then(|x|x.as_u64()),arr2.iter().map(|x|x.as_u64()).collect::<Vec<_>>());
  let o=v.get_object("o").unwrap();
  println!("get_object o len={} y={:?} keys={:?}",o.len(),o.get("y").and_then(|x|x.as_u64()),o.keys().collect::<Vec<_>>());
  let o2=v.try_get_object("o").unwrap().unwrap(); println!("try_get_object y={:?}",o2.get("y").and_then(|x|x.as_u64()));
  let a3=v.try_get_array("a").unwrap().unwrap(); println!("try_get_array get2={:?}",a3.get(2).and_then(|x|x.as_u64()));
  let l=lazy::Value::from_tape(v);
  println!("lazy get_array get2={:?}",l.get_array("a").unwrap().get(2).and_then(|x|x.as_u64()));
  let l2=lazy::Value::from_tape(v); let mut l3=l2.clone(); l3.try_insert("z",9); println!("lazy tape==upgraded(same content) {}",l2==l3);
  let mut l4=l2.clone(); let _=l4.as_object_mut(); println!("lazy tape==upgraded(no change) is_value={} eq={}",l4.is_value(),l2==l4);
  let mut b2=br#"{"z":9,"a":[1,2,3],"o":{"y":2,"x":1},"n":[[1,2]]}"#.to_vec();
  let t2=simd_json::to_tape(&mut b2).unwrap();
  println!("tape eq reorder {}",t.as_value()==t2.as_value());
  let mut c1=vec![1u8]; let mut c2=br#"{"z":9,"a":[1,2,3],"o":{"y":2,"x":1},"n":[[1,2]]}"#.to_vec();
  let _=&mut c1;
  println!("owned eq reorder {}",simd_json::to_owned_value(&mut br#"{"a":1,"b":2}"#.to_vec()).unwrap()==simd_json::to_owned_value(&mut br#"{"b":2,"a":1}"#.to_vec()).unwrap());
  let _=&mut c2;
  // error offsets
  for s in [r#"            "abc\q""#, r#"            "abc\uZZZZ""#, r#"     ["aaaa", "bbb\x"]"#] { let mut x=s.as_bytes().to_vec(); println!("err {:?} -> {:?}",s,simd_json::to_owned_value(&mut x).map_err(|e|(e.index(),e.character(),format!("{:?}",e.error())))); }
  // error at non escape
  for s in ["[1,2,3] x","{\"a\":1} x"] {let mut x=s.as_bytes().to_vec(); println!("trail {:?} -> {:?}",s,simd_json::to_owned_value(&mut x));}
  // Buffers reuse
  let mut buf=simd_json::Buffers::new(4);
  let mut inp=br#"{"a\n":"b\n"}"#.to_vec(); let r=simd_json::to_owned_value_with_buffers(&mut inp,&mut buf); println!("{:?}",r);
  let long=format!("[\"{}\\n\"]","x".repeat(10000)); let mut inp=long.as_bytes().to_vec(); println!("{:?}",simd_json::to_owned_value_with_buffers(&mut inp,&mut buf).map(|v|v[0].as_str().unwrap().len()));
  let mut inp=b"[1]".to_vec(); println!("{:?}",simd_json::to_owned_value_with_buffers(&mut inp,&mut buf));
  // deserializer restart / as_value
  let mut inp=br#"[1,2]"#.to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut inp).unwrap(); let x:Vec<u8>=serde::Deserialize::deserialize(&mut d).unwrap(); d.restart(); let y:Vec<u8>=serde::Deserialize::deserialize(&mut d).unwrap(); println!("{:?} {:?}",x,y);
  // value conv
  let f=simd_json::OwnedValue::from(1.5f64); println!("{:?} {:?} {:?}",f.as_i64(),f.as_u8(),f.cast_f64());
  let n=simd_json::OwnedValue::from(u64::MAX); println!("{:?} {:?} {:?}",n.as_i64(),n.as_f64(),n.cast_f64());
  let n=simd_json::OwnedValue::from(-1i64); println!("{:?} {:?}",n.as_u64(),n.as_usize());
  // from_slice f32/f64 from ints
  let mut x=b"18446744073709551615".to_vec(); println!("f64 from u64max {:?}",simd_json::from_slice::<f64>(&mut x));
  let mut x=b"1e39".to_vec(); println!("f32 overflow {:?}",simd_json::from_slice::<f32>(&mut x));
  println!("  sj {:?}",serde_json::from_str::<f32>("1e39").map_err(|e|e.to_string()));
  let mut x=b"1.5".to_vec(); println!("u8 from float {:?}",simd_json::from_slice::<u8>(&mut x));
  let mut x=b"300".to_vec(); println!("u8 overflow {:?}",simd_json::from_slice::<u8>(&mut x));
  let mut x=b"\"\\n\"".to_vec(); println!("char {:?}",simd_json::from_slice::<char>(&mut x));
  let mut x=b"null".to_vec(); println!("Opt {:?}",simd_json::from_slice::<Option<u8>>(&mut x));
  let mut x=b"[]".to_vec(); println!("unit from [] {:?}",simd_json::from_slice::<()>(&mut x));
  let mut x=b"null".to_vec(); println!("bool from null {:?}",simd_json::from_slice::<bool>(&mut x));
  let mut x=br#"{"a":1}"#.to_vec(); println!("bytes key {:?}",simd_json::from_slice::<std::collections::BTreeMap<String,u8>>(&mut x));
  // Value Display
  println!("disp {}",simd_json::to_owned_value(&mut br#"{"a":[1,"x"]}"#.to_vec()).unwrap());
  println!("disp {}",simd_json::to_owned_value(&mut br#""a\"b""#.to_vec()).unwrap());
  if a=="trunc" { let mut b=br#"{"n":[[1,2]]}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value(); let arr=v.get_array("n").unwrap(); println!("nested iter..."); for x in arr.iter(){println!("{:?}",x.as_u64());} println!("get0 {:?}",arr.get(0).is_some()); }
  if a=="trunco" { let mut b=br#"{"o":{"k":[1,2]}}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value(); let o=v.get_object("o").unwrap(); println!("{:?}",o.len()); for (k,x) in o.iter(){println!("{} {:?}",k,x.as_u64());} }
}
