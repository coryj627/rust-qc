use pulldown_cmark::{html, Event, Options, Parser, Tag, BrokenLink};
fn render(s: &str, o: Options) -> String {
    let mut out = String::new(); html::push_html(&mut out, Parser::new_ext(s,o)); out
}
fn plain(s: &str) -> String { render(s,Options::empty()) }
#[test] fn crlf_code() { assert_eq!(plain("`a\r\nb`"), "<p><code>a b</code></p>\n"); }
#[test] fn trailing_tab_fence() {
    for fence in ["```", "~~~"] { let s=format!("{fence}\nx\n{fence}\t\nafter\n"); assert_eq!(plain(&s), "<pre><code>x\n</code></pre>\n<p>after</p>\n"); }
}
#[test] fn case_footnote() {
 let out=render("ref [^FOO] [^foo]\n\n[^foo]: Note\n",Options::ENABLE_FOOTNOTES);
 println!("{out}"); assert!(!out.contains("#FOO")); assert_eq!(out.matches(">1</a>").count(),2);
}
#[test] fn unicode_case_footnote() {
 let out=render("ref [^STRASSE] [^straße]\n\n[^straße]: Note\n",Options::ENABLE_FOOTNOTES);
 println!("{out}"); assert_eq!(out.matches("href=\"#straße\"").count(),2); assert_eq!(out.matches(">1</a>").count(),2);
}
#[test] fn repeated_footnotes() {
 let out=render("first[^a] second[^a] third[^b]\n\n[^a]: A\n\n[^b]: B\n",Options::ENABLE_FOOTNOTES);
 println!("{out}"); assert_eq!(out.matches(">1</a>").count(),2); assert_eq!(out.matches(">2</a>").count(),1); assert!(!out.contains(">3</a>"));
}
#[test] fn short_writes() {
 struct Short(Vec<u8>); impl std::io::Write for Short {
 fn write(&mut self,b:&[u8])->std::io::Result<usize> {let n=b.len().min(2);self.0.extend_from_slice(&b[..n]); Ok(n)}
 fn flush(&mut self)->std::io::Result<()> {Ok(())}
 }
 let input="3. café & tea\n"; let mut short=Short(Vec::new()); html::write_html_io(&mut short,Parser::new(input)).unwrap();
 assert_eq!(String::from_utf8(short.0).unwrap(),plain(input));
}
#[test] fn cr_blocks() {
 for s in ["    x\n\nafter\n", "```rust\nx\n```\n", "<pre>\nx\n</pre>\n\nafter\n"] {
   let expected=plain(s); let cr=s.replace('\n',"\r"); let actual=plain(&cr).replace("\r\n","\n").replace('\r',"\n");
   assert_eq!(actual,expected,"{s:?}");
 }
}
#[test] fn cr_heading_attrs() {
 let input="# heading\r\rpara\r";assert_eq!(render(input,Options::ENABLE_HEADING_ATTRIBUTES),"<h1>heading</h1>\n<p>para</p>\n");
}
fn label_resolves(c: &str, n:usize) -> bool {
 let label=c.repeat(n); let input=format!("[text][{label}]\n\n[{label}]: /url\n"); render(&input, Options::empty()).contains("href=\"/url\"")
}
#[test] fn label_unicode_boundary() { assert!(label_resolves("é",499)); assert!(label_resolves("é",500)); }
#[test] fn label_emoji_boundary() { assert!(label_resolves("🦀",249)); assert!(label_resolves("🦀",250)); }
#[test] fn label_cjk_boundary() { assert!(label_resolves("界",334)); }
#[test] fn label_ascii_boundary() { assert!(label_resolves("a",999)); assert!(!label_resolves("a",1000)); }
#[test] fn wiki_ranges() {
 for s in ["[[Wiki]]", "[[Wiki|text]]", "![[foo.png]]"] {
   let ranges:Vec<_>=Parser::new_ext(s,Options::ENABLE_WIKILINKS).into_offset_iter().filter(|(e,_)|matches!(e,Event::Start(Tag::Link{..}|Tag::Image{..})|Event::End(pulldown_cmark::TagEnd::Link|pulldown_cmark::TagEnd::Image))).collect();
   println!("{s:?}: {ranges:?}"); assert_eq!(ranges.len(),2); for (_,r) in ranges {assert_eq!(r,0..s.len());}
 }
}
#[test] fn wiki_empty_alias() { assert_eq!(render("[[Wiki|]]",Options::ENABLE_WIKILINKS),"<p><a href=\"Wiki\"></a></p>\n"); }
#[test] fn heading_single_attribute() {
 let s="# heading {a bb c=d}"; let out=render(s, Options::ENABLE_HEADING_ATTRIBUTES); assert_eq!(out,"<h1 a=\"\" bb=\"\" c=\"d\">heading</h1>\n");
}
#[test] fn nul_normalized() {assert_eq!(plain("a\0b"),"<p>a�b</p>\n");}
#[test] fn tab_fence_info() {assert_eq!(plain("```rust\tcustom\nx\n```"),"<pre><code class=\"language-rust\">x\n</code></pre>\n");}
#[test] fn html_block_uppercase_close() {
 for (open,close) in [("script","SCRIPT"),("pre","PRE")] { let s=format!("<{open}>\nx\n</{close}>\n\nafter\n"); let out=plain(&s); assert!(out.ends_with("<p>after</p>\n"),"{out:?}"); }
}
#[test] fn html_block_other_close() {let out=plain("<pre>\nx\n</style>\n\nafter\n");assert!(out.ends_with("<p>after</p>\n"),"{out:?}");}
#[test] fn short_table_row() {
 let input="| a | b | c |\n|---|---|---|\n| one |\n";
 let out=render(input,Options::ENABLE_TABLES); println!("{out}"); assert_eq!(out.matches("<td>").count(),3);
 let cells=Parser::new_ext(input,Options::ENABLE_TABLES).filter(|e| matches!(e,Event::Start(Tag::TableCell))).count();assert_eq!(cells,6);
}
#[test] fn heading_tab_close() {
 for s in ["# foo\t###\t\n", "# foo\t###\n", "# foo ###\t\n"] {assert_eq!(plain(s),"<h1>foo</h1>\n");}
}
#[test] fn title_requires_separator() {
 assert_eq!(plain("[a](<url> \"title\")"),"<p><a href=\"url\" title=\"title\">a</a></p>\n");
 assert_eq!(plain("[a](<url>\"title\")"),"<p>[a](&lt;url&gt;\"title\")</p>\n");
}
#[test] fn sub_super_interference() {
 let s="^a b~ c^";let control=render(s,Options::ENABLE_SUPERSCRIPT);assert_eq!(control,"<p><sup>a b~ c</sup></p>\n");assert_eq!(render(s,Options::ENABLE_SUPERSCRIPT|Options::ENABLE_SUBSCRIPT),control);
}
#[test] fn super_strike_interference() {
 let s="~~a b^ c~~";let control=render(s,Options::ENABLE_STRIKETHROUGH);assert_eq!(control,"<p><del>a b^ c</del></p>\n");assert_eq!(render(s,Options::ENABLE_SUPERSCRIPT|Options::ENABLE_STRIKETHROUGH),control);
}
#[test] fn linked_wiki_image() { assert_eq!(render("[![[foo.png]]](bar)",Options::ENABLE_WIKILINKS),"<p><a href=\"bar\"><img src=\"foo.png\" alt=\"foo.png\" /></a></p>\n");}
#[test] fn defined_link_callback() {
 let mut calls=0;let p=Parser::new_with_broken_link_callback("[ok][known]\n\n[known]: /defined \"Defined\"\n",Options::empty(),Some(|_:BrokenLink<'_>|{calls+=1;Some(("/fallback".into(),"Fallback".into()))}));
 let mut out=String::new();html::push_html(&mut out,p);assert_eq!(calls,0);assert_eq!(out,"<p><a href=\"/defined\" title=\"Defined\">ok</a></p>\n");
}
#[test] fn subscript_only_retains_inlines() {
 for s in ["~~*a*~~", "~~[a](b)~~", "~~`a`~~"] {assert_eq!(render(s,Options::ENABLE_SUBSCRIPT),plain(s));}
}
#[test] fn nested_image_alt() {assert_eq!(plain("![before *em* ![inner](inner.png) after](outer.png) tail"),"<p><img src=\"outer.png\" alt=\"before em inner after\" /> tail</p>\n");}
#[test] fn tabs_not_hard_breaks() {for s in ["foo\t\t\nbar", "foo \t\nbar", "foo\t \nbar"] {assert_eq!(plain(s),"<p>foo\nbar</p>\n");}}
#[test] fn html_unquoted_tab() {let s="a <b x=one\ty=two>z</b>";assert_eq!(plain(s),format!("<p>{s}</p>\n"));}
#[test] fn table_internal_colon() {let s="a | b\n-:- | ---\nc | d\n";assert_eq!(render(s,Options::ENABLE_TABLES),plain(s));}
#[test] fn smart_punctuation_behavior() {assert_eq!(render("a -- ---",Options::ENABLE_SMART_PUNCTUATION),"<p>a – —</p>\n");}
