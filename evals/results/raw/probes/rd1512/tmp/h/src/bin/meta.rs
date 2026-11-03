use pulldown_cmark::*;
fn evs(s:&str,o:Options)->Vec<Event<'static>>{ TextMergeStream::new(Parser::new_ext(s,o)).map(|e|e.into_static()).collect() }
fn corpus()->Vec<(String,String)>{ let mut v=vec![]; for f in ["third_party/CommonMark/spec.txt","third_party/GitHub/spec.txt","specs/regression.txt","specs/table.txt","specs/footnotes.txt","specs/math.txt","specs/wikilinks.txt","specs/strikethrough.txt","specs/super_sub.txt","specs/definition_lists.txt","specs/heading_attrs.txt","specs/blockquotes_tags.txt","specs/metadata_blocks.txt"] {
  let t=std::fs::read_to_string(format!("/work/crate/pulldown-cmark/{}",f)).unwrap(); let mut lines=t.lines().peekable(); 
  while let Some(l)=lines.next(){ if l.starts_with("````````") && l.ends_with("example") || (l.starts_with("````````") && l.contains("example")) { let mut md=String::new(); for m in lines.by_ref(){ if m=="." {break;} md.push_str(m); md.push('\n'); } for m in lines.by_ref(){ if m.starts_with("````````"){break;} } v.push((f.to_string(),md.replace('→',"\t"))); } } }
 v }
fn strip_para(v:&[Event<'static>])->Vec<Event<'static>>{ v.iter().filter(|e|!matches!(e,Event::Start(Tag::Paragraph)|Event::End(TagEnd::Paragraph))).cloned().collect() }
fn norm(v:Vec<Event<'static>>)->Vec<Event<'static>>{ v.into_iter().map(|e| match e { Event::Text(t)=>Event::Text(t.replace("\r\n","\n").replace('\r',"\n").into()), Event::Html(t)=>Event::Html(t.replace("\r\n","\n").into()), Event::InlineHtml(t)=>Event::InlineHtml(t.replace("\r\n","\n").into()),Event::InlineMath(t)=>Event::InlineMath(t.replace("\r\n","\n").into()),Event::DisplayMath(t)=>Event::DisplayMath(t.replace("\r\n","\n").into()), o=>o}).collect() }
fn mh(v:Vec<Event<'static>>)->Vec<Event<'static>>{ let mut o:Vec<Event<'static>>=vec![]; for e in v { if let (Some(Event::Html(a)),Event::Html(b))=(o.last(),&e) { let n=format!("{}{}",a,b); *o.last_mut().unwrap()=Event::Html(n.into()); continue; } o.push(e);} o }
fn main(){
 let o=Options::ENABLE_TABLES|Options::ENABLE_FOOTNOTES|Options::ENABLE_STRIKETHROUGH|Options::ENABLE_TASKLISTS|Options::ENABLE_HEADING_ATTRIBUTES|Options::ENABLE_MATH|Options::ENABLE_GFM|Options::ENABLE_DEFINITION_LIST|Options::ENABLE_SUPERSCRIPT|Options::ENABLE_SUBSCRIPT|Options::ENABLE_WIKILINKS;
 let c=corpus(); println!("corpus {}",c.len());
 let mut cnt=[0;4];
 for (f,md) in &c {
  let md2=md.clone(); let b=match std::panic::catch_unwind(move||evs(&md2,o)){Ok(b)=>b,Err(_)=>{println!("PANIC base {:?}",md);continue}};
  // CRLF
  let crlf=md.replace('\n',"\r\n");
  let x=mh(norm(evs(&crlf,o))); if x!=mh(norm(b.clone())) { cnt[0]+=1; if cnt[0]<=25 {println!("CRLF {} {:?}\n  base {:?}\n  got  {:?}",f,md,b,x);} }
  // CR
  // blockquote
  let q:String=md.lines().map(|l|if l.is_empty(){">\n".to_string()}else{format!("> {}\n",l)}).collect();
  let q2=q.clone(); match std::panic::catch_unwind(move||evs(&q2,o)){ Ok(x)=>{ if x.len()<2 || x[1..x.len()-1]!=b[..] { cnt[1]+=1; if cnt[1]<=25 {println!("BQ {} {:?}\n  base {:?}\n  got  {:?}",f,md,b,x);} } }, Err(_)=>println!("PANIC bq {:?}",q) }
  // list item
  let mut first=true; let li:String=md.lines().map(|l|{ let p=if first {"- "} else if l.is_empty(){""} else {"  "}; first=false; format!("{}{}\n",p,l)}).collect();
  let q2=li.clone(); match std::panic::catch_unwind(move||evs(&q2,o)){ Ok(x)=>{ if x.len()<4 || strip_para(&x[2..x.len()-2])!=strip_para(&b) { cnt[2]+=1; if cnt[2]<=25 {println!("LI {} {:?}\n  base {:?}\n  got  {:?}",f,md,b,x);} } }, Err(_)=>println!("PANIC li {:?}",li) }
  // no trailing newline
  let nt=md.trim_end_matches('\n'); let n2=nt.to_string(); match std::panic::catch_unwind(move||evs(&n2,o)){ Ok(x)=>{ if x!=b { cnt[3]+=1; if cnt[3]<=15 {println!("NONL {} {:?}\n  base {:?}\n  got  {:?}",f,md,b,x);} } }, Err(_)=>println!("PANIC nonl {:?}",nt) }
 }
 println!("counts crlf/bq/li/nonl {:?}",cnt);
}
