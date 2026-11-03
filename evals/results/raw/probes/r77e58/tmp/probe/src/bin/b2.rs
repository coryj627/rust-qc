use pulldown_cmark::{html, Options, Parser};
use std::time::Instant;
fn run(s:&str,o:Options)->f64{let t=Instant::now();let mut b=String::new();html::push_html(&mut b,Parser::new_ext(s,o));std::hint::black_box(&b);t.elapsed().as_secs_f64()}
fn main(){
 let g=Options::all()-Options::ENABLE_OLD_FOOTNOTES|Options::ENABLE_FOOTNOTES; let e=Options::empty(); let f=Options::ENABLE_FOOTNOTES;
 let cases:Vec<(&str,Box<dyn Fn(usize)->String>,Options)>=vec![
  ("title-lines",Box::new(|n|"[a](b \"".to_string()+&"c\n".repeat(n)+"\")\n"),e),
  ("title-lines-unterm",Box::new(|n|"[a](b \"".to_string()+&"c\n".repeat(n)),e),
  ("deep-list-blank",Box::new(|n|"- ".repeat(n)+"a\n"+&"\n".repeat(n)),e),
  ("deep-list-blank-ws",Box::new(|n|"- ".repeat(n)+"a\n"+&" \n".repeat(n)),e),
  ("deep-ol-blank",Box::new(|n|"1. ".repeat(n)+"a\n"+&"\n".repeat(n)),e),
  ("deep-fn-blank",Box::new(|n|"[^a]: ".repeat(n)+"a\n"+&"\n".repeat(n)),f),
  ("deep-dd-blank",Box::new(|n|"t\n".to_string()+&": ".repeat(n)+"a\n"+&"\n".repeat(n)),Options::ENABLE_DEFINITION_LIST),
  ("fn-lines-distinct",Box::new(|n|(0..n).map(|i|format!("[^{i}]: b\n")).collect()),f),
  ("fn-open-lines",Box::new(|n|"x\n".to_string()+&"[^a\n".repeat(n)),f),
  ("fn-open-para",Box::new(|n|"[^a x\n".repeat(n)),f),
  ("star-run",Box::new(|n|"*".repeat(n)+"a"+&"*".repeat(n)),g),
  ("under-run",Box::new(|n|"_".repeat(n)+"a"+&"_".repeat(n)),g),
  ("caret-run",Box::new(|n|"^".repeat(n)+"a"+&"^".repeat(n)),g),
  ("tilde2",Box::new(|n|"~~a ".repeat(n)+&"b~~ ".repeat(n)),g),
  ("ticks",Box::new(|n|"`".repeat(n)),g),
  ("ticks-a",Box::new(|n|"`a".repeat(n)),g),
  ("ticks-esc",Box::new(|n|"\\`a`".repeat(n)),g),
  ("ticks-esc2",Box::new(|n|"\\``a".repeat(n)),g),
  ("dollars",Box::new(|n|"$".repeat(n)),g),
  ("lt",Box::new(|n|"<".repeat(n)),g),
  ("close-tags",Box::new(|n|"</a".repeat(n)),g),
  ("amps",Box::new(|n|"&".repeat(n)),g),
  ("bq-empty",Box::new(|n|"> \n".repeat(n)),g),
  ("empty-items",Box::new(|n|"- \n".repeat(n)),g),
  ("alt-lists",Box::new(|n|"- a\n+ b\n".repeat(n)),g),
  ("pipes-nohead",Box::new(|n|"|a|b\nc\n".repeat(n)),g),
  ("html-blocks",Box::new(|n|"<div>\n".repeat(n)),g),
  ("html-comment-block",Box::new(|n|"<!--\n".repeat(n)),g),
  ("dd-many",Box::new(|n|"a\n".to_string()+&": b\n".repeat(n)),g),
  ("dd-loose",Box::new(|n|"a\n\n".to_string()+&": b\n\n".repeat(n)),g),
  ("list-dl",Box::new(|n|"- a\n  : b\n".repeat(n)),g),
  ("task-many",Box::new(|n|"- [x]\n".repeat(n)),g),
  ("ref-use",Box::new(|n|"[a]: /u\n\n".to_string()+&"[a] ".repeat(n)),g),
  ("ref-multi-line-label",Box::new(|n|"[a\n".to_string()+&"b\n".repeat(n)+"]: /u\n"),g),
  ("label-multi-lines",Box::new(|n|"x [a\n".to_string()+&"b\n".repeat(n)+"]\n"),g),
  ("label-many-closers",Box::new(|n|"[a\nb]\n".repeat(n)),g),
  ("codespan-lines",Box::new(|n|"`a\n".repeat(n)+"`"),g),
  ("html-inline-lines",Box::new(|n|"<a b\n".to_string()+&"c\n".repeat(n)+">"),g),
  ("comment-lines",Box::new(|n|"x <!--\n".to_string()+&"c\n".repeat(n)+"-->"),g),
  ("bq-comment-lines",Box::new(|n|"> x <!--\n".to_string()+&"> c\n".repeat(n)+"> -->"),g),
  ("bq-title-lines",Box::new(|n|"> [a](b \"\n".to_string()+&"> c\n".repeat(n)+"> \")"),g),
  ("refdef-title-lines",Box::new(|n|"[a]: b \"\n".to_string()+&"c\n".repeat(n)+"\"\n"),g),
  ("table-big-body",Box::new(|n|"|a|b|\n|-|-|\n".to_string()+&"|c|d|\n".repeat(n)),g),
  ("math-lines",Box::new(|n|"$a\n".repeat(n)+"$"),g),
  ("setext-long",Box::new(|n|"a\n".repeat(n)+"===\n"),g),
  ("wiki-long",Box::new(|n|"[[".to_string()+&"a\n".repeat(n)+"]]"),g),
 ];
 let start=std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(1<<10usize);
 let deadline=Instant::now();
 for (name,f,o) in cases.iter(){
  let mut n=start; let mut prev=0.0; let mut line=format!("{name:20}");let mut worst=0.0f64;
  loop{
   let s=f(n); let t=run(&s,*o);
   let ratio=if prev>0.0{t/prev}else{0.0};
   line+=&format!(" {:.0}K:{:.1}ms",s.len() as f64/1000.0,t*1000.0);
   if prev>0.001 && ratio>worst{worst=ratio;}
   prev=t; n*=2;
   if t>1.5||n>start*256||deadline.elapsed().as_secs()>500{break;}
  }
  println!("{} worst={:.1}{}",line,worst,if worst>3.0{" <<<"}else{""});
 }
}
