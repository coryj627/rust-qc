use pulldown_cmark::{html, Options, Parser, Event};
fn h(s: &str, o: Options) -> String { let mut b=String::new(); html::push_html(&mut b, Parser::new_ext(s,o)); b }
fn ev(name:&str,s:&str,o:Options){ println!("--- events {name} {:?}",s); for (e,r) in Parser::new_ext(s,o).into_offset_iter(){println!("  {:?} {:?} src={:?}",r,e,s.get(r.clone()));} }
fn main(){
 let e=Options::empty();
 // fuel: same short reference used many times with a moderately long URL
 let url="https://example.com/a/fairly/ordinary/documentation/path/to/page.html";
 for n in [1000usize,3000,30000]{
  let s=format!("[r]: {url}\n\n")+&"See [r] now.\n".repeat(n);
  let out=h(&s,e); let links=out.matches("<a href").count(); let lit=out.matches("[r]").count();
  println!("uses={n} size={} links={links} literal={lit}",s.len());
 }
 ev("list offsets","- 1. ]\n\t-",e);
 ev("list simple","- a\n- b\n\nc\n",e);
 ev("nested list","- a\n  - b\nc\n",e);
 ev("bq","> a\n> b\n\nc\n",e);
 ev("task loose","- [x] a\n\n- b\n",Options::ENABLE_TASKLISTS);
 ev("code","```\nx\n```\n\n    y\n",e);
 ev("html","<div>\nx\n</div>\n\nz\n",e);
 ev("setext","a\nb\n===\nc\n",e);
 ev("escape","\\*a &amp; b\n",e);
 ev("table","|a|b|\n|-|-|\n|c|\n",Options::ENABLE_TABLES);
 ev("footnote","a[^1]\n\n[^1]: b\n    c\n",Options::ENABLE_FOOTNOTES);
 ev("meta","---\na: 1\n---\nb\n",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
 ev("hardbreak","a  \nb\\\nc\n",e);
 ev("autolink","<http://a.b> <c@d.e>\n",e);
 ev("wiki","[[a|b]] [[c]]\n",Options::ENABLE_WIKILINKS);
 ev("math","$a$ $$b$$\n",Options::ENABLE_MATH);
 ev("dl","t\n: d\n",Options::ENABLE_DEFINITION_LIST);
 ev("gfm","> [!NOTE]\n> a\n",Options::ENABLE_GFM);
 ev("image ref","![a][b]\n\n[b]: /u\n",e);
 ev("heading attr","# a {#x}\n",Options::ENABLE_HEADING_ATTRIBUTES);
 ev("smart","a--b \"c\"\n",Options::ENABLE_SMART_PUNCTUATION);
 ev("code span","a `b` c\n",e);
 // deep/blank
 println!("{:?}",Parser::new("- [a]: /u\n      \n").reference_definitions().get("a").map(|d|d.dest.to_string()));
 println!("doc dash: {}",h("a--b",Options::ENABLE_SMART_PUNCTUATION).escape_unicode().to_string().len());
 let ev2:Vec<Event>=Parser::new_ext("a|b\n-|-\n",Options::ENABLE_TABLES).collect(); println!("{:?}",ev2.len());
}
