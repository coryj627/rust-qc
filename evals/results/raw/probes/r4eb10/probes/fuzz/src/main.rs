use pulldown_cmark::*;
use std::panic;
use std::time::Instant;
thread_local! {static LOC:std::cell::RefCell<String>=std::cell::RefCell::new(String::new());}
fn check(s2: &str, o: Options) {
    let mut depth = 0i64;
    let mut stack = vec![];
    let mut rs: Vec<std::ops::Range<usize>> = vec![];
    let nest = std::env::var("NEST").is_ok();
    for (e, rg) in Parser::new_ext(s2, o).into_offset_iter() {
        if nest {
            if let Some(p) = rs.last() {
                if !matches!(e, Event::End(_)) {
                    assert!(p.start <= rg.start && rg.end <= p.end, "NOTNESTED parent={} child={}", tagname(stack.last()), evname(&e));
                }
            }
            match &e {
                Event::Start(_) => rs.push(rg.clone()),
                Event::End(_) => { let q = rs.pop().unwrap(); assert!(q == rg, "ENDDIFF {}", tagname(stack.last())); }
                _ => {}
            }
        }
        assert!(rg.start <= rg.end && rg.end <= s2.len(), "BADRANGE {:?}", rg);
        
        match &e {
            Event::Start(t) => {
                depth += 1;
                stack.push(t.to_end())
            }
            Event::End(t) => {
                depth -= 1;
                let x = stack.pop().expect("UNBALANCED");
                assert!(x == *t, "MISMATCH");
            }
            _ => {}
        }
    }
    assert!(depth == 0, "DEPTH");
    let mut h = String::new();
    html::push_html(&mut h, Parser::new_ext(s2, o));
    let c: Vec<Event> = Parser::new_ext(s2, o).collect();
    let d: Vec<Event> = Parser::new_ext(s2, o).into_offset_iter().map(|x| x.0).collect();
    assert!(c == d, "OFFSETDIFF");
}
fn tagname(t: Option<&TagEnd>) -> String { format!("{:?}", t).chars().take(24).collect() }
fn evname(e: &Event) -> String { let d = format!("{:?}", e); d.split(|c: char| c == '{' || c == '(').take(2).collect::<Vec<_>>().join("/") }
fn sig(s: &str, o: Options) -> Option<String> {
    let s = s.to_string();
    let r = panic::catch_unwind(move || check(&s, o));
    match r {
        Ok(_) => None,
        Err(p) => {
            let m = p
                .downcast_ref::<String>()
                .cloned()
                .or(p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let m = if m.starts_with("BADRANGE") { "BADRANGE".to_string() } else { m };
            let l = LOC.with(|c| c.borrow().clone());
            let m: String = m.chars().take(90).collect();
            let _ = m; Some(format!("{}", l))
        }
    }
}
fn minimize(s: &str, o: Options, k: &str) -> (String, Options) {
    let mut cur: Vec<char> = s.chars().collect();
    let mut sz = (cur.len() / 2).max(1);
    loop {
        let mut i = 0;
        let mut ch = false;
        while i < cur.len() {
            let e = (i + sz).min(cur.len());
            let mut t = cur.clone();
            t.drain(i..e);
            let ts: String = t.iter().collect();
            if sig(&ts, o).as_deref() == Some(k) {
                cur = t;
                ch = true;
            } else {
                i += sz;
            }
        }
        if !ch {
            if sz == 1 {
                break;
            }
            sz /= 2;
        }
    }
    let mut o2 = o;
    let ts: String = cur.iter().collect();
    for b in 0..16 {
        let f = Options::from_bits_truncate(1 << b);
        if o2.contains(f) {
            let t = o2 - f;
            if sig(&ts, t).as_deref() == Some(k) {
                o2 = t;
            }
        }
    }
    (ts, o2)
}
struct R(u64);
impl R {
    fn n(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn b(&mut self, m: usize) -> usize {
        (self.n() % (m as u64)) as usize
    }
}
const F: &[&str] = &[
    "*", "**", "_", "__", "~", "~~", "^", "`", "``", "```", "~~~", "$", "$$", "[", "]", "(", ")", "![", "[^", "[^a]",
    "[^a]:", "]:", "[[", "]]", "|", "\\|", "\\", "\n", "\n\n", "\r\n", "\r", " ", "  ", "    ", "\t", ">", "> ", ">>",
    "- ", "* ", "+ ", "1. ", "2) ", "- [ ] ", "- [x] ", ": ", ":", "#", "# ", "## ", "{#a}", "{.b}", " {", "}", "=",
    "===", "---", "- - -", "<", "</", "<a>", "</a>", "<div>", "</div>", "<!--", "-->", "<?", "?>", "<![CDATA[", "]]>",
    "<!X", "<script>", "</script>", "<pre>", "&amp;", "&#35;", "&#x22;", "&", "a", "b", "foo", "é", "漢", "😀",
    "\u{a0}", "\u{2003}", "'", "\"", "...", "--", "---", "+++", "[a]: /u", "[a]", "[a][]", "[a](/u)", "(/u \"t\")",
    "<http://a.b>", "<a@b.c>", "|a|b|\n", "|-|-|\n", ":-:", "[!NOTE]", "[!WARNING]", "\n    ", "\n> ", "\n- ", "\n\t",
    "\n  ", "[[a|b]]", "[[a]]", "\\\n", "  \n", "\u{0}", "\u{b}", "\u{c}", "`a`", "$x$", "$$x$$", "~a~", "^a^", "\n: ",
    "\n---\n", "\n+++\n",
];
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a[1].parse().unwrap();
    let iters: u64 = a[2].parse().unwrap();
    let mut r = R(seed * 2654435761 + 88172645463325252);
    panic::set_hook(Box::new(|i| {
        let l = i.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        LOC.with(|c| *c.borrow_mut() = l);
    }));
    let mut seen = std::collections::HashSet::new();
    for i in 0..iters {
        let len = 1 + r.b(if i % 8 == 0 { 40 } else if i % 2 == 0 { 14 } else { 7 });
        let mut s = String::new();
        for _ in 0..len {
            s.push_str(F[r.b(F.len())]);
        }
        let bits = if r.b(2) == 0 {
            r.n() as u32 & 0xffff
        } else if r.b(2) == 0 {
            (1u32 << (1 + r.b(15))) | (1u32 << (1 + r.b(15)))
        } else {
            Options::all().bits() & !(if r.b(2) == 0 { Options::ENABLE_OLD_FOOTNOTES.bits() } else { 0 })
        };
        let o = Options::from_bits_truncate(bits);
        let t = Instant::now();
        if let Some(k) = sig(&s, o) {
            if seen.insert(k.clone()) {
                let (m, o2) = minimize(&s, o, &k);
                println!("FAIL {} opts={:#x} min={:?}", k, o2.bits(), m);
            }
        }
        let el = t.elapsed().as_millis();
        if el > 500 {
            println!("SLOW {}ms {:?}", el, s);
        }
    }
    println!("done");
}
