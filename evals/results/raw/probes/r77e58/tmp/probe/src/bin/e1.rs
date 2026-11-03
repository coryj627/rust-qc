// Exhaustive token enumeration with structural validation of the event stream.
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use std::collections::BTreeMap;
const TOK:&[&str]=&["\n"," ","    ","- ","> ","[a]: /u","a","|","-","[^a]: ",": ","\t","\r\n","*","`","[","]","<","# ","=","$","~","^","\\","(",")","!","_","x|y\n-|-\n","{#a}","[[","]]","1. ","[x] ","---\n","```\n","\"t\"","<b>"];
fn is_block(t:&Tag)->bool{matches!(t,Tag::Paragraph|Tag::Heading{..}|Tag::BlockQuote(_)|Tag::CodeBlock(_)|Tag::HtmlBlock|Tag::List(_)|Tag::Item|Tag::FootnoteDefinition(_)|Tag::DefinitionList|Tag::DefinitionListTitle|Tag::DefinitionListDefinition|Tag::Table(_)|Tag::TableHead|Tag::TableRow|Tag::TableCell|Tag::MetadataBlock(_))}
fn name(t:&Tag)->&'static str{match t{Tag::Paragraph=>"P",Tag::Heading{..}=>"H",Tag::BlockQuote(_)=>"BQ",Tag::CodeBlock(_)=>"CB",Tag::HtmlBlock=>"HB",Tag::List(_)=>"L",Tag::Item=>"LI",Tag::FootnoteDefinition(_)=>"FD",Tag::DefinitionList=>"DL",Tag::DefinitionListTitle=>"DT",Tag::DefinitionListDefinition=>"DD",Tag::Table(_)=>"T",Tag::TableHead=>"TH",Tag::TableRow=>"TR",Tag::TableCell=>"TC",Tag::MetadataBlock(_)=>"MB",Tag::Emphasis=>"em",Tag::Strong=>"st",Tag::Strikethrough=>"del",Tag::Superscript=>"sup",Tag::Subscript=>"sub",Tag::Link{..}=>"a",Tag::Image{..}=>"img"}}
fn check(s:&str,o:Options,out:&mut BTreeMap<String,(usize,String,String)>){
  let st=s.to_string();
  let r=std::panic::catch_unwind(move||{
    let mut errs:Vec<String>=vec![];
    let v:Vec<_>=Parser::new_ext(&st,o).into_offset_iter().collect();
    let mut stack:Vec<(&'static str,std::ops::Range<usize>,usize)>=vec![];
    for (i,(e,r)) in v.iter().enumerate(){
      if r.start>r.end||r.end>st.len(){errs.push("range-invalid".into());}
      if let Some(p)=stack.last(){ if r.start<p.1.start||r.end>p.1.end { if !matches!(e,Event::End(_)){ errs.push(format!("child-range-outside:{}>{}",p.0,kind(e))); } } }
      match e{
       Event::Start(t)=>{
         let n=name(t);
         if let Some(p)=stack.last(){
           let inl=matches!(p.0,"P"|"H"|"em"|"st"|"del"|"sup"|"sub"|"a"|"img"|"DT"|"TC");
           if inl && is_block(t){errs.push(format!("block-in-inline:{}>{}",p.0,n));}
           let ok=match p.0{"T"=>matches!(n,"TH"|"TR"),"TH"|"TR"=>n=="TC","L"=>n=="LI","DL"=>matches!(n,"DT"|"DD"),"CB"|"HB"|"MB"=>false,_=>true};
           if !ok{errs.push(format!("bad-child:{}>{}",p.0,n));}
         }
         if n=="LI"&&stack.last().map(|p|p.0)!=Some("L"){errs.push("item-outside-list".into());}
         if matches!(n,"TC"|"TR"|"TH")&&!matches!(stack.last().map(|p|p.0),Some("T"|"TH"|"TR")){errs.push("cell-outside".into());}
         if matches!(n,"P"|"em"|"st"|"del"|"sup"|"sub") { if let Some((Event::End(_),_))=v.get(i+1){errs.push(format!("empty-{}",n));} }
         if n=="P"{ if let Some((Event::HardBreak|Event::SoftBreak,_))=v.get(i+1){errs.push("para-starts-with-break".into());} }
         stack.push((n,r.clone(),i));
       }
       Event::End(_)=>{stack.pop();}
       Event::Text(t)=>{ if t.is_empty(){errs.push("empty-text".into());} }
       Event::SoftBreak|Event::HardBreak=>{ if let Some((Event::End(_),_))=v.get(i+1){errs.push("break-at-end".into());} }
       Event::TaskListMarker(_)=>{}
       _=>{}
      }
    }
    let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&st,o));
    errs
  });
  match r{Ok(errs)=>{for k in errs{add(out,k,s,o);}},Err(p)=>{let m=p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|x|x.to_string())).unwrap_or_default();add(out,format!("PANIC {}",m.chars().take(60).collect::<String>().replace(|c:char|c.is_ascii_digit(),"N")),s,o);}}
}
fn kind(e:&Event)->String{match e{Event::Start(t)=>name(t).into(),Event::Text(_)=>"Text".into(),Event::Code(_)=>"Code".into(),Event::Html(_)=>"Html".into(),Event::InlineHtml(_)=>"IHtml".into(),Event::InlineMath(_)|Event::DisplayMath(_)=>"Math".into(),Event::FootnoteReference(_)=>"FnRef".into(),Event::SoftBreak=>"SB".into(),Event::HardBreak=>"HBk".into(),Event::Rule=>"Rule".into(),Event::TaskListMarker(_)=>"Task".into(),Event::End(_)=>"End".into()}}
fn add(out:&mut BTreeMap<String,(usize,String,String)>,k:String,s:&str,o:Options){let e=out.entry(k).or_insert((0,s.to_string(),format!("{:?}",o)));e.0+=1;if s.len()<e.1.len(){e.1=s.to_string();e.2=format!("{:?}",o);}}
fn main(){
 std::panic::set_hook(Box::new(|_|{}));
 let maxl:usize=std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(4);
 let g=Options::all()-Options::ENABLE_OLD_FOOTNOTES|Options::ENABLE_FOOTNOTES;
 let sets=[Options::empty(),g,Options::all(),Options::ENABLE_TABLES|Options::ENABLE_DEFINITION_LIST,Options::ENABLE_WIKILINKS|Options::ENABLE_TASKLISTS,Options::ENABLE_FOOTNOTES|Options::ENABLE_HEADING_ATTRIBUTES|Options::ENABLE_GFM];
 let mut hs=vec![];
 for th in 0..4usize{
  let sets=sets.clone();
  hs.push(std::thread::spawn(move||{
   let mut out=BTreeMap::new();
   let n=TOK.len();
   let mut seed=(th as u64+1).wrapping_mul(0x9E3779B97F4A7C15)|1;
   let mut nx=move||{seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;seed};
   let t0=std::time::Instant::now();
   while t0.elapsed().as_secs()<maxl as u64{
      for _ in 0..1000{
      let l=5+(nx()%6) as usize; let mut s=String::new();
      for _ in 0..l{s.push_str(TOK[(nx()%(n as u64)) as usize]);}
      for o in sets.iter(){ check(&s,*o,&mut out); }
      }
   }
   out
  }));
 }
 let mut all=BTreeMap::new();
 for h in hs{ for (k,v) in h.join().unwrap(){ let e=all.entry(k).or_insert((0usize,v.1.clone(),v.2.clone())); e.0+=v.0; if v.1.len()<e.1.len(){e.1=v.1;e.2=v.2;} } }
 for (k,v) in all{ println!("{k}: count={} shortest={:?} opts={}",v.0,v.1,v.2); }
 println!("done");
}
