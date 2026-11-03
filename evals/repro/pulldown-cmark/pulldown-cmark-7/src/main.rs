use pulldown_cmark::{Event, Options, Parser, Tag};
fn main() {
    let s = "For a new ferrari, [[Wikientry|click here]]!";
    let want = 19..43;
    println!("input {s:?}; the wikilink source text is {:?} = {want:?}", &s[want.clone()]);
    let evs: Vec<_> = Parser::new_ext(s, Options::ENABLE_WIKILINKS).into_offset_iter().collect();
    for (e, r) in &evs { println!("{r:?} {e:?}"); }
    let got = evs.iter().find_map(|(e, r)| matches!(e, Event::Start(Tag::Link { .. })).then(|| r.clone())).unwrap();
    println!("wikilink range: got {got:?} = {:?}, want {want:?}", &s[got.clone()]);
    std::process::exit(if got != want { 0 } else { 1 });
}
