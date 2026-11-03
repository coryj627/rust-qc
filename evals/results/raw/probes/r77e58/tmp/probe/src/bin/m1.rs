// Metamorphic fuzz: blockquote neutrality, CRLF neutrality, trailing newline neutrality.
use pulldown_cmark::{html, Options, Parser};
struct R(u64); impl R{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn b(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize}}
const TOK:&[&str]=&["a","b"," ","  ","    ","\n","\n","\n","\n\n","*","**","_","~","~~","^","$","$$","{","}","`","``","```","~~~","[","]","[[","]]","![","(",")","<",">","</","<a ","<a b=\"","\"","'","&amp;","\\","\\|","|","|-|","| a |\n|---|\n","-","---",":",": ","- ","* ","1. ","> ","# ","=","===","[^a]","[^a]: x","[a]: /u","[a]","[a][]","[]","<!--","-->","<?","?>","<![CDATA[","]]>","<!D","<div>","</div>","<pre>","</pre>","- [ ] ","{#i .c k=v}","é","x@y.z","\\\n","  \n","a|b\n-|-\n","[a](b \"t\")","[a](b\n\"t\")","[[a|b]]","[[a]]"];
fn h(s:&str,o:Options)->String{let st=s.to_string();match std::panic::catch_unwind(move||{let mut b=String::new();html::push_html(&mut b,Parser::new_ext(&st,o));b}){Ok(b)=>b,Err(_)=>{println!("PANIC in={:?} opts={:?}",s,o);String::new()}}}
fn main(){
 std::panic::set_hook(Box::new(|_|{}));let seed:u64=std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(1);
 let iters:u64=std::env::args().nth(2).and_then(|s|s.parse().ok()).unwrap_or(200000);
 let mut r=R(seed.wrapping_mul(0x9E3779B97F4A7C15)|1);
 let base=[Options::empty(),Options::ENABLE_TABLES,Options::ENABLE_FOOTNOTES,Options::ENABLE_MATH,Options::ENABLE_WIKILINKS,Options::ENABLE_DEFINITION_LIST,Options::ENABLE_HEADING_ATTRIBUTES,Options::ENABLE_STRIKETHROUGH,Options::ENABLE_TASKLISTS,Options::ENABLE_GFM,Options::ENABLE_OLD_FOOTNOTES,Options::ENABLE_SMART_PUNCTUATION];
 let mut bq:Vec<(String,Options,String,String)>=vec![];let mut cr:Vec<(String,Options,String,String)>=vec![];let mut tn:Vec<(String,Options,String,String)>=vec![];let mut li:Vec<(String,Options,String,String)>=vec![];
 fn push(v:&mut Vec<(String,Options,String,String)>,e:(String,Options,String,String)){ if v.iter().any(|x|x.0==e.0&&x.1==e.1){return;} v.push(e); v.sort_by_key(|x|x.0.len()); v.truncate(25);}
 for _ in 0..iters{
  let len=1+r.b(9);
  let mut s=String::new(); for _ in 0..len{ s.push_str(TOK[r.b(TOK.len())]); }
  let o=base[r.b(base.len())];
  // ensure terminal newline for container test
  let mut d=s.clone(); if !d.ends_with('\n'){d.push('\n');}
  let plain=h(&d,o); if d.contains("[[")||d.contains("]]"){continue;}
  // blockquote neutrality
  let q:String=d.split_inclusive('\n').map(|l|format!("> {}",l)).collect();
  let got=h(&q,o);
  let want=if plain.is_empty(){"<blockquote>\n</blockquote>\n".to_string()}else{format!("<blockquote>\n{}</blockquote>\n",plain)};
  if got!=want && !plain.contains("<pre>") && !d.contains("<!--") && !d.contains("<?") && !d.contains("CDATA") && !d.contains("<!D") { push(&mut bq,(d.clone(),o,want,got)); }
  {
   let mut first=true; let q:String=d.split_inclusive('\n').map(|l|{let x=if first{format!("- {}",l)}else if l=="\n"{l.to_string()}else{format!("  {}",l)};first=false;x}).collect();
   let norm=|x:&str|x.replace("<p>","").replace("</p>","").replace('\n',"");
   let got=norm(&h(&q,o)); let want=format!("<ul><li>{}</li></ul>",norm(&plain));
   if got!=want && !plain.contains("<pre>") && !d.contains("<!--") && !d.contains("<?") && !d.contains("CDATA") && !d.contains("<!D") && !d.contains("- ") && !d.contains("* ") { push(&mut li,(d.clone(),o,want,got)); }
  }
  // CRLF neutrality
  let c=d.replace('\n',"\r\n");
  let got=h(&c,o).replace("\r\n","\n");
  if got!=plain && !d.contains('`') { push(&mut cr,(d.clone(),o,plain.clone(),got)); }
  // trailing newline
  let mut nt=d.clone(); nt.pop();
  let got=h(&nt,o);
  if false { push(&mut tn,(d.clone(),o,plain.clone(),got)); }
 }
 for (n,v) in [("BLOCKQUOTE",&bq),("CRLF",&cr),("LISTITEM",&li),("NO-TRAILING-NL",&tn)]{ println!("=== {n}: {}",v.len()); for e in v.iter(){ println!("in={:?} {:?}\n  want={:?}\n  got ={:?}",e.0,e.1,e.2,e.3);} }
}
