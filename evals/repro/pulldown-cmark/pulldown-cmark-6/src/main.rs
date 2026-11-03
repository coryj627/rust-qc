use pulldown_cmark::{Options, Parser};
fn main() {
    let doc = "%\n:\u{301}\n\t:";
    println!("doc {doc:?}, ENABLE_DEFINITION_LIST");
    let mut bad = 0;
    for (ev, r) in Parser::new_ext(doc, Options::ENABLE_DEFINITION_LIST).into_offset_iter() {
        let ok = doc.is_char_boundary(r.start) && doc.is_char_boundary(r.end);
        println!("{r:?} valid_str_slice={ok} {ev:?}");
        if !ok { bad += 1; }
    }
    println!("ranges that are not valid slices of the input: {bad}");
    if bad == 0 { std::process::exit(1); }
    let r = Parser::new_ext(doc, Options::ENABLE_DEFINITION_LIST).into_offset_iter()
        .map(|(_, r)| r).find(|r| !doc.is_char_boundary(r.start) || !doc.is_char_boundary(r.end)).unwrap();
    let c = std::panic::catch_unwind(|| doc[r.clone()].len());
    println!("slicing doc[{r:?}] panicked: {}", c.is_err());
    std::process::exit(if c.is_err() { 0 } else { 1 });
}
