use pulldown_cmark::{html, Options, Parser, Event, CowStr, TextMergeStream};
fn h(s: &str, o: Options) -> String { let mut b=String::new(); html::push_html(&mut b, Parser::new_ext(s,o)); b }
fn t(s:&str,o:Options){println!("{:?} => {:?}",s,h(s,o));}
fn ev(s:&str,o:Options){ println!("events {:?}",s); for (e,r) in Parser::new_ext(s,o).into_offset_iter(){println!("  {:?} {:?} src={:?}",r,e,s.get(r.clone()));} }
fn main(){
 let e=Options::empty();
 t("foo </a\n     > bar\n",e);
 t("foo <a\n     > bar\n",e);
 ev("- a\n\t- b\n",e);
 t("> t\n>\n> : d\n",Options::ENABLE_DEFINITION_LIST);
 t("t\n\n: d\n",Options::ENABLE_DEFINITION_LIST);
 t("- [r]: /u\n- b\n",e);
 t("\u{feff}---\na: 1\n---\n",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
 t("`a\0b` c\0d\n",e);
 t("![[a|]] t\n",Options::ENABLE_WIKILINKS);
 t("[[a]|]] t\n",Options::ENABLE_WIKILINKS);
 // Text merge with empty event pairs
 let v:Vec<Event>=TextMergeStream::new(vec![Event::Text(CowStr::Borrowed(""))].into_iter()).collect(); println!("{:?}",v);
 // tight list + refdef-only item
 ev("- [a]: /u\n",e);
 t("[a]: /u\n     \n[a]\n",e);
 t("[a]: /u\n     x\n",e);
 t("* a\n* [b]: /u\n      \n",Options::ENABLE_TABLES);
}
