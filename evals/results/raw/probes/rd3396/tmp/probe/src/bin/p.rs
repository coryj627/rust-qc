use pulldown_cmark::*;
use std::io::Read;
fn opts(s:&str)->Options{ let mut o=Options::empty(); for t in s.split(','){ o|=match t {
"t"=>Options::ENABLE_TABLES,"f"=>Options::ENABLE_FOOTNOTES,"of"=>Options::ENABLE_OLD_FOOTNOTES,"s"=>Options::ENABLE_STRIKETHROUGH,"tl"=>Options::ENABLE_TASKLISTS,"sp"=>Options::ENABLE_SMART_PUNCTUATION,"ha"=>Options::ENABLE_HEADING_ATTRIBUTES,"y"=>Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,"pl"=>Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,"m"=>Options::ENABLE_MATH,"g"=>Options::ENABLE_GFM,"dl"=>Options::ENABLE_DEFINITION_LIST,"sup"=>Options::ENABLE_SUPERSCRIPT,"sub"=>Options::ENABLE_SUBSCRIPT,"w"=>Options::ENABLE_WIKILINKS,"all"=>Options::all(),"-"=>Options::empty(),x=>panic!("opt {x}")};} o}
fn main(){ let mut s=String::new(); std::fs::File::open(std::env::args().nth(1).unwrap()).unwrap().read_to_string(&mut s).unwrap();
 let verbose = std::env::args().nth(2).is_some();
 for line in s.lines(){ if line.trim().is_empty()||line.starts_with("##"){continue;} let (o,inp)=line.split_once('\t').expect("tab");
  let inp=inp.replace("\\n","\n").replace("\\r","\r").replace("\\t","\t");
  let op=opts(o); println!("=== [{o}] {inp:?}");
  let r=std::panic::catch_unwind(||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&inp,op)); println!("{h:?}");
   if verbose { for (e,r) in Parser::new_ext(&inp,op).into_offset_iter(){ println!("  {:?} {:?} {:?}",r.clone(),inp.get(r.clone()),e);} }
  }); if r.is_err(){println!("PANIC");}
 }}
