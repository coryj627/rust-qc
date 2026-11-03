use pulldown_cmark::*; use std::time::Instant;
fn show(s:&str,o:Options,off:bool){
 let s2=s.to_string();
 let r=std::panic::catch_unwind(move||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s2,o)); h});
 println!("--- {:?} opts={:#x}",s,o.bits());
 match r { Ok(h)=>println!("{:?}",h), Err(e)=>{println!("PANIC {:?}",e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|x|x.to_string())));return;} }
 if off { for (e,r) in Parser::new_ext(s,o).into_offset_iter(){ println!("   {:?} {:?}={:?}",e,r,s.get(r.clone())); } }
}
fn tm(name:&str,o:Options,f:&dyn Fn(usize)->String,base:usize){ let mut n=base; for _ in 0..5 { let s=f(n); let st=Instant::now(); let c=Parser::new_ext(&s,o).count(); let d=st.elapsed().as_secs_f64(); println!("{} n={} bytes={} ev={} t={:.3}",name,n,s.len(),c,d); if d>5.0 {break;} n*=2; } }
fn main(){
 let e=Options::empty(); let w=Options::ENABLE_WIKILINKS;let st=Options::ENABLE_STRIKETHROUGH;
 show("[a]\\[b]\n\n[a]: /a\n[b]: /b",e,false); show("[a]\\[]\n\n[a]: /a",e,false); show("![a]\\[b]\n\n[a]: /a\n[b]: /b",e,false); show("[x]\\[b]\n\n[b]: /b",e,false);
 show("![x [[a]]](u) tail",w,false); show("[y ![x [[a]]](u) tail](v)",w,false);
 show("~~a~ b~~",st,false); show("~~a b~~",st,false); show("~a~~ b~",st,false);
 show("~a^ b~",Options::ENABLE_SUBSCRIPT|Options::ENABLE_SUPERSCRIPT,false); show("~a b~",Options::ENABLE_SUBSCRIPT|Options::ENABLE_SUPERSCRIPT,false);
 tm("*a_",e,&|n|"*a_ ".repeat(n),40000);
 tm("yaml",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,&|n|"---\t\na\n\n".repeat(n),5000);
 tm("title",e,&|n|format!("[a](b \"{}","x\n".repeat(n)),20000);
}
