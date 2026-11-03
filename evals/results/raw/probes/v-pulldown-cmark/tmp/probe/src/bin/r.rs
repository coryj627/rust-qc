use pulldown_cmark::*; use std::panic::*;
fn main(){
 std::panic::set_hook(Box::new(|_|{}));
 let header=format!("{}\n{}\n","|a".repeat(1000)+"|","|-".repeat(1000)+"|");
 for rows in [263,264,265,270,300]{
  for tail in ["\nafter *x*\n\n> quote\n","after\n","\n> quote\n\nafter\n"] {
  let s=format!("{header}{}{tail}","|c|\n".repeat(rows));
  let r=catch_unwind(AssertUnwindSafe(||{let mut n=0;let mut bad=0;let mut prev=vec![];for (e,rg) in Parser::new_ext(&s,Options::ENABLE_TABLES).into_offset_iter(){ n+=1; if rg.start>rg.end{bad+=1; if bad<3 {prev.push(format!("{:?} {:?}",rg,e));}} } (n,bad,prev)}));
  println!("rows {rows} tail {:?}: {:?}",tail,r.map_err(|e|e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s|s.to_string()))));
  }
 }
}
