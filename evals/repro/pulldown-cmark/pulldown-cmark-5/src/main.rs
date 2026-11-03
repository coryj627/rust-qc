use pulldown_cmark::{Options, Parser};
use std::time::Instant;
fn run(s: &str, o: Options) -> (usize, f64) {
    let mut best = f64::MAX; let mut n = 0;
    for _ in 0..3 {
        let t = Instant::now();
        n = Parser::new_ext(s, o).count();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    (n, best)
}
fn curve(make: impl Fn(usize) -> String, sizes: &[usize], o: Options, min_ratio: f64) -> ! {
    println!("{:>9} {:>11} {:>10} {:>12} {:>8}", "n", "bytes", "events", "best_ms", "ratio");
    let (mut last, mut prev) = (0.0, 0.0);
    for &k in sizes {
        let s = make(k);
        let (e, ms) = run(&s, o);
        let r = if prev > 0.0 { ms / prev } else { 0.0 };
        println!("{k:>9} {:>11} {e:>10} {ms:>12.2} {r:>8.2}", s.len());
        prev = ms; last = r;
    }
    println!("last doubling ratio {last:.2}; linear is 2, quadratic is 4; demonstrated if >= {min_ratio}");
    std::process::exit(if last >= min_ratio { 0 } else { 1 });
}

fn main() {
    println!("- x n, then a, then 2n blank lines, default options");
    curve(|n| "- ".repeat(n) + "a" + &"\n".repeat(2 * n), &[4000, 8000, 16000, 32000], Options::empty(), 3.0);
}
