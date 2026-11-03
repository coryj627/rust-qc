use simd_json::prelude::*;
struct R(u64); impl R{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn b(&mut self,k:u64)->u64{self.n()%k}}
fn own(s:&str)->String{ let mut b=s.as_bytes().to_vec(); match simd_json::to_owned_value(&mut b){Ok(v)=>format!("OK {:?}",v),Err(e)=>format!("ERR {:?} idx {}",e.error(),e.index())} }
fn main(){
 for c in ["18446744073709551616","-9223372036854775809","-18446744073709551616","340282366920938463463374607431768211455","340282366920938463463374607431768211456","-170141183460469231731687303715884105728","-170141183460469231731687303715884105729","12345678901234567890123","123456789012345678901234567890abc","1e400","1.5","0.1","1E22","1e23","9007199254740993","[-9223372036854775809,1]","-0","100000000000000000000000000000000000000000","1"] { println!("{c} => {}",own(c)); }
 let mut r=R(0xDEADBEEFCAFEF00D); let mut bad=0u64; let mut total=0u64; let mut shown=0;
 for _ in 0..3_000_000 {
  let digits=1+r.b(22); let mut s=String::new(); if r.b(2)==0{s.push('-');}
  let ip=1+r.b(digits); for i in 0..digits { if i==ip {s.push('.');} let d= if i==0 {1+r.b(9)} else {r.b(10)}; s.push(char::from(b'0'+d as u8)); }
  if !s.contains('.') { s.push_str(".0"); }
  if r.b(2)==0 { let e=r.b(640) as i64-330; s.push_str(&format!("e{e}")); }
  let want:f64=s.parse().unwrap(); if want.is_infinite(){continue;}
  total+=1;
  let mut b=s.clone().into_bytes(); match simd_json::to_owned_value(&mut b){ Ok(v)=>{ let got=v.as_f64().unwrap(); if got.to_bits()!=want.to_bits(){bad+=1; if shown<15 {shown+=1; println!("float diff {s}: got {got:e} want {want:e}");}} }, Err(e)=>{bad+=1; if shown<15{shown+=1; println!("float err {s}: {e}");}} }
 }
 println!("float differential: {bad} bad of {total}");
 // 128/ints differential
 let mut bad=0; for _ in 0..1000000 { let mut s=String::new(); let neg=r.b(2)==0; if neg {s.push('-');} let d=1+r.b(20); for i in 0..d { let c= if i==0 {1+r.b(9)} else {r.b(10)}; s.push(char::from(b'0'+c as u8)); } let mut b=s.clone().into_bytes(); let g=simd_json::to_owned_value(&mut b); let ok= if neg { s.parse::<i64>().ok().map(|w| g.as_ref().ok().and_then(|v|v.as_i64())==Some(w)) } else { s.parse::<u64>().ok().map(|w| g.as_ref().ok().and_then(|v|v.as_u64())==Some(w)) }; if ok==Some(false) {bad+=1; if bad<10 {println!("int diff {s} {g:?}");}} }
 println!("int differential bad {bad}");
}
