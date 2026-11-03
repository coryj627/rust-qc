// Time to_owned_value / to_borrowed_value on an object whose keys are all the same,
// against one with the same number of distinct keys. Release build.
use std::time::Instant;
fn build(n: usize, dup: bool) -> Vec<u8> {
    let mut s = String::with_capacity(n * 16);
    s.push('{');
    for i in 0..n {
        if i > 0 { s.push(','); }
        if dup { s.push_str("\"k\":1"); } else { s.push_str(&format!("\"k{i}\":1")); }
    }
    s.push('}');
    s.into_bytes()
}
fn best(n: usize, dup: bool, owned: bool) -> (f64, usize) {
    let input = build(n, dup);
    let mut t = f64::MAX;
    for _ in 0..3 {
        let mut d = input.clone();
        let st = Instant::now();
        if owned {
            let v = simd_json::to_owned_value(&mut d).unwrap();
            std::hint::black_box(&v);
            let e = st.elapsed().as_secs_f64();
            drop(v);
            t = t.min(e);
        } else {
            let v = simd_json::to_borrowed_value(&mut d).unwrap();
            std::hint::black_box(&v);
            let e = st.elapsed().as_secs_f64();
            drop(v);
            t = t.min(e);
        }
    }
    (t, input.len())
}
fn main() {
    let sizes = [10_000usize, 20_000, 40_000, 80_000, 160_000];
    let mut exit = 0;
    for &(label, owned) in &[("to_owned_value", true), ("to_borrowed_value", false)] {
        println!("{label}: keys  input_bytes  dup_secs  distinct_secs  dup/distinct");
        let mut last: Option<f64> = None;
        let mut ratios = vec![];
        let mut final_factor = 0.0;
        for &n in &sizes {
            let (td, b) = best(n, true, owned);
            let (tu, _) = best(n, false, owned);
            let g = last.map(|l| td / l).unwrap_or(0.0);
            println!("  {n:7} {b:9} {td:9.4} {tu:9.4} {:9.1}  growth_on_doubling={g:.2}", td / tu);
            if last.is_some() { ratios.push(g); }
            last = Some(td);
            final_factor = td / tu;
        }
        let last2 = &ratios[ratios.len() - 2..];
        let quad = last2.iter().all(|g| *g > 2.8);
        println!("  last two doublings grew {:.2} and {:.2}; quadratic={quad}; dup/distinct at top size={final_factor:.1}", last2[0], last2[1]);
        if !quad || final_factor < 10.0 { exit = 1; }
    }
    std::process::exit(exit);
}
