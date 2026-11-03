use pulldown_cmark::*;
use std::panic;
use std::time::Instant;
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
fn time(name:&str,o:Options,g:&dyn Fn(usize)->String){ print!("{:<20}",name); let mut n=2000; loop{ let s=g(n); let t=Instant::now(); let c=Parser::new_ext(&s,o).count(); let d=t.elapsed().as_secs_f64(); print!(" {}KB:{:.3}s",s.len()/1024,d); let _=c; if d>3.0||n>2_000_000{break} n*=2;} println!(); }
fn main(){
  panic::set_hook(Box::new(|i|{ println!("   panic at {}",i.location().unwrap()); }));
  let e=Options::empty();
  show("natural-1",e,"- [a]: /u\n      \n");
  show("natural-2",e,"- [a]: /u\n      \n- b\n");
  show("natural-3",e,"- [a]: /u\n\n      \n- b\n");
  show("natural-4",e,"* item\n* [a]: /u\n        \n");
  show("natural-5",e,"[a]: /u\n      \nfoo\n");
  show("natural-6",e,"> - [a]: /u\n>       \n");
  show("natural-7",e,"[a]: /u\n      \n");
  show("hb-tabs",e,"a\t\t\nb");
  show("hb-sp-tab",e,"a \t\nb");
  show("hb-ff",e,"a\u{c}\u{c}\nb");
  show("hb-spaces",e,"a  \nb");
  show("close-tag-nl",e,"foo </a\n    >");
  show("open-tag-nl",e,"foo <a\n    >");
  let w=Options::ENABLE_WIKILINKS;
  show("img-with-wiki",w,"![x [[a]] y](/u)");
  show("img-with-link",w,"![x [a](/b) y](/u)");
  show("wiki-then-bracket",w,"[[a]] b]");
  ev("tab-offset-default",e,"- é\n\t- b\n");
  ev("tab-offset-ascii",e,"- a\n\t- b\n");
  { let s="- é\n\t- b\n"; for (_,r) in Parser::new(s).into_offset_iter(){ let s2=s.to_string(); let r2=r.clone(); let ok=panic::catch_unwind(move||s2[r2].len()).is_ok(); if !ok {println!("   slice {:?} panics",r);} } }
  let y=Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
  time("meta-bq-hr",y,&|n|"> ---\n".repeat(n/6));
  time("meta-ol-hr",y,&|n|"> ---\n\n".repeat(n/7));
  time("plus",Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,&|n|"> +++\n\n".repeat(n/7));
  // punct table via flanking
  let mut out=String::new();
  for cp in 0x80u32..0x110000 { if let Some(c)=char::from_u32(cp){ if c.is_whitespace(){continue} let s=format!("x*{}a*",c); let em=Parser::new(&s).any(|e|matches!(e,Event::Start(Tag::Emphasis))); if !em { out.push_str(&format!("{:x}\n",cp)); } } }
  std::fs::write("/work/probes/data/punct.txt",out).unwrap();
  // ascii
  let mut asc=String::new(); for b in 0x21u8..0x7f { let c=b as char; if c.is_ascii_alphanumeric(){continue} let s=format!("\\{}",c); let h:String=Parser::new(&s).filter_map(|e| if let Event::Text(t)=e {Some(t.to_string())} else {None}).collect(); if h!=c.to_string(){asc.push(c);} } println!("ascii not escapable: {:?}",asc);
  // entities
  let names=std::fs::read_to_string("/work/probes/data/ents.txt").unwrap(); let mut out=String::new();
  for n in names.lines(){ let s=format!("a&{};b",n); let t:String=Parser::new(&s).filter_map(|e| if let Event::Text(t)=e {Some(t.to_string())} else {None}).collect(); let hex:Vec<String>=t.chars().map(|c|format!("{:x}",c as u32)).collect(); out.push_str(&format!("{} {}\n",n,hex.join(","))); }
  std::fs::write("/work/probes/data/ents_out.txt",out).unwrap();
}
