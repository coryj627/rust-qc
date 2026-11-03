// Compile-level API checks and serde-free sanity
use pulldown_cmark::{Parser,InlineStr};
fn main(){
  let p=Parser::new("[a]: /u\n");
  let d=p.reference_definitions().get("a").unwrap();
  // A caller can hold the value only through inference; naming it would need pulldown_cmark::LinkDef.
  println!("{:?}",d);
  let e=InlineStr::try_from("0123456789012345678901234567890").unwrap_err();
  println!("{:?}",e);
}
