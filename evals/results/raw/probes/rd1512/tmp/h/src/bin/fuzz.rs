use pulldown_cmark::*;
use std::panic;
struct R(u64);
impl R { fn n(&mut self)->u64{ let mut x=self.0; x^=x<<13; x^=x>>7; x^=x<<17; self.0=x; x } fn b(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize} }
const T:&[&str]=&["a","b ","\n","\n","\n\n","\r\n","\r","  ","    ","\t","> ",">","- ","* ","+ ","1. ","2) ","# ","## ","---","===","```","~~~","`","``","\\","*","**","_","__","~","~~","^","$","$$","[","]","[[","]]","![","(",")","()","<",">","</a>","<a b='c'>","<!--","-->","<?","?>","<![CDATA[","]]>","<!x","|","|-|","| a |","\n|-|-|\n",":--",":","::",": ","[^a]","[^a]: ","[a]: /u","[a]:","\"","'","&amp;","&#35;","&","é","漢","🍔","\u{a0}","\u{2028}","{#i .c k=v}","{","}","[x] ","[ ] ","[!NOTE]","\\|","\\\n","+++","--- \n","\\[","http://a.b","x@y.z","<http://a>","<x@y.z>",".","!", "=", "-", "\u{85}"];
fn main(){
 let a:Vec<String>=std::env::args().collect();
 let seed:u64=a[1].parse().unwrap(); let iters:u64=a[2].parse().unwrap();
 panic::set_hook(Box::new(|_|{}));
 let mut r=R(seed*2654435761+88172645463325252);
 let mut seen=std::collections::HashSet::new();
 for _ in 0..iters {
  let l=1+r.b(30); let mut s=String::new(); for _ in 0..l { s.push_str(T[r.b(T.len())]); }
  let o=Options::from_bits_truncate(match r.b(4){0=>Options::all().bits(),1=>0,_=>(r.n() & 0xffff) as u32} & !0x8000);
  let s2=s.clone();
  let res=panic::catch_unwind(move ||{ check(&s2,o) });
  let msg=match res { Ok(Ok(()))=>continue, Ok(Err(m))=>m, Err(e)=>{ let loc=e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|x|x.to_string())).unwrap_or_default(); format!("PANIC {}",loc)} };
  let key:String=kind(&msg);
  if seen.insert(key.clone()) { let m=minimize(&s,o,&key); let o2=mino(&m,o,&key); println!("{} | opts={:#x} | {:?}",run1(&m,o2).unwrap_or(msg),o2.bits(),m); }
 }
}
fn kind(m:&str)->String{ let w:Vec<&str>=m.split(' ').take(5).collect(); let k=w.join(" "); k.chars().filter(|c|!c.is_ascii_digit()).take(36).collect() }
fn run1(s:&str,o:Options)->Option<String>{ let s2=s.to_string(); match panic::catch_unwind(move ||check(&s2,o)) { Ok(Ok(()))=>None, Ok(Err(m))=>Some(m), Err(e)=>Some(format!("PANIC {}",e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|x|x.to_string())).unwrap_or_default())) } }
fn minimize(s:&str,o:Options,key:&str)->String{ let mut cur:Vec<char>=s.chars().collect(); let mut ch=true; while ch { ch=false; let mut w=cur.len()/2; while w>=1 { let mut i=0; while i+w<=cur.len(){ let mut t=cur.clone(); t.drain(i..i+w); let ts:String=t.iter().collect(); if run1(&ts,o).map(|m|kind(&m)==key).unwrap_or(false){ cur=t; ch=true; } else { i+=1; } } w/=2; } } cur.iter().collect() }
fn mino(s:&str,o:Options,key:&str)->Options{ let mut c=o; for b in 0..32 { let bit=1u32<<b; if c.bits()&bit!=0 { let n=Options::from_bits_truncate(c.bits()&!bit); if run1(s,n).map(|m|kind(&m)==key).unwrap_or(false){c=n;} } } c }
fn check(s:&str,o:Options)->Result<(),String>{
 let e1:Vec<_>=Parser::new_ext(s,o).collect();
 let e2:Vec<_>=Parser::new_ext(s,o).into_offset_iter().collect();
 let e2e:Vec<_>=e2.iter().map(|x|x.0.clone()).collect();
 if e1!=e2e {return Err(format!("offset-vs-plain differ"));}
 let mut st:Vec<(TagEnd,std::ops::Range<usize>)>=vec![];
 let mut prev_end=0usize; 
 for (e,r) in &e2 {
  if r.start>r.end||r.end>s.len(){return Err(format!("range oob {:?} {:?}",e,r));}
  if !s.is_char_boundary(r.start)||!s.is_char_boundary(r.end){return Err(format!("range nonboundary {:?}",e));}
  match e {
   Event::Start(t)=>{ if let Some(p)=st.last(){ if r.start<p.1.start||r.end>p.1.end {return Err(format!("child outside parent {:?} {:?} parent {:?}",tagname(t),r,p));} } st.push((t.to_end(),r.clone())); prev_end=r.start; }
   Event::End(t)=>{ let Some(p)=st.pop() else {return Err("unbalanced end".into())}; if p.0!=*t {return Err(format!("mismatched end {:?} {:?}",p.0,t));} if p.1!=*r {return Err(format!("end range differs {:?}",t));} prev_end=r.end; }
   Event::Text(x)=>{ if let Some(p)=st.last(){ if r.start<p.1.start||r.end>p.1.end {return Err(format!("text outside parent {:?} {:?}",p.0,r));} }
      if r.start<prev_end && !matches!(st.last(),Some((TagEnd::CodeBlock|TagEnd::HtmlBlock|TagEnd::MetadataBlock(_),_))) { return Err(format!("text overlaps previous {:?}",x.len()>0)); } prev_end=r.end;
      
   }
   _=>{ if let Some(p)=st.last(){ if r.start<p.1.start||r.end>p.1.end {return Err(format!("leaf outside parent {:?} {:?} {:?}",p.0,e,r));} } prev_end=r.end; }
  }
 }
 if !st.is_empty(){return Err("unclosed".into());}
 let mut h=String::new(); html::push_html(&mut h,e1.into_iter());
 // into_static
 let v:Vec<_>=Parser::new_ext(s,o).map(|e|e.into_static()).collect(); drop(v);
 Ok(())
}
fn tagname(t:&Tag)->String{ format!("{:?}",t).chars().take(12).collect() }
