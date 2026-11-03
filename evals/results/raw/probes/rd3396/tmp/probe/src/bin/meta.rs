use pulldown_cmark::*; use std::collections::BTreeMap;
fn ex(path:&str)->Vec<String>{ let t=std::fs::read_to_string(path).unwrap(); let mut v=vec![]; let mut cur:Option<String>=None; let mut in_md=false;
 for l in t.lines(){ if l.starts_with("````````````````````````````````"){ if l.contains("example"){cur=Some(String::new());in_md=true;} else { if let Some(c)=cur.take(){v.push(c);} in_md=false; } continue;} if cur.is_some()&&in_md{ if l=="."{in_md=false;} else { let c=cur.as_mut().unwrap(); c.push_str(&l.replace('→',"\t")); c.push('\n'); } } } v }
fn html(s:&str,o:Options)->Option<String>{ std::panic::catch_unwind(||{let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(s,o)); h}).ok() }
fn squash(s:&str)->String{ s.replace("<p>","").replace("</p>","").replace('\n',"").replace("\r","") }
fn main(){ std::panic::set_hook(Box::new(|_|{})); let b="/work/crate/pulldown-cmark/";
 let f=Options::ENABLE_FOOTNOTES;
 let files:Vec<(String,Options)>=vec![("third_party/CommonMark/spec.txt".into(),Options::empty()),("third_party/GitHub/gfm_table.txt".into(),Options::ENABLE_TABLES),("specs/table.txt".into(),Options::ENABLE_TABLES),("specs/footnotes.txt".into(),f),("specs/old_footnotes.txt".into(),Options::ENABLE_OLD_FOOTNOTES),("specs/heading_attrs.txt".into(),Options::ENABLE_HEADING_ATTRIBUTES),("specs/math.txt".into(),Options::ENABLE_MATH|Options::ENABLE_TABLES),("specs/wikilinks.txt".into(),Options::ENABLE_WIKILINKS),("specs/definition_lists.txt".into(),Options::ENABLE_DEFINITION_LIST),("specs/blockquotes_tags.txt".into(),Options::ENABLE_GFM),("specs/super_sub.txt".into(),Options::ENABLE_SUPERSCRIPT|Options::ENABLE_SUBSCRIPT|Options::ENABLE_STRIKETHROUGH),("specs/strikethrough.txt".into(),Options::ENABLE_STRIKETHROUGH),("specs/regression.txt".into(),Options::ENABLE_TABLES|f|Options::ENABLE_STRIKETHROUGH|Options::ENABLE_TASKLISTS|Options::ENABLE_HEADING_ATTRIBUTES)];
 let which=std::env::args().nth(1).unwrap_or("crlf,cr,bq,li,nonl".into()); let lim:usize=std::env::args().nth(2).unwrap_or("6".into()).parse().unwrap();
 for t in which.split(','){ let mut tot=0; let mut fails:BTreeMap<String,Vec<(String,String,String)>>=BTreeMap::new();
  for (p,o) in &files{ let path=format!("{b}{p}"); if !std::path::Path::new(&path).exists(){println!("missing {p}");continue;} for m in ex(&path){ if m.contains('\t')&&t!="crlf"{continue;} tot+=1; let Some(base)=html(&m,*o) else {continue;};
    let (inp,exp,norm):(String,String,fn(&str)->String)=match t{
     "crlf"=>(m.replace('\n',"\r\n"),base.clone(),|s|s.replace("\r\n","\n")),
     "cr"=>(m.replace('\n',"\r"),base.clone(),|s|s.replace('\r',"\n")),
     "bq"=>(m.split_inclusive('\n').map(|l|format!("> {l}")).collect(),format!("<blockquote>\n{base}</blockquote>\n"),|s|s.to_string()),
     "li"=>{let mut first=true; (m.split_inclusive('\n').map(|l|{ if first{first=false;format!("- {l}")} else if l=="\n"{l.to_string()} else {format!("  {l}")}}).collect(),format!("<ul><li>{}</li></ul>",squash(&base)),|s|squash(s))},
     "nonl"=>(m.trim_end_matches('\n').to_string(),base.clone(),|s|s.trim_end_matches('\n').replace("\n</code>","</code>")),
     _=>panic!()};
    match html(&inp,*o){ None=>fails.entry(p.clone()).or_default().push((inp,"PANIC".into(),exp)), Some(h)=>{ if norm(&h)!=norm(&exp){ fails.entry(p.clone()).or_default().push((inp,norm(&h),norm(&exp))); } } }
  }}
  println!("##### {t}: {} failures of {tot}",fails.values().map(|v|v.len()).sum::<usize>());
  for (p,v) in fails{ println!("-- {p}: {}",v.len()); for (i,g,e) in v.iter().take(lim){ println!(" IN  {i:?}\n GOT {g:?}\n EXP {e:?}"); } }
 } }
