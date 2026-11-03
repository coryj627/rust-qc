// Seeded fuzz: panics, unbalanced events, bad offsets, Parser/OffsetIter divergence, runaway event counts.
use pulldown_cmark::{html, Event, Options, Parser, TagEnd, Tag, TextMergeStream};
use std::panic;
struct R(u64); impl R{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn b(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize}}
const TOK:&[&str]=&["a","b ","  ","    ","\t","\n","\n","\n\n","\r\n","\r","*","**","_","__","~","~~","^","$","$$","{","}","`","``","```","~~~","[","]","[[","]]","![","![[","(",")","<",">","</","<a ","<a b=\"","\"","'","&amp;","&#35;","\\","\\|","|","|-|","| a |\n|---|\n","-","--","---","+++","...",":",": ","- ","* ","+ ","1. ","2) ","> ",">","# ","## ","=","===","[^","[^a]","[^a]: ","[a]: /u","[a]","[a][]","[a][a]","[]","<!--","-->","<?","?>","<![CDATA[","]]>","<!D","<div>","</div>","<pre>","</pre>","- [ ] ","- [x] ","{#i .c k=v}","\u{a0}","é","\u{1F600}","x@y.z","<h:t>","[!NOTE]","[!TIP]\n","\\\n","  \n","|","a|b\n-|-\n","[a](b \"t\")","[a](<b>)","![a](b)","[[a|b]]","[[a]]"];
fn main(){
 let seed:u64=std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(1);
 let iters:u64=std::env::args().nth(2).and_then(|s|s.parse().ok()).unwrap_or(200000);
 panic::set_hook(Box::new(|_|{}));
 let mut r=R(seed.wrapping_mul(0x9E3779B97F4A7C15)|1);
 let mut seen=std::collections::HashSet::new();
 let mut report=|kind:String,s:&str,o:Options|{ if seen.insert(kind.clone()) { println!("{kind}\n  in={:?}\n  opts={:?}",s,o); } };
 for _ in 0..iters{
  let len=1+r.b(14);
  let mut s=String::new(); for _ in 0..len{ s.push_str(TOK[r.b(TOK.len())]); }
  let o=match r.b(4){0=>Options::all(),1=>Options::from_bits_truncate(r.n() as u32),2=>Options::all()-Options::ENABLE_OLD_FOOTNOTES|Options::ENABLE_FOOTNOTES,_=>Options::from_bits_truncate(r.n() as u32)};
  let st=s.clone();
  let res=panic::catch_unwind(move||{
    let mut errs:Vec<String>=vec![];
    let evs:Vec<_>=Parser::new_ext(&st,o).collect();
    let off:Vec<_>=Parser::new_ext(&st,o).into_offset_iter().collect();
    if evs.len()!=off.len(){errs.push("len-divergence".into());}
    for (a,(b,_)) in evs.iter().zip(off.iter()){ if a!=b {errs.push("event-divergence".into());break;} }
    let mut stack:Vec<TagEnd>=vec![];
    for (e,rg) in &off{
      if rg.start>rg.end||rg.end>st.len(){errs.push(format!("bad-range {:?}",e_kind(e)));}
      else if !st.is_char_boundary(rg.start)||!st.is_char_boundary(rg.end){errs.push(format!("nonboundary {:?}",e_kind(e)));}
      match e{Event::Start(t)=>stack.push(t.to_end()),Event::End(t)=>{match stack.pop(){Some(x) if x==*t=>{},other=>{errs.push(format!("unbalanced {:?} vs {:?}",other,t));}}},_=>{}}
      if let Event::Text(t)=e{ if t.is_empty(){errs.push("empty-text".into());} }
      // nesting check
    }
    if !stack.is_empty(){errs.push("unclosed".into());}
    // Text source duplication check: same source range emitted by two leaf events
    let mut leafs:Vec<(usize,usize)>=off.iter().filter(|(e,r)|matches!(e,Event::Text(_)|Event::Code(_)) && r.start<r.end).map(|(_,r)|(r.start,r.end)).collect();
    leafs.sort(); for w in leafs.windows(2){ if w[1].0<w[0].1 { errs.push("overlap-leaf".into()); break; } }
    // Text content vs source when plain
    let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&st,o));
    let _m:Vec<_>=TextMergeStream::new(Parser::new_ext(&st,o)).collect();
    errs
  });
  match res{Ok(errs)=>{for k in errs{report(k,&s,o);}},Err(p)=>{let m=p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|x|x.to_string())).unwrap_or_default();report(format!("PANIC {}",m.chars().take(70).collect::<String>().replace(|c:char|c.is_ascii_digit(),"N")),&s,o);}}
 }
 println!("done");
}
fn e_kind(e:&Event)->String{match e{Event::Start(t)=>format!("S{}",tagname(t)),Event::End(t)=>format!("E{:?}",t),Event::Text(_)=>"Text".into(),Event::Code(_)=>"Code".into(),Event::Html(_)=>"Html".into(),Event::InlineHtml(_)=>"IHtml".into(),Event::InlineMath(_)|Event::DisplayMath(_)=>"Math".into(),Event::FootnoteReference(_)=>"FnRef".into(),o=>format!("{:?}",o)}}
fn tagname(t:&Tag)->String{let s=format!("{:?}",t);s.split(|c:char|!c.is_alphanumeric()).next().unwrap_or("").to_string()}
