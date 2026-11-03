use pulldown_cmark::{html, Options, Parser, Event, BrokenLink, Tag};
fn h(s: &str, o: Options) -> String { let mut b=String::new(); html::push_html(&mut b, Parser::new_ext(s,o)); b }
fn t(name:&str, s:&str, o:Options){
  let r = std::panic::catch_unwind(||h(s,o));
  let d=if s.len()>120 {format!("{:?}...",s.chars().take(60).collect::<String>())}else{format!("{:?}",s)};
  println!("--- {name}\nIN : {}", d);
  match r { Ok(x)=>{ if x.len()>600 {println!("OUT(len {}): {:?}", x.len(), x.chars().take(100).collect::<String>())} else {println!("OUT: {:?}", x)} }, Err(_)=>println!("OUT: PANIC") }
}
fn ev(name:&str,s:&str,o:Options){ println!("--- events {name} {:?}",s); match std::panic::catch_unwind(||Parser::new_ext(s,o).into_offset_iter().collect::<Vec<_>>()){Ok(v)=>for (e,r) in v{println!("  {:?} {:?} src={:?}",r,e,s.get(r.clone()));},Err(_)=>println!("  PANIC")} }
fn main(){
 let e=Options::empty(); let tb=Options::ENABLE_TABLES;
 let hook=std::panic::take_hook(); std::panic::set_hook(Box::new(move |i|{println!("panic: {}",i);let _=&hook;}));
 t("refdef ws list","- [a]: /u\n      \n",e);
 t("refdef ws list2","- [a]: /u\n      \nnext\n",e);
 t("refdef ws list tab","- [a]: /u\n  \t\n",e);
 t("refdef ws list3","* item\n* [a]: /u\n      \n* third\n",e);
 t("refdef ws top","[a]: /u\n    \nfoo [a]\n",e);
 t("refdef ws top tab","[a]: /u\n\t\nfoo [a]\n",e);
 t("refdef ws bq","> [a]: /u\n>     \n> foo\n",e);
 t("refdef title ws","[a]: /u \"t\"\n    \nfoo\n",e);
 t("refdef ws ol","1. [a]: /u\n       \n",e);
 t("refdef ws crlf list","- [a]: /u\r\n      \r\n",e);
 t("deflist refdef","x\n: [a]: /u\n      \n",Options::ENABLE_DEFINITION_LIST);
 t("fn refdef ws","[^n]: [a]: /u\n        \n",Options::ENABLE_FOOTNOTES);
 println!("offset iter:"); let r=std::panic::catch_unwind(||Parser::new("- [a]: /u\n      \n").into_offset_iter().count()); println!("{:?}",r.is_err());
 // table cell cap
 let mut s=String::new(); s.push_str(&"|h".repeat(1000)); s.push_str("|\n"); s.push_str(&"|-".repeat(1000)); s.push_str("|\n"); for i in 0..300 { s.push_str(&format!("|r{}\n",i)); } s.push_str("\nafter *x*\n");
 let evs:Vec<_>=Parser::new_ext(&s,tb).collect();
 let n=evs.iter().filter(|e|matches!(e,Event::Start(Tag::TableRow))).count(); println!("rows={} (300 written)",n);
 let mut depth=0; let mut inside=vec![]; for ev in &evs { match ev { Event::Start(Tag::Table(_))=>depth+=1, Event::End(pulldown_cmark::TagEnd::Table)=>depth-=1, Event::Start(Tag::Paragraph) if depth>0 =>inside.push("para-in-table"), _=>{} } }
 println!("paragraphs nested in table: {}", inside.len());
 let hh=h(&s,tb); println!("html tail: {:?}",&hh[hh.len()-260..]);
 // hard breaks with tabs
 t("hard tabs","foo\t\t\nbar\n",e); t("hard mix","foo \t\nbar\n",e); t("hard one tab sp","foo\t \nbar\n",e);
 // collapsed offsets
 ev("collapsed","[foo][] x\n\n[foo]: /u\n",e);
 ev("collapsed img","![foo][] x\n\n[foo]: /u\n",e);
 ev("shortcut","[foo] x\n\n[foo]: /u\n",e);
 let mut sp=vec![]; { let mut cb=|b:BrokenLink|{sp.push((b.span.clone(),b.link_type,b.reference.to_string()));None};
   let src="[foo][] [bar][baz] [q] ![i][]"; for _ in Parser::new_with_broken_link_callback(src,e,Some(&mut cb)){} } println!("broken spans {:?}",sp);
 // old footnote containers
 t("oldfn bq","> [^a]:\n> x\n",Options::ENABLE_OLD_FOOTNOTES);
 t("oldfn plain","[^a]:\nx\n",Options::ENABLE_OLD_FOOTNOTES);
 t("oldfn list","- [^a]:\n  x\n",Options::ENABLE_OLD_FOOTNOTES);
 // smart
 t("smart dash","a--b---c\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart apos","it's 'q' \"d\" rock'n'roll\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart emph quote","*'a'*\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart code","\"`a`\"\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart link title","[a](b \"it's\") ...\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart ellipsis 4","a....b\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart backslash","a\\--b \\... \\'c\\'\n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart autolink","<http://a--b...c> \n",Options::ENABLE_SMART_PUNCTUATION);
 t("smart heading attr","# a--b {#x--y}\n",Options::ENABLE_SMART_PUNCTUATION|Options::ENABLE_HEADING_ATTRIBUTES);
 t("smart in table","|a--b|\n|-|\n",Options::ENABLE_SMART_PUNCTUATION|tb);
 // label
 t("label nonascii",&format!("[{0}]\n\n[{0}]: /u\n","é".repeat(600)),e);
 t("label nonascii 999",&format!("[{0}]\n\n[{0}]: /u\n","日".repeat(400)),e);
 // eof
 t("fence eof","```\nabc",e); t("indent eof","    abc",e); t("html eof","<div>",e);
 // math
 let m=Options::ENABLE_MATH;
 t("math spaces","$ a $ $a $ $ a$\n",m); t("math esc","$\\$a$\n",m); t("math bq","> $a\n> b$\n",m); t("math disp","$$\na\n$$\n",m); t("math disp2","$$a$$ b $$c$$\n",m); t("math in emph","*$a*b$*\n",m); t("math in link","[$a](b)$]\n",m); t("math table","|$a\\|b$|\n|-|\n",m|tb); t("math code","`$a`$\n",m); t("math money","cost $5 and $6\n",m); t("math backslash end","$a\\$\n",m); t("math crlf disp","$$a\r\nb$$\r\n",m);
 // tables
 t("table pipe row","| a |\n|---|\n|\n| b |\n",tb);
 t("table esc","| a\\|b | `c\\|d` |\n|---|---|\n",tb);
 t("table no lead","a\n-\n",tb);
 t("table header mismatch","a|b\n-\n",tb);
 t("table lazy bq","> a|b\n> -|-\nc|d\n",tb);
 t("table followed","a|b\n-|-\nc\n# h\nd|e\n",tb);
 t("table html row","a|b\n-|-\n<div>\n",tb);
 t("table trailing","|a|b|\n|-|-|\n|c|d|e|\n",tb);
 t("table img pipe","a\n|b|\n|-|\n",tb);
 t("table setext","a|b\n---\n",tb);
 t("table delim spaces","a|b\n- |   -\n",tb);
 t("table emph cross","|*a|b*|\n|-|-|\n",tb);
 t("table code cross","|`a|b`|\n|-|-|\n",tb);
 t("table link cross","|[a|b](c)|\n|-|-|\n",tb);
 t("table refdef","a|b\n-|-\n[r]: /u\n\n[r]\n",tb);
 // tasklists
 let tl=Options::ENABLE_TASKLISTS;
 t("task tab","- [\t] a\n",tl); t("task nonspace","- [x]a\n",tl); t("task empty","- [x]\n- [ ]\n",tl); t("task bq","> - [x] a\n",tl); t("task para","- a\n\n  [x] b\n",tl); t("task nested","- - [x] a\n",tl); t("task ol","1. [X] a\n",tl); t("task loose","- [x] a\n\n- [ ] b\n",tl);t("task heading","- [x] # a\n",tl);
 // gfm
 let g=Options::ENABLE_GFM;
 t("gfm lower","> [!note]\n> a\n",g); t("gfm text","> [!NOTE] a\n",g); t("gfm second","> a\n> [!NOTE]\n",g); t("gfm nested","> > [!TIP]\n> > a\n",g); t("gfm lazy","> [!TIP]\na\n",g); t("gfm list","> [!TIP]\n> - a\n",g); t("gfm in list","- > [!TIP]\n  > a\n",g); t("gfm bad","> [!FOO]\n> a\n",g);
 t("gfm blank","> [!TIP]\n>\n> a\n",g);
 // deflist
 let d=Options::ENABLE_DEFINITION_LIST;
 t("dl basic","t\n: d\n",d); t("dl multi","t\n: d\n: e\n\nu\n: f\n",d); t("dl para after","t\n: d\n\nplain\n",d); t("dl multi title","a\nb\n: d\n",d); t("dl indent","t\n  : d\n    more\n",d); t("dl code","t\n:     code\n",d); t("dl no title",": d\n",d); t("dl list","- t\n  : d\n",d); t("dl lazy","t\n: d\nlazy\n",d); t("dl loose","t\n\n: d\n\n: e\n",d); t("dl heading","# t\n: d\n",d); t("dl table","a|b\n-|-\n: d\n",d|tb);t("dl bq","> t\n> : d\n",d);t("dl after","t\n: d\n# h\n: x\n",d);
 t("dl colon text","a: b\n:c\n",d);
 // heading attr
 let ha=Options::ENABLE_HEADING_ATTRIBUTES;
 t("ha setext multi","a {#x}\nb {#y}\n===\n",ha); t("ha code","# `a {#x}`\n",ha); t("ha esc","# a \\{#x}\n",ha); t("ha closing","# a {#x} #\n",ha); t("ha closing2","# a # {#x}\n",ha); t("ha nbsp","# a {#\u{a0}x}\n",ha); t("ha empty","# {#x}\n",ha); t("ha dup","# a {#x #y .c .c}\n",ha); t("ha link","# [a](b) {#x}\n",ha); t("ha html","# <a {#x}>\n",ha); t("ha setext crlf","a {#x}\r\n===\r\n",ha); t("ha trailing tab","# a {#x}\t\n",ha);t("ha bq setext","> a\n> b {#x}\n> ===\n",ha);t("ha quote","# a {#\"x\" k=\"v w\"}\n",ha);
 t("empty heading closer text","# *a* #\n",e); ev("empty text","# *a* #\n",e);
 // misc
 t("nul","a\0b\n",e); t("bom","\u{feff}# a\n",e); t("nbsp list","-\u{a0}a\n",e); t("vt","a\x0bb\n",e); t("entity nul","&#0;&#xD800;&#1114112;\n",e);
 t("link angle nl","[a](<b\nc>)\n",e); t("link dest ctl","[a](b\x7fc)\n",e); t("img alt nest","![a *b* [c](d) `e`](f)\n",e);
 t("html comment dash","a <!-- b -- c --> d\n",e);t("html empty comment","a <!--> b <!---> c\n",e);
 t("backslash eof","a\\",e);t("heading backslash","# a\\\nb\n",e);t("heading backslash attr","# a\\\nb\n",ha);
 t("ol big","1234567890. a\n123456789. b\n",e);t("ol zero","0. a\n",e);
 t("hrule","* * *\n- - -\n_ _ _\n***a\n",e);t("hr vt","--- \x0c\n",e);
}
