use simd_json::prelude::*;
fn main(){
 let args:Vec<String>=std::env::args().collect();
 if args.len()>1 { let d:usize=args[2].parse().unwrap(); let mode=args[1].clone();
  let mut s=String::new(); if mode=="obj" { for _ in 0..d {s.push_str("{\"a\":");} s.push('1'); for _ in 0..d {s.push('}');} } else { for _ in 0..d {s.push('[');} for _ in 0..d {s.push(']');} }
  let mut b=s.into_bytes(); let n=b.len();
  let h=std::thread::spawn(move||{ match mode.as_str(){ "tapeonly"=>{let t=simd_json::to_tape(&mut b).unwrap(); println!("to_tape ok nodes={}",t.0.len());}, "tapeenc"=>{let t=simd_json::to_tape(&mut b).unwrap(); println!("to_tape ok, encoding"); println!("{}",t.as_value().encode().len());}, _=>{ println!("{}",simd_json::to_owned_value(&mut b).is_ok()); } } });
  println!("spawned input bytes={n}"); println!("join {:?}",h.join().is_ok()); return; }
 for (doc,want) in [
  (r#"{"sensor":"a1","time":1700000000,"lat":48.858369400000000000012}"#,48.858369400000000000012f64),
  (r#"{"id":1,"name":"reading","value":0.12345678901234567890123}"#,0.12345678901234567890123f64),
  (r#"{"sensor_id":"abc","pad":"xxxxxxxxxxxxxx","pi":3.14159265358979323846264}"#,3.14159265358979323846264f64),
 ]{
  let mut b=doc.as_bytes().to_vec(); let off=doc.rfind(':').unwrap()+1; let v=simd_json::to_owned_value(&mut b).unwrap(); let k=doc[..off-1].rsplit('"').nth(1).unwrap().to_string(); let got=v.get(k.as_str()).unwrap().as_f64().unwrap(); println!("{doc}\n  value at byte {off}: got {got} want {want}");
  let sj:serde_json::Value=serde_json::from_str(doc).unwrap(); println!("  serde_json {}",sj[k.as_str()]);
 }
 let mut r=1u64; let mut bad=0; let mut tot=0; for pad in 0..200 { for _ in 0..200 { r^=r<<13;r^=r>>7;r^=r<<17; let digits=20+(r%10) as usize; let mut s=String::new(); let mut x=r; s.push(char::from(b'1'+(x%9) as u8)); s.push('.'); for _ in 1..digits { x=x.wrapping_mul(6364136223846793005).wrapping_add(1); s.push(char::from(b'0'+((x>>33)%10) as u8)); } let want:f64=s.parse().unwrap(); let doc=format!("{}{}"," ".repeat(pad),s); let mut b=doc.into_bytes(); tot+=1; if simd_json::to_owned_value(&mut b).unwrap().as_f64()!=Some(want){bad+=1;} } } println!("long floats wrong {bad} of {tot} across offsets 0..200");
}
