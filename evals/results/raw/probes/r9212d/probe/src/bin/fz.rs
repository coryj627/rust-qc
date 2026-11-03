use simd_json::prelude::*;
use std::panic::{catch_unwind,AssertUnwindSafe};
struct R(u64); impl R{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn b(&mut self,k:usize)->usize{(self.n()%k as u64) as usize}}
fn norm(v:&serde_json::Value)->String{ match v { serde_json::Value::Number(n)=>format!("{}",n.as_f64().unwrap_or(0.0)), serde_json::Value::Array(a)=>format!("[{}]",a.iter().map(norm).collect::<Vec<_>>().join(",")), serde_json::Value::Object(o)=>format!("{{{}}}",o.iter().map(|(k,v)|format!("{k:?}:{}",norm(v))).collect::<Vec<_>>().join(",")), x=>x.to_string() } }
fn check(input:&[u8],stats:&mut (u64,u64,u64,u64),shown:&mut usize,bufs:&mut simd_json::Buffers){
  stats.0+=1;
  let sv:Result<serde_json::Value,_>=serde_json::from_slice(input);
  let r=catch_unwind(AssertUnwindSafe(||{
    let mut b=input.to_vec(); let o=simd_json::to_owned_value(&mut b).map(|v|v.encode()).map_err(|e|format!("{:?}",e.error()));
    let mut b2=input.to_vec(); let bv=simd_json::to_borrowed_value(&mut b2).map(|v|v.encode()).map_err(|e|format!("{:?}",e.error()));
    let mut b3=input.to_vec(); let t=simd_json::to_tape(&mut b3).map(|t|t.as_value().encode()).map_err(|e|format!("{:?}",e.error()));
    let mut b4=input.to_vec(); let wb=simd_json::to_tape_with_buffers(&mut b4,bufs).map(|t|t.as_value().encode()).map_err(|e|format!("{:?}",e.error()));
    let mut b5=input.to_vec(); let sd=simd_json::from_slice::<serde_json::Value>(&mut b5).map(|v|v.to_string()).map_err(|e|format!("{:?}",e.error()));
    (o,bv,t,wb,sd)
  }));
  let Ok((o,bv,t,wb,sd))=r else { stats.3+=1; if *shown<40 {*shown+=1; println!("PANIC {:?}",String::from_utf8_lossy(input));} return; };
  if o.is_ok()!=bv.is_ok()||o.is_ok()!=t.is_ok()||o.is_ok()!=wb.is_ok()||o.is_ok()!=sd.is_ok() || (o.is_ok()&&(wb!=t)) { if *shown<40 {*shown+=1; println!("PATH DIFF {:?} o={o:?} bv={bv:?} t={t:?} wb={wb:?} sd={sd:?}",String::from_utf8_lossy(input));} }
  let known=|s:&[u8]| s.contains(&0) || s.windows(2).any(|w|w==b"\\u"||w==b"\\U") || s.windows(17).any(|w|w.iter().all(|c|c.is_ascii_digit()||*c==b'.'));
  match (&o,&sv){
    (Ok(_),Err(_)) if !known(input)=>{stats.1+=1; if *shown<40 {*shown+=1; println!("SIMD ACCEPTS {:?}: {o:?}",String::from_utf8_lossy(input));}}
    (Err(e),Ok(_)) if !known(input)=>{stats.2+=1; if *shown<40 {*shown+=1; println!("SIMD REJECTS {:?}: {e}",String::from_utf8_lossy(input));}}
    (Ok(a),Ok(b)) if !known(input)=>{ let av:serde_json::Value=match serde_json::from_str(a){Ok(v)=>v,Err(e)=>{ if *shown<40 {*shown+=1; println!("ENCODE BAD {:?}: {a} {e}",String::from_utf8_lossy(input));} return;}}; if norm(&av)!=norm(b) { if *shown<40 {*shown+=1; println!("VALUE DIFF {:?}: simd={a} serde={b}",String::from_utf8_lossy(input));} } }
    _=>{}
  }
}
fn main(){
  std::panic::set_hook(Box::new(|_|{}));
  let mut st=(0,0,0,0); let mut sh=0; let mut bufs=simd_json::Buffers::default();
  // exhaustive short
  let alpha:&[u8]=b"[]{}\",:0-1.eEtfn\\ u\xc3\xa9\x00";
  for len in 0..=4 { let mut idx=vec![0usize;len]; loop { let s:Vec<u8>=idx.iter().map(|&i|alpha[i]).collect(); check(&s,&mut st,&mut sh,&mut bufs); let mut p=0; while p<len { idx[p]+=1; if idx[p]<alpha.len(){break;} idx[p]=0; p+=1;} if p==len {break;} } }
  println!("after exhaustive {:?}",st);
  // mutations
  let seeds:&[&str]=&[r#"{"a":[1,2.5,-3e4,true,false,null,"x\n\"\\y\u00e9"],"b":{"c":{}},"d":[],"e":"é😀"}"#,r#"[[[[]]],{"k":"v"},0.1,1E2,-0,0e0,"\u0041\ud83d\ude00"]"#,
    r#"{"long string that crosses the sixty four byte chunk boundary of the stage one parser":"value with \\ and \" and é across the boundary 0123456789012345678901234567890123456789","n":123456789012345678,"f":1.7976931348623157e308}"#];
  let mut r=R(0x9E3779B97F4A7C15);
  let toks:&[&[u8]]=&[b"\"",b"\\",b"\\u",b"[",b"]",b"{",b"}",b",",b":",b" ",b"\n",b"0",b"1",b"-",b".",b"e",b"E",b"+",b"true",b"false",b"null",b"\xc3",b"\xa9",b"\xf0\x9f",b"\t",b"\x01",b"\\n",b"12345678",b"\x7f"];
  let n:u64=std::env::args().nth(1).and_then(|a|a.parse().ok()).unwrap_or(1_000_000);
  for _ in 0..n { let mut s=seeds[r.b(seeds.len())].as_bytes().to_vec(); for _ in 0..1+r.b(4) { let p=r.b(s.len()+1); match r.b(3){0=>{let t=toks[r.b(toks.len())]; s.splice(p..p,t.iter().copied());},1=>{ if p<s.len(){s.remove(p);} },_=>{ if p<s.len(){s[p]=toks[r.b(toks.len())][0];} }} } if r.b(4)==0 { let pad=r.b(70); let mut x=vec![b' ';pad]; x.extend_from_slice(&s); s=x; } check(&s,&mut st,&mut sh,&mut bufs); }
  println!("total inputs,accepts,rejects,panics {:?}",st);
}
