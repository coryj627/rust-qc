use pulldown_cmark::*;
fn main(){ let mut p=String::new(); let mut w=String::new();
 for u in 0x80u32..0x110000{ let Some(c)=char::from_u32(u) else{continue}; 
   let s=format!("a*{c}b*"); let mut h=String::new(); html::push_html(&mut h,Parser::new(&s)); if !h.contains("<em>"){ p.push_str(&format!("{u:x}\n")); }
   let s=format!("*{c}b*"); let mut h=String::new(); html::push_html(&mut h,Parser::new(&s)); if !h.contains("<em>"){ w.push_str(&format!("{u:x}\n")); } }
 std::fs::write("/work/tmp/punct.txt",p).unwrap(); std::fs::write("/work/tmp/ws.txt",w).unwrap();
 let cases=[("[\u{212A}]: /u\n\n[k]\n"),("[k]: /u\n\n[\u{212A}]\n"),("[\u{17F}]: /u\n\n[S]\n"),("[ß]: /u\n\n[SS]\n"),("[Σ]: /u\n\n[σ]\n[ς]\n")]; for c in cases{ let mut h=String::new(); html::push_html(&mut h,Parser::new(c)); println!("{c:?} {h:?}"); }
 let mut o=Options::empty(); o.insert(Options::ENABLE_FOOTNOTES); let mut h=String::new(); html::push_html(&mut h,Parser::new_ext("[^\u{212A}]\n\n[^k]: x\n",o)); println!("{h:?}");
}
