#![allow(dead_code)]
mod pt { include!("../pt_copy.rs"); pub fn f(c:char)->bool{ is_punctuation(c) } pub fn a(c:u8)->bool{ is_ascii_punctuation(c) } }
fn main(){ let mut s=String::new(); for u in 0..0x110000u32 { if let Some(c)=char::from_u32(u){ if pt::f(c){ s.push_str(&format!("{}\n",u)); } } } std::fs::write("/work/tmp/punct.txt",s).unwrap(); let a:Vec<u8>=(0u8..128).filter(|&b|pt::a(b)).collect(); println!("ascii {:?}",String::from_utf8(a)); }
