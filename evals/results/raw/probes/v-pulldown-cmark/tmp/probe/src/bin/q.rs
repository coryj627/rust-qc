use probe::*; use pulldown_cmark::*; use std::panic::*;
fn msg(e: Box<dyn std::any::Any+Send>)->String{ e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or("?".into()) }
fn try_html(name:&str,o:Options,s:&str){
  match catch_unwind(AssertUnwindSafe(||html(o,s))){Ok(h)=>{let l=h.len(); println!("{name}: html[{l}] tail={:?}",&h[h.len().saturating_sub(200)..])},Err(e)=>println!("{name}: PANIC {}",msg(e))}
}
fn try_off(name:&str,o:Options,s:&str){
  let r=catch_unwind(AssertUnwindSafe(||{let mut bad=0;let mut depth=0i64;let mut n=0;for (ev,rg) in Parser::new_ext(s,o).into_offset_iter(){n+=1;match ev{Event::Start(_)=>depth+=1,Event::End(_)=>depth-=1,_=>{}}; if rg.start>rg.end||s.get(rg.clone()).is_none(){bad+=1;if bad<3{println!("  bad range {:?} {:?}",rg,ev)}}};(n,bad,depth)}));
  match r{Ok(x)=>println!("{name}: offset (events,badranges,depth)={:?}",x),Err(e)=>println!("{name}: offset PANIC {}",msg(e))}
}
fn main(){
 std::panic::set_hook(Box::new(|_|{}));
 println!("debug_assertions={}",cfg!(debug_assertions));
 for (n,o,s) in [("wiki1","wiki","![[]j][x]]]\n\n[x]: /u"),("wiki2","wiki","![[x]y](u)]]"),("setext","attrs","- a\n   {#x}\n  ---\n"),("task","task,attrs","- [ ] \\\n  ---\n"),("ref","none","- [a]: /u\n      \n- b")]{
   try_html(n,opts(o),s); try_off(n,opts(o),s);
 }
 // table cap
 let header=format!("{}\n{}\n","|a".repeat(1000)+"|","|-".repeat(1000)+"|");
 for rows in [200,270,300]{
  let s=format!("{header}{}\n# after\n\npara *x*\n","|c|\n".repeat(rows));
  try_html(&format!("tablecap{rows}"),opts("tables"),&s); try_off(&format!("tablecap{rows}"),opts("tables"),&s);
  println!("  size {}",s.len());
 }
 // autofill budget
 let t=format!("{}{}\n",header,"|c|\n".repeat(200));
 let s=t.repeat(60); let o=opts("tables");
 let c=Parser::new_ext(&s,o).count(); let h=html(o,&s);
 println!("autofill: input {} events {} html {}",s.len(),c,h.len());
 // ref cap
 let s=format!("[r]: {}\n\n{}","/".repeat(69),"[r] ".repeat(3000));
 let h=html(Options::empty(),&s); println!("refcap: input {} links {} literal {}",s.len(),h.matches("<a ").count(),h.matches("[r]").count());
 let s=format!("[logo]: data:{}\n\n[logo] [logo] [logo] [logo]\n","a".repeat(40000));
 let h=html(Options::empty(),&s); println!("refcap2: input {} links {} literal {}",s.len(),h.matches("<a ").count(),h.matches("[logo]").count());
 // labels
 for (n,lab) in [("1000a","a".repeat(1000)),("999a","a".repeat(999)),("5000a","a".repeat(5000)),("400cjk","日".repeat(400)),("334cjk","日".repeat(334)),("333cjk","日".repeat(333)),("600e","é".repeat(600))]{
   let s=format!("[{lab}]\n\n[{lab}]: /u\n");
   let h=html(Options::empty(),&s); println!("label {n}: linked={}",h.contains("<a "));
 }
 // collapsed/broken span
 let mut cb=|b:BrokenLink|{println!("  broken span {:?} ref {:?}",b.span,b.reference);Some(("/x".into(),"".into()))};
 for _ev in Parser::new_with_broken_link_callback("x [a][] y",Options::empty(),Some(&mut cb)){}
 // wiki panic with callback
 let r=catch_unwind(||{let mut cb=|_b:BrokenLink|Some(("/x".into(),"".into())); Parser::new_with_broken_link_callback("![[]j][x]]]",opts("wiki"),Some(&mut cb)).count()});
 println!("wiki cb: {:?}",r.map_err(msg));
 // exponential
 for n in [8,16,20,24]{ let s="[[a|]] ".repeat(n); let t=std::time::Instant::now(); let c=Parser::new_ext(&s,opts("wiki")).count(); println!("wikiexp n={n} bytes={} events={c} {:?}",s.len(),t.elapsed()); }
 // write_html_io
 struct Fail; impl std::io::Write for Fail{fn write(&mut self,_:&[u8])->std::io::Result<usize>{Err(std::io::Error::new(std::io::ErrorKind::Other,"x"))} fn flush(&mut self)->std::io::Result<()>{Err(std::io::Error::new(std::io::ErrorKind::Other,"x"))}}
 let r=pulldown_cmark::html::write_html_io(std::io::BufWriter::new(Fail),Parser::new("hi")); println!("bufwriter failing: {:?}",r);
 let r=pulldown_cmark::html::write_html_io(Fail,Parser::new("hi")); println!("unbuffered failing: {:?}",r);
 // text merge
 let v:Vec<_>=TextMergeStream::new(vec![Event::Rule,Event::Text("".into()),Event::Rule].into_iter()).collect(); println!("tms single {:?}",v);
 let v:Vec<_>=TextMergeStream::new(vec![Event::Rule,Event::Text("".into()),Event::Text("".into()),Event::Rule].into_iter()).collect(); println!("tms double {:?}",v);
}
