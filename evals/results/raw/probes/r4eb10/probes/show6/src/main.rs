use pulldown_cmark::*;
use std::panic;
fn show(name:&str,o:Options,s:&str){
  let s2=s.to_string();
  let r=panic::catch_unwind(move||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s2,o)); h});
  match r { Ok(h)=>println!("{:<20} {:?} => {:?}",name,s,h), Err(_)=>println!("{:<20} {:?} => PANIC",name,s) }
}
fn main(){
  panic::set_hook(Box::new(|i|{ println!("   panic at {}",i.location().unwrap()); }));
  let e=Options::empty();
  show("bq-comment",e,"> a <!-- b\n> c -->\n");
  show("bq-pi",e,"> a <? b\n> c ?>\n");
  show("bq-cdata",e,"> a <![CDATA[ b\n> c ]]>\n");
  show("bq-decl",e,"> a <!X b\n> c >\n");
  show("bq-tag",e,"> a <b c\n> d>\n");
  show("dl-bq",Options::ENABLE_DEFINITION_LIST,"> a\n>\n> : b\n");
  show("dl-plain",Options::ENABLE_DEFINITION_LIST,"a\n\n: b\n");
  let evs:Vec<Event>=Parser::new("# *a* #\n").collect(); println!("{:?}",evs);
  let evs:Vec<Event>=Parser::new("# [a](/u) #\n").filter(|e|matches!(e,Event::Text(_))).collect(); println!("{:?}",evs);
}
