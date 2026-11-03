// Exhaustive short inputs + seeded mutations, under catch_unwind, with event invariants.
use pulldown_cmark::*;
use std::panic;
fn check(inp:&str, op:Options)->Result<(),String>{
  // balance, range checks
  let mut stack:Vec<TagEnd>=vec![];
  for (e,r) in Parser::new_ext(inp,op).into_offset_iter(){
    if r.start>r.end||r.end>inp.len(){return Err(format!("bad range {:?} {:?}",r,e));}
    if !inp.is_char_boundary(r.start)||!inp.is_char_boundary(r.end){return Err(format!("non-boundary range {:?} {:?}",r,e));}
    match &e{Event::Start(t)=>stack.push(t.to_end()),Event::End(t)=>{ if stack.pop()!=Some(*t){return Err(format!("unbalanced end {:?}",t));}},_=>{}}
  }
  if !stack.is_empty(){return Err("unclosed".into());}
  let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(inp,op));
  // callback supplying a destination for every broken reference, text merging and into_static
  let cb=|_l:BrokenLink<'_>|Some(("cb".into(),"t".into()));
  let mut h2=String::new(); let p=Parser::new_with_broken_link_callback(inp,op,Some(cb)); html::push_html(&mut h2,TextMergeStream::new(p));
  let cb2=|l:BrokenLink<'_>|Some((l.reference.to_string().into(),"".into()));
  for (e,r) in TextMergeWithOffset::new(Parser::new_with_broken_link_callback(inp,op,Some(cb2)).into_offset_iter()){ if r.start>r.end||r.end>inp.len(){return Err(format!("cb bad range {:?}",r));} let _=e.into_static(); }
  // plain vs offset iterator agreement
  let a:Vec<_>=Parser::new_ext(inp,op).collect(); let b:Vec<_>=Parser::new_ext(inp,op).into_offset_iter().map(|x|x.0).collect();
  if a!=b {return Err("iter mismatch".into());}
  Ok(())
}
static LOC:std::sync::Mutex<String>=std::sync::Mutex::new(String::new());
struct R(u64); impl R{fn n(&mut self)->u64{self.0^=self.0<<13;self.0^=self.0>>7;self.0^=self.0<<17;self.0} fn b(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize}}
fn main(){
  panic::set_hook(Box::new(|i|{ if let Some(l)=i.location(){ *LOC.lock().unwrap()=format!("{}:{}",l.file(),l.line()); } }));
  let mode=std::env::args().nth(1).unwrap(); let arg:u64=std::env::args().nth(2).unwrap().parse().unwrap();
  let seen=std::sync::Mutex::new(std::collections::HashMap::<String,(String,u32)>::new());
  let report=|inp:&str,op:Options,msg:String|{ let k:String=msg.chars().take(70).collect(); let mut s=seen.lock().unwrap(); let en=s.entry(k).or_insert((inp.to_string(),op.bits())); if inp.len()<en.0.len(){*en=(inp.to_string(),op.bits());}};
  let run=|inp:&str,op:Options|{ let r=panic::catch_unwind(||check(inp,op)); match r{Ok(Ok(()))=>{},Ok(Err(m))=>report(inp,op,m),Err(p)=>{let m=p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or("?".into()); let loc=LOC.lock().unwrap().clone(); report(inp,op,format!("PANIC {} {}",loc,m));}}};
  let optsets=[Options::all(),Options::all()-Options::ENABLE_OLD_FOOTNOTES|Options::ENABLE_FOOTNOTES,Options::empty(),Options::ENABLE_WIKILINKS,Options::ENABLE_TABLES|Options::ENABLE_FOOTNOTES,Options::ENABLE_DEFINITION_LIST|Options::ENABLE_FOOTNOTES,Options::ENABLE_MATH|Options::ENABLE_TABLES,Options::ENABLE_GFM|Options::ENABLE_TASKLISTS|Options::ENABLE_TABLES,Options::ENABLE_HEADING_ATTRIBUTES|Options::ENABLE_DEFINITION_LIST];
  if mode=="ex"{ // exhaustive up to len arg over alphabet
    let alpha:Vec<char>="\n>-[]|:^ *`!$\\#a{}".chars().collect(); let n=alpha.len();
    let optsets2=[optsets[0],optsets[1],optsets[3],optsets[5]];
    for l in 1..=arg as u32{ let tot=(n as u64).pow(l); for mut i in 0..tot{ let mut s=String::new(); for _ in 0..l{s.push(alpha[(i%n as u64) as usize]); i/=n as u64;} for &o in &optsets2{run(&s,o);} } }
  } else {
    let seeds=["# h {#i .c a=b}\n\n> q\n> [!NOTE]\n> x\n\n- [x] a\n- b\n  c\n\n1. o\n\n   p\n\n|a|b|\n|-|:-|\n|`c\\|`|*d*|\n\nterm\n: def\n\n: def2\n\n[^1]: foot\n    more\n\nr[^1] [l](u \"t\") ![i](v) [[w|x]] ![[y]] $m$ $$n$$ ~s~ ~~t~~ ^u^ <b a='1'> <http://x.y> &amp; \\* \"q\" 'r' -- ... \n\n[r]: /u 't'\n\n```rs\nc\n```\n\n    ind\n\n<div>\nh\n</div>\n\n---\ny: z\n---\n\n+++\nz\n+++\n\ns\n===\n","> - a\n>   b\n> - | x |\n>   |---|\n>   | y |\n>\n>     code\n>\n> ```\n> f\n> ```\n[x]: <a b>\n  'title\n  more'\n[^f]: > q\n  - l\n\n\\\n*a**b*** _c_ __d__ `e``f` ``g` [h][x] [x][] [x] ![j][x] \r\n\r\nk  \r\nl\\\r\nm\r\n","é*ü* 日本語|表\n-|-\n ß | ∑ \n\n🍔[[é|ü]] $é$ ~é~\n\n* * *\n\t- tab\n\t\tmore\n\n<!-- c -->\n<?p?>\n<!D>\n<![CDATA[x]]>\n<pre>\ny</pre>\n"];
    let toks=["\n","\r\n","\r",">","> ","- ","1. ","* ","[","]","(",")","![","[[","]]","|","|-|\n","\n|-|\n",":",": ","[^","[^a]","[^a]: ","^","~","~~","*","**","_","`","``","```\n","~~~\n","$","$$","{","}","\\","#","# ","<","<a>","</a>","<!--","-->","<?","?>","&amp;","&#","'","\"","\t","    ","  ","a","é","🍔","---\n","+++\n","===\n","[!NOTE]","[x]","[ ]","http://a.b","<http://a.b>","a@b.c","<a@b.c>","{#a}"," {#a .b}\n"];
    let mut r=R(arg|1);
    let mut cnt=0u64;
    loop{ cnt+=1; if cnt>4_000_000{break;}
      let base=seeds[r.b(seeds.len())];
      let mut s:String=if r.b(3)==0 {String::new()} else { let cs:Vec<char>=base.chars().collect(); let a=r.b(cs.len()); let l=r.b(80).min(cs.len()-a); cs[a..a+l].iter().collect() };
      for _ in 0..r.b(12){ let cs:Vec<(usize,char)>=s.char_indices().collect(); let at=if cs.is_empty(){0}else{cs[r.b(cs.len())].0}; match r.b(3){0|1=>s.insert_str(at,toks[r.b(toks.len())]),_=>{ if !cs.is_empty(){ s.remove(at);} }} }
      let o=optsets[r.b(optsets.len())]; run(&s,o);
    }
  }
  let s=seen.lock().unwrap(); println!("distinct failures: {}",s.len()); for (k,(i,o)) in s.iter(){println!("{k}\n   opts={o:#x} input={i:?}");}
}
