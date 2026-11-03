use pulldown_cmark::*; use std::collections::BTreeMap;
fn ex(path:&str)->Vec<String>{ let t=std::fs::read_to_string(path).unwrap(); let mut v=vec![]; let mut cur:Option<String>=None; let mut in_md=false;
 for l in t.lines(){ if l.starts_with("````````````````````````````````"){ if l.contains("example"){cur=Some(String::new());in_md=true;} else { if let Some(c)=cur.take(){v.push(c);} in_md=false; } continue;} if cur.is_some()&&in_md{ if l=="."{in_md=false;} else { let c=cur.as_mut().unwrap(); c.push_str(&l.replace('→',"\t")); c.push('\n'); } } } v }
fn main(){ let b="/work/crate/pulldown-cmark/"; let mut op=Options::all(); op.remove(Options::ENABLE_OLD_FOOTNOTES); op.insert(Options::ENABLE_FOOTNOTES); op.remove(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);op.remove(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);
 let mut seen:BTreeMap<String,(usize,String,String)>=BTreeMap::new();
 let mut note=|k:&str,m:&str,d:String|{let e=seen.entry(k.to_string()).or_insert((0,m.to_string(),d.clone())); e.0+=1; if m.len()<e.1.len(){e.1=m.to_string();e.2=d;}};
 for p in ["third_party/CommonMark/spec.txt","third_party/GitHub/gfm_table.txt","specs/table.txt","specs/footnotes.txt","specs/heading_attrs.txt","specs/math.txt","specs/wikilinks.txt","specs/definition_lists.txt","specs/blockquotes_tags.txt","specs/super_sub.txt","specs/strikethrough.txt","specs/regression.txt"]{
  for m in ex(&format!("{b}{p}")){ let mut stack:Vec<(TagEnd,std::ops::Range<usize>)>=vec![];
   let mut last_end_at_depth:Vec<usize>=vec![0];
   for (e,r) in Parser::new_ext(&m,op).into_offset_iter(){
     match &e{ Event::End(_)=>{ let (_,sr)=stack.pop().unwrap(); if sr!=r{note("end range differs from start",&m,format!("{:?} {:?} {:?}",e,sr,r));} last_end_at_depth.pop(); *last_end_at_depth.last_mut().unwrap()=r.end; continue; }, _=>{} }
     if let Some((t,pr))=stack.last(){ if r.start<pr.start||r.end>pr.end{ note(&format!("child outside parent: {:?} in {:?}",std::mem::discriminant(&e),t),&m,format!("{:?} {:?} parent {:?} {:?}",e,r,t,pr)); } }
     if r.start<*last_end_at_depth.last().unwrap(){ note("sibling overlaps previous",&m,format!("{:?} {:?} prev end {}",e,r,last_end_at_depth.last().unwrap())); }
     if let Event::Text(CowStr::Borrowed(s))=&e{ if *s!=&m[r.clone()]{ note("borrowed text != source slice",&m,format!("{:?} {:?} src {:?}",s,r,&m[r.clone()])); } }
     if let Event::Code(_)=&e{ if !m[r.clone()].starts_with('`')||!m[r.clone()].ends_with('`'){ note("code range lacks backticks",&m,format!("{:?} {:?} src {:?}",e,r,&m[r.clone()])); } }
     if let Event::Start(t)=&e{ stack.push((t.to_end(),r.clone())); last_end_at_depth.push(r.start); } else { *last_end_at_depth.last_mut().unwrap()=r.end; }
   } } }
 for (k,(n,m,d)) in seen{ println!("[{n}] {k}\n   in {m:?}\n   {d}"); }
}
