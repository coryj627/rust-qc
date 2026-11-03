use pulldown_cmark::*; use std::time::Instant;
fn t(name:&str,o:Options,f:&dyn Fn(usize)->String,base:usize){
 let mut n=base; let mut prev=0.0;
 for _ in 0..6 { let s=f(n); let st=Instant::now(); let c=Parser::new_ext(&s,o).count(); let d=st.elapsed().as_secs_f64();
  println!("{} n={} bytes={} events={} t={:.3}s ratio={:.1}",name,n,s.len(),c,d,if prev>0.0{d/prev}else{0.0}); prev=d; if d>6.0 {break;} n*=2; }
}
fn main(){
 let e=Options::empty();
 t("title-lines",e,&|n|format!("[a](b \"{}","x\n".repeat(n)),10000);
 t("yaml-unclosed",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,&|n|"---\t\na\n".repeat(n),10000);
 t("wiki-image",Options::ENABLE_WIKILINKS,&|n|format!("{}{}","![[a ".repeat(n),"]]".repeat(n)),5000);
 t("list-blank",e,&|n|format!("{}a\n{}","- ".repeat(n),"\n".repeat(n)),2000);
 t("bq-lazy",e,&|n|format!("{}a\n{}",">".repeat(n),"b\n".repeat(n)),2000);
 t("table-cells",Options::ENABLE_TABLES,&|n|format!("{}\n{}\n{}","|a".repeat(n),"|-".repeat(n),"|\n".repeat(n)),1000);
 t("math-brace",Options::ENABLE_MATH,&|n|"{$a ".repeat(n),5000);
 t("math-unb",Options::ENABLE_MATH,&|n|"$a}$b ".repeat(n),5000);
 t("code-run",e,&|n|{let mut s=String::new(); for i in 1..n { s.push_str(&"`".repeat(i)); s.push(' ');} s},300);
 t("deflist",Options::ENABLE_DEFINITION_LIST,&|n|"a\n: b\n".repeat(n),10000);
 t("deflist2",Options::ENABLE_DEFINITION_LIST,&|n|"a\n\n".repeat(n)+": b\n",10000);
 t("footnote-nest",Options::ENABLE_FOOTNOTES,&|n|"[^a]:".repeat(n),5000);
 t("html-comment",e,&|n|"a <!- ".repeat(n),10000);
 t("cdata",e,&|n|"a <![CDATA[ ]".repeat(n),10000);
 t("label-pipe",Options::ENABLE_TABLES,&|n|format!("| a |\n|---|\n{}", "| [a\\|b] |\n".repeat(n)),10000);
 t("refdef-lines",e,&|n|format!("[a]: b \"{}","x\n".repeat(n)),10000);
 t("heading-attr",Options::ENABLE_HEADING_ATTRIBUTES,&|n|format!("{}{{#x}}\n===\n","a\n".repeat(n)),10000);
 t("star-under",e,&|n|"*a_ ".repeat(n)+&"_a* ".repeat(n),10000);
 t("tilde",Options::ENABLE_STRIKETHROUGH,&|n|"~a ".repeat(n)+&"~~b~ ".repeat(n),10000);
 t("bracket",e,&|n|"[".repeat(n)+&"a]".repeat(n),10000);
 t("img-nest",e,&|n|"![".repeat(n)+&"](a)".repeat(n),5000);
 t("link-img-star",e,&|n|"![*a ".repeat(n)+&"*](a)".repeat(n),5000);
 t("task",Options::ENABLE_TASKLISTS,&|n|"- [ ] ".repeat(n),5000);
 t("lt",e,&|n|"<a b=\"\n".repeat(n),10000);
}
