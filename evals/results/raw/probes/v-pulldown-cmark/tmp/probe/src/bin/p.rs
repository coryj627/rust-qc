use probe::*; use pulldown_cmark::*; use std::panic::*;
// cases on stdin-free file: opts<TAB>input-escaped ; prefix opts with "R:" for ranges
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let txt = std::fs::read_to_string(path).unwrap();
    std::panic::set_hook(Box::new(|_| {}));
    for line in txt.lines() {
        if line.is_empty() || line.starts_with("##") { if !line.is_empty(){println!("{line}");} continue; }
        let (o, inp) = line.split_once('\t').unwrap();
        let (ranges, o2) = match o.strip_prefix("R:") { Some(x) => (true, x), None => (false, o) };
        let s = unesc(inp);
        let op = opts(o2);
        println!("--- [{o}] {:?}", s);
        let s2 = s.clone();
        match catch_unwind(AssertUnwindSafe(|| html(op, &s2))) {
            Ok(h) => println!("  html: {:?}", h),
            Err(e) => println!("  PANIC html: {:?}", e.downcast_ref::<String>().map(|x|x.as_str()).or(e.downcast_ref::<&str>().copied())),
        }
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut out = vec![];
            for (ev, rg) in Parser::new_ext(&s, op).into_offset_iter() {
                out.push(format!("  {:?} {:?} {:?}", rg, ev, s.get(rg.clone())));
            }
            out
        }));
        match r {
            Ok(v) => if ranges { for l in v { println!("{l}"); } },
            Err(e) => println!("  PANIC offset: {:?}", e.downcast_ref::<String>().map(|x|x.as_str()).or(e.downcast_ref::<&str>().copied())),
        }
    }
}
