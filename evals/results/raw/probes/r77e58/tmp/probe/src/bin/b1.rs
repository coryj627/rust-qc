// Doubling timing harness: prints time and growth ratio for crafted inputs.
use pulldown_cmark::{html, Options, Parser};
use std::time::Instant;
fn run(s:&str,o:Options)->f64{let t=Instant::now();let mut b=String::new();html::push_html(&mut b,Parser::new_ext(s,o));std::hint::black_box(&b);t.elapsed().as_secs_f64()}
fn main(){
 let a=Options::all(); let g=Options::all()-Options::ENABLE_OLD_FOOTNOTES|Options::ENABLE_FOOTNOTES;
 let cases:Vec<(&str,Box<dyn Fn(usize)->String>,Options)>=vec![
  ("wiki-open",Box::new(|n|"[[a".repeat(n)),g),
  ("wiki-open-close",Box::new(|n|"[[a]] ".repeat(n)),g),
  ("wiki-img",Box::new(|n|"![[a".repeat(n)+&"]]".repeat(n)),g),
  ("wiki-nest",Box::new(|n|"[[".repeat(n)+"a"+&"]]".repeat(n)),g),
  ("wiki-pipe",Box::new(|n|"[[a|".repeat(n)+&"]]".repeat(n)),g),
  ("wiki-long-pipe",Box::new(|n|"[[".to_string()+&"a ".repeat(n)+"]] "),g),
  ("close-only",Box::new(|n|"]]".repeat(n)),g),
  ("math-$",Box::new(|n|"$a ".repeat(n)),g),
  ("math-$$",Box::new(|n|"$$a ".repeat(n)),g),
  ("math-brace",Box::new(|n|"${".repeat(n)+&"}$".repeat(n)),g),
  ("math-brace2",Box::new(|n|"$".to_string()+&"{".repeat(n)+&"$".repeat(n)),g),
  ("math-brace3",Box::new(|n|"{$a}".repeat(n)),g),
  ("math-brace4",Box::new(|n|"$}".repeat(n)),g),
  ("tilde-caret",Box::new(|n|"~a^ ".repeat(n)),g),
  ("tilde-mix",Box::new(|n|"~a ~~b ".repeat(n)+&"c~ d~~ ".repeat(n)),g),
  ("caret-open",Box::new(|n|"^a ".repeat(n)+&"b~ ".repeat(n)),g),
  ("star-under",Box::new(|n|"*a _b ".repeat(n)),g),
  ("star-under2",Box::new(|n|"a*_".repeat(n)),g),
  ("emph-mod3",Box::new(|n|"*a **b ***c ".repeat(n)+&"d* e** f*** ".repeat(n)),g),
  ("link-open",Box::new(|n|"[".repeat(n)),g),
  ("link-close",Box::new(|n|"[a](".repeat(n)),g),
  ("link-title",Box::new(|n|"[a](b \"".repeat(n)),g),
  ("link-title2",Box::new(|n|"[a](b '".to_string().repeat(n)),g),
  ("link-ref",Box::new(|n|"[a][".repeat(n)),g),
  ("link-img",Box::new(|n|"![".repeat(n)+&"](a)".repeat(n)),g),
  ("link-nested",Box::new(|n|"[".repeat(n)+&"](a)".repeat(n)),g),
  ("link-multiline",Box::new(|n|"[a](b \n".repeat(n)),g),
  ("collapsed",Box::new(|n|"[a][] ".repeat(n)),g),
  ("label-long",Box::new(|n|format!("[{}]\n\n",  "a".repeat(n)).repeat(4)),g),
  ("label-many-long",Box::new(|n|format!("[{}]",  "a ".repeat(n))),g),
  ("label-closers",Box::new(|n|"[".to_string()+&"a]".repeat(n)),g),
  ("backslash-label",Box::new(|n|"[a\\]".repeat(n)),g),
  ("code-ticks",Box::new(|n|(1..n).map(|i|"`".repeat(i%50+1)+" ").collect()),g),
  ("html-open",Box::new(|n|"<a ".repeat(n)),g),
  ("html-attr",Box::new(|n|"<a b=\"".repeat(n)),g),
  ("html-attr2",Box::new(|n|"<a b ".to_string()+&"c ".repeat(n)+"<"),g),
  ("html-multi",Box::new(|n|"<a b\n".repeat(n)),g),
  ("html-comment",Box::new(|n|"x <!-".repeat(n)),g),
  ("html-comment2",Box::new(|n|"x <!-- -".repeat(n)),g),
  ("html-pi",Box::new(|n|"x <? ".repeat(n)),g),
  ("html-cdata",Box::new(|n|"x <![CDATA[ ".repeat(n)),g),
  ("html-cdata2",Box::new(|n|"x <![CDATA[ ]".repeat(n)),g),
  ("html-decl",Box::new(|n|"x <!A ".repeat(n)),g),
  ("autolink",Box::new(|n|"<a:".repeat(n)),g),
  ("email",Box::new(|n|"<a@".repeat(n)),g),
  ("table-wide",Box::new(|n|"|a".repeat(n)+"\n"+&"|-".repeat(n)+"\n"+&"|\n".repeat(n)),g),
  ("table-many",Box::new(|n|"a|b\n-|-\n\n".repeat(n)),g),
  ("table-pipes",Box::new(|n|"|".repeat(n)+"\n"),g),
  ("table-rows",Box::new(|n|"a|b\n-|-\n".to_string()+&"c|d|e|f\n".repeat(n)),g),
  ("table-para",Box::new(|n|"|a\n".repeat(n)),g),
  ("table-para2",Box::new(|n|"x\n".to_string()+&"|a|\n".repeat(n)),g),
  ("bq-deep",Box::new(|n|">".repeat(n)+"a\n"),g),
  ("bq-deep-lines",Box::new(|n|(">".repeat(n)+"a\n").repeat(16)),g),
  ("list-deep",Box::new(|n|"- ".repeat(n)+"a\n"),g),
  ("list-many",Box::new(|n|"- a\n".repeat(n)),g),
  ("list-grow",Box::new(|n|(0..n.min(2000)).map(|i|" ".repeat(i*2)+"- a\n").collect()),g),
  ("fn-many",Box::new(|n|"[^a]: b\n".repeat(n)),g),
  ("fn-many-ref",Box::new(|n|"[^a]".repeat(n)+"\n\n[^a]: b\n"),g),
  ("fn-old-nest",Box::new(|n|"[^a]: ".repeat(n)),a),
  ("fn-old-nest2",Box::new(|n|"[^a]:\n".repeat(n)),a),
  ("fn-gfm-nest",Box::new(|n|"[^a]: ".repeat(n)),g),
  ("deflist",Box::new(|n|"a\n: b\n".repeat(n)),g),
  ("deflist2",Box::new(|n|"a\n: b\n\nc\n".repeat(n)),g),
  ("deflist-deep",Box::new(|n|"a\n".to_string()+&": ".repeat(n)+"b\n"),g),
  ("deflist-nested",Box::new(|n|(0..n.min(2000)).map(|i|" ".repeat(i*2)+"a\n"+&" ".repeat(i*2)+": b\n").collect()),g),
  ("refdef",Box::new(|n|"[a]: b\n".repeat(n)),g),
  ("refdef-title",Box::new(|n|"[a]: b\n'c\n".repeat(n)),g),
  ("refdef-title2",Box::new(|n|"[a]: b '".to_string()+&"c\n".repeat(n)),g),
  ("refdef-open",Box::new(|n|"[a\n".repeat(n)),g),
  ("refdef-lines",Box::new(|n|"[a]:\n".repeat(n)),g),
  ("meta",Box::new(|n|"---\na\n".repeat(n)),g),
  ("meta2",Box::new(|n|"+++\n\n".repeat(n)),g),
  ("meta3",Box::new(|n|"---a\n".repeat(n)),g),
  ("meta-open-many",Box::new(|n|"+++\nx\n---\nx\n".repeat(n)),g),
  ("meta-unclosed",Box::new(|n|"\n---\nx\n\n\n".to_string().replace("---","+++")+&"x\n\n".repeat(n)),g),
  ("heading-attr",Box::new(|n|"# a {".repeat(n)),g),
  ("heading-attr2",Box::new(|n|"a{\n".repeat(n)+"===\n"),g),
  ("setext-attr",Box::new(|n|"a\n".repeat(n)+"{#a}\n===\n"),g),
  ("setext-attr-many",Box::new(|n|"a {#a}\n".repeat(n)+"===\n"),g),
  ("setext-bq",Box::new(|n|"> a\n".repeat(n)+"> {#a}\n> ===\n"),g),
  ("gfm-note",Box::new(|n|"> [!NOTE]\n".repeat(n)),g),
  ("gfm-note2",Box::new(|n|"> [!NOTE]\n> ".repeat(n)),g),
  ("task",Box::new(|n|"- [ ] a\n".repeat(n)),g),
  ("smart",Box::new(|n|"'\"".repeat(n)),g),
  ("dash",Box::new(|n|"-".repeat(n)+"a"),g),
  ("entity",Box::new(|n|"&a".repeat(n)),g),
  ("entity2",Box::new(|n|"&".to_string()+&"a".repeat(n)),g),
  ("hard",Box::new(|n|"a\\\n".repeat(n)),g),
  ("trailing-ws",Box::new(|n|"a".to_string()+&" ".repeat(n)+"\n"),g),
  ("backslashes",Box::new(|n|"\\".repeat(n)+"\n"),g),
  ("ws-lines",Box::new(|n|format!("a{}\n",&" ".repeat(n)).repeat(4)),g),
 ];
 let start=std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(1<<12usize);
 let deadline=Instant::now();
 for (name,f,o) in cases.iter(){
  let mut n=start; let mut prev=0.0; let mut line=format!("{name:18}");let mut worst=0.0f64;
  loop{
   let s=f(n); let t=run(&s,*o);
   let ratio=if prev>0.0{t/prev}else{0.0};
   line+=&format!(" {:.0}K:{:.1}ms",s.len() as f64/1000.0,t*1000.0);
   if prev>0.002 && ratio>worst{worst=ratio;}
   prev=t; n*=2;
   if t>2.0||n>start*32||deadline.elapsed().as_secs()>560{break;}
  }
  println!("{} worst_ratio={:.1}{}",line,worst,if worst>3.0{" <<<"}else{""});
 }
}
