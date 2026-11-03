use pulldown_cmark::*;
use std::panic;
fn examples(path:&str)->Vec<String>{
  let t=std::fs::read_to_string(path).unwrap(); let mut out=vec![]; let mut cur:Option<String>=None; let mut inmd=false;
  for l in t.lines(){
    if l.starts_with("````````````````````````````````") { if l.contains("example"){cur=Some(String::new()); inmd=true;} else if let Some(c)=cur.take(){ out.push(c);} continue; }
    if cur.is_some(){ if l=="." && inmd { inmd=false; continue;} if inmd { let c=cur.as_mut().unwrap(); c.push_str(&l.replace('→',"\t")); c.push('\n'); } }
  }
  out
}
fn evs(s:&str,o:Options)->Vec<String>{
  let v:Vec<Event>=TextMergeStream::new(Parser::new_ext(s,o)).collect();
  v.iter().filter(|e|!matches!(e,Event::Start(Tag::Paragraph)|Event::End(TagEnd::Paragraph))).map(|e|format!("{:?}",e).replace("Borrowed","X").replace("Boxed","X").replace("Inlined(InlineStr { inner: ","X(").to_string()).collect()
}
fn norm(v:Vec<Event>)->Vec<String>{ let _=v; vec![] }
fn txt(s:&str,o:Options)->String{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(s,o)); h }
fn main(){
  panic::set_hook(Box::new(|_|{}));
  let _=norm;
  let base="/work/crate/pulldown-cmark";
  let files=[("third_party/CommonMark/spec.txt",Options::empty()),("specs/table.txt",Options::ENABLE_TABLES),("specs/footnotes.txt",Options::ENABLE_FOOTNOTES),("specs/heading_attrs.txt",Options::ENABLE_HEADING_ATTRIBUTES),("specs/math.txt",Options::ENABLE_MATH),("specs/wikilinks.txt",Options::ENABLE_WIKILINKS),("specs/definition_lists.txt",Options::ENABLE_DEFINITION_LIST),("specs/super_sub.txt",Options::ENABLE_SUPERSCRIPT|Options::ENABLE_SUBSCRIPT),("specs/blockquotes_tags.txt",Options::ENABLE_GFM),("third_party/GitHub/gfm_table.txt",Options::ENABLE_TABLES),("specs/regression.txt",Options::all()-Options::ENABLE_OLD_FOOTNOTES)];
  for (f,o) in files {
    let ex=match std::fs::metadata(format!("{}/{}",base,f)){Ok(_)=>examples(&format!("{}/{}",base,f)),Err(_)=>{println!("skip {}",f);continue}};
    let mut res=[0usize;4]; let mut shown=[0usize;4]; let total=ex.len();
    for md in &ex {
      let o2=o;
      let b=txt(md,o);
      let variants:[(&str,String);4]=[("CRLF",md.replace('\n',"\r\n")),("CR",md.replace('\n',"\r")),("BQ",md.lines().map(|l|format!("> {}\n",l)).collect()),("LI",md.lines().enumerate().map(|(i,l)|if i==0{format!("1. {}\n",l)}else if l.is_empty(){"\n".to_string()}else{format!("   {}\n",l)}).collect())];
      for (i,(name,v)) in variants.iter().enumerate(){
        if v.is_empty(){continue}
        let m=md.clone(); let v2=v.clone(); let name=*name;
        let r=panic::catch_unwind(move||{
          match name {
            "CRLF"=>{ let a=txt(&v2,o2).replace("\r\n","\n"); let b=txt(&m,o2); a==b.replace("\r\n","\n") },
            "CR"=>{ let a=txt(&v2,o2).replace('\r',"\n"); let b=txt(&m,o2); a==b },
            "BQ"=>{ let a=txt(&v2,o2); let inner=txt(&m,o2); a==format!("<blockquote>\n{}</blockquote>\n",inner) },
            _=>{ let a=evs(&v2,o2); let inner=evs(&m,o2); a.len()>=4 && a[2..a.len()-2]==inner[..] },
          }
        });
        let bad=!matches!(r,Ok(true)); let pan=r.is_err();
        if bad { res[i]+=1; if shown[i]<40 && (i==2||i==3) && !md.contains('\t') && !md.contains("---\n") { shown[i]+=1; println!("[{}/{}{}] {:?}\n   base: {:?}\n   var : {:?}",f.split('/').last().unwrap(),name,if pan{" PANIC"}else{""},md,b,panic::catch_unwind(||0).map(|_|{ let v3=v.clone(); panic::catch_unwind(move||txt(&v3,o2)).unwrap_or("PANIC".into()) }).unwrap()); } }
      }
    }
    println!("=== {}: total {} mismatches CRLF={} CR={} BQ={} LI={}",f,total,res[0],res[1],res[2],res[3]);
  }
}
