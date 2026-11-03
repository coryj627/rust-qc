// Extension neutrality: an option must not change documents without its trigger syntax.
use pulldown_cmark::{html, Options, Parser};
struct R(u64); impl R{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn b(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize}}
const TOK:&[&str]=&["a","b"," ","  ","    ","\t","\n","\n","\n","\n\n","\r\n","*","**","_","~","~~","^","$","$$","{","}","`","``","```","~~~","[","]","[[","]]","![","(",")","<",">","</","<a ","<a b=\"","\"","'","&amp;","\\","\\|","|","|-|","| a |\n|---|\n","-","--","---","...",":",": ","- ","* ","1. ","> ","# ","=","===","[^a]","[^a]: x","[a]: /u","[a]","[a][]","[]","<!--","-->","<?","?>","<![CDATA[","]]>","<div>","</div>","<pre>","[ ] ","[x] ","{#i}","é","x@y.z","[!NOTE]\n","\\\n","  \n","a|b\n-|-\n","[a](b \"t\")","+++"];
fn h(s:&str,o:Options)->String{let st=s.to_string();std::panic::catch_unwind(move||{let mut b=String::new();html::push_html(&mut b,Parser::new_ext(&st,o));b}).unwrap_or_else(|_|"PANIC".into())}
fn main(){
 std::panic::set_hook(Box::new(|_|{}));
 let seed:u64=std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(1);
 let iters:u64=std::env::args().nth(2).and_then(|s|s.parse().ok()).unwrap_or(200000);
 let mut r=R(seed.wrapping_mul(0x9E3779B97F4A7C15)|1);
 let opts:Vec<(&str,Options,Vec<&str>)>=vec![
  ("tables",Options::ENABLE_TABLES,vec!["|"]),
  ("footnotes",Options::ENABLE_FOOTNOTES,vec!["[^"]),
  ("oldfootnotes",Options::ENABLE_OLD_FOOTNOTES,vec!["[^"]),
  ("strike",Options::ENABLE_STRIKETHROUGH,vec!["~"]),
  ("sub",Options::ENABLE_SUBSCRIPT,vec!["~"]),
  ("sup",Options::ENABLE_SUPERSCRIPT,vec!["^"]),
  ("tasks",Options::ENABLE_TASKLISTS,vec!["[ ]","[x]"]),
  ("smart",Options::ENABLE_SMART_PUNCTUATION,vec!["\"","'","--","..."]),
  ("attrs",Options::ENABLE_HEADING_ATTRIBUTES,vec!["{"]),
  ("yaml",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,vec!["---"]),
  ("plus",Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,vec!["+++"]),
  ("math",Options::ENABLE_MATH,vec!["$"]),
  ("gfm",Options::ENABLE_GFM,vec!["[!"]),
  ("deflist",Options::ENABLE_DEFINITION_LIST,vec![":"]),
  ("wiki",Options::ENABLE_WIKILINKS,vec!["[["]),
 ];
 let mut found:Vec<Vec<(String,String,String)>>=vec![vec![];opts.len()];
 for _ in 0..iters{
  let len=1+r.b(10); let mut s=String::new(); for _ in 0..len{s.push_str(TOK[r.b(TOK.len())]);}
  let base=h(&s,Options::empty());
  for (i,(_,o,tr)) in opts.iter().enumerate(){
    if tr.iter().any(|t|s.contains(t)){continue;}
    let got=h(&s,*o);
    if got!=base{ let f=&mut found[i]; if f.iter().all(|x|x.0!=s){f.push((s.clone(),base.clone(),got)); f.sort_by_key(|x|x.0.len()); f.truncate(6);} }
  }
 }
 for (i,(n,_,_)) in opts.iter().enumerate(){ println!("=== {n}: {}",found[i].len()); for x in &found[i]{println!("in={:?}\n  base={:?}\n  got ={:?}",x.0,x.1,x.2);} }
}
