use pulldown_cmark::{Options, Parser};
fn main(){ for n in [1usize,2,4,8,12,16,20,22,24]{ let s="[[a|]] ".repeat(n); let t=std::time::Instant::now(); let c=Parser::new_ext(&s,Options::ENABLE_WIKILINKS).take(200_000_000).count(); println!("n={n} bytes={} events={c} {:?}",s.len(),t.elapsed()); } }
