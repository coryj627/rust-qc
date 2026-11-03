use simd_json::prelude::*;
fn main(){ let mut b=b"[\"ab\xff\xfe\"]".to_vec(); let r=simd_json::to_borrowed_value(&mut b); match &r { Ok(v)=>{ let s=v.get_idx(0).unwrap().as_str().unwrap(); println!("Ok &str bytes={:?} valid_utf8={}",s.as_bytes(),std::str::from_utf8(s.as_bytes()).is_ok()); }, Err(e)=>println!("Err {e}") } }
