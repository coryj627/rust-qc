use pulldown_cmark::*;
use std::panic;
fn show(name:&str,o:Options,s:&str){
  let s2=s.to_string();
  let r=panic::catch_unwind(move||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s2,o)); h});
  match r { Ok(h)=>println!("{:<26} {:?}\n   => {:?}",name,s,h), Err(_)=>println!("{:<26} {:?}\n   => PANIC",name,s) }
}
fn ev(name:&str,o:Options,s:&str){
  println!("EV {} {:?}",name,s);
  let s2=s.to_string();
  let _=panic::catch_unwind(move||{ for (e,r) in Parser::new_ext(&s2,o).into_offset_iter(){ println!("   {:?} {:?} src={:?}",r,e,s2.get(r.clone())); } });
}
fn main(){
  panic::set_hook(Box::new(|i|{ println!("   panic at {}",i.location().unwrap()); }));
  let e=Options::empty();
  ev("bq-tab",e,">\t- b\n");
  let dl=Options::ENABLE_DEFINITION_LIST;
  ev("dl-tab-midchar",dl,"a\n: é\n\t- b\n");
  { let s="a\n: é\n\t- b\n"; for (_,r) in Parser::new_ext(s,dl).into_offset_iter(){ let s2=s.to_string(); let r2=r.clone(); if panic::catch_unwind(move||s2[r2].len()).is_err() {println!("   slicing src[{:?}] panics",r);} } }
  let h=Options::ENABLE_HEADING_ATTRIBUTES;
  show("attr-one-char",h,"# Intro {x}\n");
  show("attr-two-char",h,"# Intro {xy}\n");
  show("attr-empty-key",h,"# Intro {=v}\n");
  let f=Options::ENABLE_FOOTNOTES;
  show("fn-case2",f,"see [^Note] and [^note]\n\n[^NOTE]: y\n");
  show("code-crlf3",e,"``a\r\nb\r\nc``");
  show("fence-crlf",e,"```\r\na\r\n```\r\n");
  show("code-cr",e,"~~~\na\rb\n~~~\n");
  show("indent-cr",e,"    a\rb\n");
  show("setext-cr",e,"a\r===\r");
  show("para-cr",e,"a\rb\r");
  show("list-cr",e,"- a\r- b\r");
  show("hr-cr",e,"---\r***\r");
  show("html-cr",e,"<div>\rfoo\r\rbar\r");
  show("html-comment-cr",e,"<!-- a -->\rfoo\r");
  show("tbl-tab-closer",Options::ENABLE_TABLES,"|a|\t\n|-|\t\n|b|\t\n");
  show("tbl-crlf",Options::ENABLE_TABLES,"|a|\r\n|-|\r\n|b|\r\n");
  show("sup-sub-tilde-open",Options::ENABLE_SUPERSCRIPT|Options::ENABLE_STRIKETHROUGH,"^a ~~ b^");
  show("sup-strike-close",Options::ENABLE_SUPERSCRIPT|Options::ENABLE_STRIKETHROUGH,"^a~~ b^");
  show("metadata-tab-closer",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,"---\na\n---\t\nb\n");
  show("meta-bq-closer-outside",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,"> ---\n> a\n---\nb\n");
}
