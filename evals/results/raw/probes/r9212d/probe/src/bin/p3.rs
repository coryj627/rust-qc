use simd_json::prelude::*;
fn own(s:&[u8])->String{ let mut b=s.to_vec(); match simd_json::to_owned_value(&mut b){Ok(v)=>format!("OK {:?}",v),Err(e)=>format!("ERR {:?} idx {}",e.error(),e.index())} }
fn main(){
  for c in [&b"1\0abc"[..],b"[1\0,2]",b"[true\0]",b"{\"a\":null\0}",b"1\0\0\0\0\0",b"[1\0x]",b"true\0garbage",b"-1\0x",b"1.5\0x",b"null\0x",b"\"a\"\0x", b"\"a\"\0"] { println!("{:?} => {}", String::from_utf8_lossy(c), own(c)); }
  for c in ["0e99999999999","1e-99999999999","0.0e4294967296","1.0E+2","1E400","1e-320","123456789012345678901234567890.5","0.1e1","-1.5E-3", "1.0000000000000000000000000000000000000000000000000000000001","0.00000000000000000000000000000000000001e38","100000000000000000000e-20","9007199254740993.0","9007199254740993e0"] { let s:Option<f64>=c.parse().ok(); println!("{c} => {} (std {:?})",own(c.as_bytes()),s); }
  // long strings across boundaries
  for n in [15,16,17,31,32,33,63,64,65,127,128,129,1000] {
    let body="é".repeat(n); let s=format!("[\"{}\\n{}\\\"\"]",body,body); let want:serde_json::Value=serde_json::from_str(&s).unwrap(); let mut b=s.into_bytes(); let v=simd_json::to_owned_value(&mut b).unwrap(); let got:serde_json::Value=serde_json::from_str(&v.encode()).unwrap(); if got!=want {println!("string mismatch n={n}");}
  }
  // backslash runs across 64-byte boundary
  for pad in 0..70 { for runs in 1..8 { let s=format!("{}\"{}\"",".".repeat(0)+&" ".repeat(pad),"\\\\".repeat(runs)); let want:serde_json::Value=serde_json::from_str(&s).unwrap(); let mut b=s.clone().into_bytes(); let v=simd_json::to_owned_value(&mut b).unwrap(); if v.as_str()!=want.as_str() {println!("bs mismatch {s}");} } }
  // depth
  let args:Vec<String>=std::env::args().collect();
  if args.len()>1 { let d:usize=args[2].parse().unwrap(); let mut s=String::new(); for _ in 0..d {s.push('[');} for _ in 0..d {s.push(']');} let mut b=s.into_bytes();
    match args[1].as_str(){
     "tape"=>{let t=simd_json::to_tape(&mut b).unwrap(); println!("tape ok {}",t.0.len()); let e=t.as_value().encode(); println!("enc {}",e.len());}
     "owned"=>{let v=simd_json::to_owned_value(&mut b); println!("owned ok {}",v.is_ok());}
     "borrowed"=>{let v=simd_json::to_borrowed_value(&mut b); println!("borrowed ok {}",v.is_ok());}
     "serde"=>{let v:Result<simd_json::OwnedValue,_>=simd_json::from_slice(&mut b); println!("serde ok {}",v.is_ok());}
     _=>{}
    }
  }
}
