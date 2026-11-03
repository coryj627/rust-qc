use pulldown_cmark::*;
use std::panic;
use std::io::Write;
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
struct FailW; impl Write for FailW { fn write(&mut self,_:&[u8])->std::io::Result<usize>{Err(std::io::Error::other("disk full"))} fn flush(&mut self)->std::io::Result<()>{Err(std::io::Error::other("disk full"))} }
fn main(){
  panic::set_hook(Box::new(|i|{ println!("   panic at {}",i.location().unwrap()); }));
  let e=Options::empty();
  show("cdata-inner-bracket",e,"foo <![CDATA[a]b]]> bar");
  show("cdata-plain",e,"foo <![CDATA[ab]]> bar");
  show("cdata-first-fail-second",e,"<![CDATA[a] <![CDATA[b]]>");
  show("pre-upper-close",e,"<PRE>\nfoo\n</PRE>\n\n*bar*\n");
  show("script-upper-close",e,"<script>\nfoo\n</SCRIPT>\n\n*bar*\n");
  show("style-other-close",e,"<style>\nfoo\n</script>\n\n*bar*\n");
  show("close-tag-newline",e,"foo </a\n>");
  show("meta-tag",e,"<meta charset=x> foo\n\nbar\n");
  show("div-tag",e,"<div charset=x> foo\n\nbar\n");
  let t=Options::ENABLE_TABLES;
  show("tbl-title-pipe-a",t,"|h|\n|-|\n|[a](/u \"x\\\\|y\")|\n");
  show("tbl-title-pipe-p",t,"|h|\n|-|\n|[a](/u \"x\\\\|!\")|\n");
  // table cell budget
  { let cols=1000; let mut s=String::new(); s.push_str(&"|h".repeat(cols)); s.push_str("|\n"); s.push_str(&"|-".repeat(cols)); s.push_str("|\n"); s.push_str(&"|c|\n".repeat(300)); s.push_str("\nafter table\n");
    let mut depth=0; let mut maxd=0; let mut seq=vec![]; for ev in Parser::new(&s).into_iter(){ let _=ev; }
    let o=t; let evs:Vec<Event>=Parser::new_ext(&s,o).collect();
    for ev in &evs{ match ev { Event::Start(tag)=>{depth+=1; if depth>maxd{maxd=depth} if !matches!(tag,Tag::TableCell|Tag::TableRow){seq.push(format!("S{:?}@{}",std::mem::discriminant(tag),depth));}}, Event::End(t)=>{depth-=1; if !matches!(t,TagEnd::TableCell|TagEnd::TableRow){seq.push(format!("E{:?}@{}",t,depth));}}, Event::Text(x) if x.as_ref()!="c"&&x.as_ref()!="h"=>seq.push(format!("T{:?}@{}",x,depth)), _=>{} } }
    println!("tbl-budget: maxdepth={} events={} tail={:?}",maxd,evs.len(),&seq[seq.len().saturating_sub(8)..]);
    let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s,o)); println!("   html tail {:?}",&h[h.len()-160..]);
    let rows=h.matches("<tr>").count(); println!("   rows={}",rows);
  }
  // io flush
  { let w=std::io::BufWriter::new(FailW); let r=html::write_html_io(w,Parser::new("hello")); println!("bufwriter-failing -> {:?}",r);
    let r=html::write_html_io(FailW,Parser::new("hello")); println!("direct-failing -> {:?}",r); }
  // text merge
  { let evs=vec![Event::Rule,Event::Text("".into()),Event::Rule]; println!("merge-single-empty {:?}",TextMergeStream::new(evs.into_iter()).collect::<Vec<_>>()); 
    let evs=vec![Event::Text("".into())]; println!("merge-only-empty {:?}",TextMergeStream::new(evs.into_iter()).collect::<Vec<_>>());
    let evs=vec![Event::Text("".into()),Event::Text("".into())]; println!("merge-two-empty {:?}",TextMergeStream::new(evs.into_iter()).collect::<Vec<_>>());}
  ev("dl-boundary",Options::ENABLE_TABLES|Options::ENABLE_DEFINITION_LIST|e,"|\n:<!c>\u{2003}\n\t1.");
  ev("dl-boundary2",Options::ENABLE_DEFINITION_LIST,"a\n:<!c>\u{2003}\n\t1.");
  ev("dl-html",Options::ENABLE_DEFINITION_LIST,"a\n:<!c>é\n\tb\n");
  ev("dl-html-ascii",Options::ENABLE_DEFINITION_LIST,"a\n:<!c>xyz\n\tb\n");
  ev("dl-code",Options::ENABLE_DEFINITION_LIST,"a\n:```\n\tb\n");
  ev("list-tab-html",e,"- <!c>xyz\n\tb\n");
  ev("list-1col-html",e,"a\n\n-\t<!c\n\tb>\n");
  ev("empty-para",e,"[a]: /u\n\u{c}");
}
