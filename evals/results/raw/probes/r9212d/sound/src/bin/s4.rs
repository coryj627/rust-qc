use simd_json::prelude::*;
struct R(u64); impl R{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn b(&mut self,k:usize)->usize{(self.n()%k as u64) as usize}}
fn main(){ let seeds:&[&str]=&[r#"{"a":[1,2.5,-3e4,true,false,null,"x\n\"\\y\u00e9"],"b":{"c":{}},"d":[],"e":"é😀"}"#,r#"[[[[]]],{"k":"v"},0.1,1E2,-0,0e0,"\u0041\ud83d\ude00","\ud800","1234567890123456789012","\\\\\\\\\\"]"#,r#"{"long string that crosses the sixty four byte chunk boundary of the stage one parser":"value with \\ and \" and é across the boundary 0123456789012345678901234567890123456789","n":123456789012345678,"f":1.7976931348623157e308,"g":12345678901234567890,"h":1.2345678901234567890123}"#];
 let toks:&[&[u8]]=&[b"\"",b"\\",b"\\u",b"\\uD83D",b"[",b"]",b"{",b"}",b",",b":",b" ",b"0",b"1",b"-",b".",b"e",b"true",b"null",b"\xc3",b"\xa9",b"\xf0\x9f",b"\x01",b"\\n",b"12345678",b"\x00"];
 let mut r=R(0x1234567); let mut bufs=simd_json::Buffers::default(); let mut oks=0;
 for _ in 0..1500 { let mut s=seeds[r.b(3)].as_bytes().to_vec(); for _ in 0..r.b(3) { let p=r.b(s.len()+1); match r.b(3){0=>{let t=toks[r.b(toks.len())]; s.splice(p..p,t.iter().copied());},1=>{ if p<s.len(){s.remove(p);} },_=>{ if p<s.len(){s[p]=toks[r.b(toks.len())][0];} }} } if r.b(3)==0 { let pad=r.b(70); let mut x=vec![b' ';pad]; x.extend_from_slice(&s); s=x; } 
  let mut a=s.clone(); if let Ok(v)=simd_json::to_owned_value(&mut a){ oks+=1; let _=v.encode(); }
  let mut a=s.clone(); if let Ok(v)=simd_json::to_borrowed_value(&mut a){ let _=v.encode(); }
  let mut a=s.clone(); if let Ok(t)=simd_json::to_tape_with_buffers(&mut a,&mut bufs){ let _=t.as_value().encode(); let _=t.as_value().encode_pp(); }
 }
 println!("oks {oks}");
}
