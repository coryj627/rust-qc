// Exhaustive token sequences, panics and invariants.
use pulldown_cmark::*; use std::panic;
fn check(inp:&str, op:Options)->Result<(),String>{ let mut stack:Vec<TagEnd>=vec![];
  for (e,r) in Parser::new_ext(inp,op).into_offset_iter(){ if r.start>r.end||r.end>inp.len(){return Err(format!("bad range {:?}",e));} if !inp.is_char_boundary(r.start)||!inp.is_char_boundary(r.end){return Err(format!("non-boundary {:?}",e));}
    match &e{Event::Start(t)=>stack.push(t.to_end()),Event::End(t)=>{ if stack.pop()!=Some(*t){return Err(format!("unbalanced end {:?}",t));}},_=>{}} }
  if !stack.is_empty(){return Err("unclosed".into());} let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(inp,op)); 
  let cb=|_l:BrokenLink<'_>|Some(("cb".into(),"t".into())); let mut h2=String::new(); html::push_html(&mut h2,Parser::new_with_broken_link_callback(inp,op,Some(cb)));
  let mut d=String::from(inp); d.push_str("\n\n[x]: /u\n[^x]: f\n"); let mut h3=String::new(); html::push_html(&mut h3,Parser::new_ext(&d,op)); for (_,r) in Parser::new_ext(&d,op).into_offset_iter(){ if r.start>r.end{return Err(format!("inverted {:?}",r));} } Ok(()) }
static LOC:std::sync::Mutex<String>=std::sync::Mutex::new(String::new());
fn main(){ panic::set_hook(Box::new(|i|{ if let Some(l)=i.location(){ *LOC.lock().unwrap()=format!("{}:{}",l.file(),l.line()); } }));
 let a1=std::env::args().nth(1).unwrap(); let toks:Vec<&str>=a1.split(' ').map(|s|if s=="NL"{"\n"}else if s=="SP"{" "}else if s=="TAB"{"\t"}else if s=="CR"{"\r"}else if s=="SP4"{"    "}else if s=="SP2"{"  "}else{s}).collect();
 let maxl:u32=std::env::args().nth(2).unwrap().parse().unwrap();
 let ops:Vec<Options>=std::env::args().nth(3).unwrap().split(',').map(|x|Options::from_bits_truncate(x.parse().unwrap())).collect();
 let mut seen=std::collections::HashMap::<String,(String,u32)>::new(); let n=toks.len() as u64;
 for l in 1..=maxl{ for mut i in 0..n.pow(l){ let mut s=String::new(); for _ in 0..l{s.push_str(toks[(i%n) as usize]); i/=n;}
   for &o in &ops{ let m=match panic::catch_unwind(||check(&s,o)){Ok(Ok(()))=>continue,Ok(Err(m))=>m,Err(p)=>format!("PANIC {} {}",LOC.lock().unwrap(),p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or("?".into()))};
     let en=seen.entry(m.chars().take(60).collect()).or_insert((s.clone(),o.bits())); if s.len()<en.0.len(){*en=(s.clone(),o.bits());} } } }
 println!("distinct {}",seen.len()); for (k,(i,o)) in seen{println!("{k}\n  opts={o} {i:?}");}
}
