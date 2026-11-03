use pulldown_cmark::{Options, Parser};
use std::time::Instant;
// Events are capped with take() so that the demonstration cannot exhaust memory or time.
const CAP: usize = 20_000_000;
fn main() {
    let unit = "[[[[^(\n|]]]]=]]]]]]]]\n";
    println!("unit {:?} ({} bytes), ENABLE_WIKILINKS", unit, unit.len());
    println!("{:>6} {:>8} {:>12} {:>10} {:>7}", "units", "bytes", "events", "ms", "ratio");
    let (mut prev, mut grew) = (0usize, 0);
    for k in 1..=40 {
        let s = unit.repeat(k);
        let t = Instant::now();
        let n = Parser::new_ext(&s, Options::ENABLE_WIKILINKS).take(CAP).count();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        println!("{k:>6} {:>8} {n:>12} {ms:>10.2} {:>7.2}", s.len(), n as f64 / prev.max(1) as f64);
        if prev > 0 && n as f64 >= 1.8 * prev as f64 { grew += 1; }
        prev = n;
        if n >= CAP { println!("event cap of {CAP} reached at {} input bytes", s.len()); break; }
    }
    println!("steps with event count growth >= 1.8x: {grew}");
    std::process::exit(if prev >= CAP && grew >= 5 { 0 } else { 1 });
}
