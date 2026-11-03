use pulldown_cmark::*; use std::time::Instant;
fn main(){ for (s,o) in [("~~a~ b~~",Options::ENABLE_STRIKETHROUGH),("~a^ b~",Options::ENABLE_SUBSCRIPT|Options::ENABLE_SUPERSCRIPT)] { let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(s,o)); println!("{:?} -> {:?}",s,h);} 
 for n in [40000,160000,640000] { let s="*a_ ".repeat(n); let st=Instant::now(); let c=Parser::new(&s).count(); println!("patched *a_ n={} ev={} t={:.3}",n,c,st.elapsed().as_secs_f64()); } }
