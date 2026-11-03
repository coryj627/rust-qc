use super::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn load() -> Vec<String> {
  serde_json::from_str(&fs::read_to_string("/work/tmp/fz/corpus.json").unwrap()).unwrap()
}

fn fmt_of(src: &str) -> Option<String> {
  let tokens = Lexer::test_lex(src).ok()?;
  let ast = Parser::parse_tokens(&mut Numerator::new(), &tokens).ok()?;
  Some(ast.color_display(Color::never()).to_string())
}

fn json_of(src: &str) -> Option<serde_json::Value> {
  let j = Compiler::test_compile(src).ok()?;
  serde_json::to_value(&j).ok()
}

fn strip(v: &mut serde_json::Value) {
  // ignore fields that carry no semantics for fmt
  if let serde_json::Value::Object(m) = v { m.remove("warnings"); for (_k, x) in m.iter_mut() { strip(x) } }
  if let serde_json::Value::Array(a) = v { for x in a { strip(x) } }
}

#[test]
fn fmt_roundtrip() {
  let corpus = load();
  let mut bad = 0;
  let mut ok = 0;
  for src in &corpus {
    let r = catch_unwind(AssertUnwindSafe(|| {
      let Some(f1) = fmt_of(src) else { return None };
      let mut out = Vec::new();
      let Some(mut j1) = json_of(src) else { return None };
      match fmt_of(&f1) {
        None => out.push("REPARSE-FAIL"),
        Some(f2) => { if f1 != f2 { out.push("NOT-IDEMPOTENT") } }
      }
      match json_of(&f1) {
        None => out.push("RECOMPILE-FAIL"),
        Some(mut j2) => { strip(&mut j1); strip(&mut j2); if j1 != j2 { out.push("JSON-DIFF") } }
      }
      Some((out, f1))
    }));
    match r {
      Err(_) => { println!("PANIC on:\n{src}\n-----"); bad += 1 }
      Ok(None) => {}
      Ok(Some((out, f1))) => {
        ok += 1;
        if !out.is_empty() { bad += 1; if bad < 40 { println!("{out:?} on:\n{src}\n--formatted--\n{f1}\n-----"); } }
      }
    }
  }
  println!("corpus={} compiled={ok} bad={bad}", corpus.len());
}

struct R(u64);
impl R { fn n(&mut self, m: usize) -> usize { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; (self.0 % (m as u64)) as usize } }

const FRAGS: &[&str] = &["\n","  ","\t","{{","}}","(",")","[","]","'","\"","`","'''","\"\"\"","```","x\"","f'","f\"","f'''","\\","\\\n","\\u{1F600}","\\u{","#","#!","@","-","?","+","*","$",":",":=","::","=","==","!=","=~","!~","&&","||","/","if ","else ","{","}","assert(","export ","eager ","alias ","set ","mod ","mod? ","import ","import? ","unexport ","a","b","foo","é","日本","\r\n","\r",",","[private]","[script]","[script('sh')]","[group('a')]","[doc('x')]","[arg('a', long)]","[arg('a', short='b')]","[arg('a', value='b')]","[arg('a', pattern='x')]","[working-directory('a')]","[env('a','b')]","[confirm]","[confirm('a')]","[linux]","[default]","[parallel]","[extension('x')]","set shell := ['a']","set lazy","set guards","set unstable", "f(a) := a", "f('a')","f()","os()","env('a','b')", "a := 'b'", "\n\n", "\u{feff}", "\u{0}", "{{{{"];

#[test]
fn mutate() {
  let corpus = load();
  let mut r = R(0x1234_5678_9abc_def1);
  let n: usize = std::env::var("N").ok().and_then(|s| s.parse().ok()).unwrap_or(300000);
  let mut seen = BTreeSet::new();
  std::panic::set_hook(Box::new(|_| {}));
  let mut compiled = 0;
  let mut fmt_bad = BTreeSet::new();
  for _ in 0..n {
    let mut s = corpus[r.n(corpus.len())].clone();
    if s.len() > 600 { let mut e = 600; while !s.is_char_boundary(e) { e += 1 } s.truncate(e); }
    for _ in 0..(1 + r.n(4)) {
      let mut p = r.n(s.len() + 1);
      while !s.is_char_boundary(p) { p -= 1 }
      match r.n(4) {
        0 | 1 => s.insert_str(p, FRAGS[r.n(FRAGS.len())]),
        2 => { let mut e = (p + 1 + r.n(6)).min(s.len()); while !s.is_char_boundary(e) { e += 1 } s.replace_range(p..e, ""); }
        _ => { let mut e = p; while e < s.len() && !s.is_char_boundary(e) { e += 1 } let f = FRAGS[r.n(FRAGS.len())]; s.insert_str(e, f); }
      }
    }
    let res = catch_unwind(AssertUnwindSafe(|| {
      let _ = Compiler::test_compile(&s).map(|j| { let _ = serde_json::to_string(&j); for r in j.recipes.values() { let _ = r.color_display(Color::never()).to_string(); } });
      // error rendering
      if let Err(e) = Compiler::test_compile(&s) { let _ = e.color_display(Color::never()).to_string(); }
      fmt_of(&s).map(|f| { let diff = match (json_of(&s), json_of(&f)) { (Some(mut a), Some(mut b)) => { strip(&mut a); strip(&mut b); a != b } _ => false }; (f.clone(), fmt_of(&f), json_of(&s).is_some() && json_of(&f).is_none(), diff) })
    }));
    match res {
      Err(p) => {
        let msg = p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        let key: String = msg.chars().take(70).collect();
        if seen.insert(key.clone()) { println!("PANIC {key:?}\n<<<{s}>>>\n"); }
      }
      Ok(Some((f, again, recompile_bad, diff))) => {
        compiled += 1;
        let tag = match again { None => "REPARSE", Some(ref g) if *g != f => "NONIDEM", _ if recompile_bad => "RECOMPILE", _ if diff && !s.contains("eager") && !s.contains("[private]") && !s.contains("unexport") => "JSONDIFF", _ => "" };
        if !tag.is_empty() && !(s.contains("mod") && s.contains("[doc")) && fmt_bad.len() < 25 && fmt_bad.insert(s.clone()) { println!("{tag}\n<<<{s}>>>\n---\n{f}\n=====\n"); }
      }
      Ok(None) => {}
    }
  }
  println!("done n={n} parsed={compiled} panics={} fmtbad={}", seen.len(), fmt_bad.len());
}

#[test]
fn exhaustive() {
  std::panic::set_hook(Box::new(|_| {}));
  let alpha: Vec<char> = " \n\ta:\\{}'\"#\r=(`f[]".chars().collect();
  let mut seen = BTreeSet::new();
  let maxlen: usize = std::env::var("L").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
  let mut count = 0u64;
  for len in 0..=maxlen {
    let mut idx = vec![0usize; len];
    loop {
      let s: String = idx.iter().map(|&i| alpha[i]).collect();
      count += 1;
      for src in [s.clone(), format!("a:\n  {s}"), format!("x := {s}")] {
        let r = catch_unwind(AssertUnwindSafe(|| {
          match Compiler::test_compile(&src) { Err(e) => { let _ = e.color_display(Color::never()).to_string(); } Ok(_) => {} }
        }));
        if let Err(p) = r {
          let msg = p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
          let key: String = msg.chars().take(60).collect();
          if seen.insert(key.clone()) { println!("PANIC {key:?} on {src:?}"); }
        }
      }
      let mut k = 0;
      while k < len { idx[k] += 1; if idx[k] == alpha.len() { idx[k] = 0; k += 1 } else { break } }
      if k == len { break }
    }
  }
  println!("exhaustive count={count} panics={}", seen.len());
}

#[test]
fn invocations() {
  std::panic::set_hook(Box::new(|_| {}));
  let src = "[arg('o', long, short='o')]\n[arg('f', long, value='on')]\nfoo o='d' f='off' a='1' *rest:\n  echo\nbar x y='2':\n  echo\nbaz +v:\n\nalias q := bar\n";
  let j = Compiler::test_compile(src).unwrap();
  let toks = ["foo","bar","baz","q","--o","-o","--o=1","--f","--f=1","--","-","a","","::","foo::","::foo","bar::x","--=","-=","-é","--é","=","a:b",":"];
  let mut r = R(0xdead_beef_cafe_f00d);
  let mut seen = BTreeSet::new();
  for _ in 0..1_000_000 {
    let n = r.n(6);
    let args: Vec<&str> = (0..n).map(|_| toks[r.n(toks.len())]).collect();
    let res = catch_unwind(AssertUnwindSafe(|| { let _ = InvocationParser::parse_invocations(&j, &args).map_err(|e| e.color_display(Color::never()).to_string()); }));
    if let Err(p) = res {
      let msg = p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
      if seen.insert(msg.clone()) { println!("PANIC {msg:?} on {args:?}"); }
    }
  }
  println!("invocations panics={}", seen.len());
  // unindent exhaustive
  let alpha: Vec<char> = " \t\n\rae\u{e9}".chars().collect();
  let mut bad = 0;
  for len in 0..=8usize {
    let mut idx = vec![0usize; len];
    loop {
      let s: String = idx.iter().map(|&i| alpha[i]).collect();
      if catch_unwind(|| unindent(&s)).is_err() { if bad < 5 { println!("UNINDENT PANIC {s:?}"); } bad += 1; }
      let mut k = 0;
      while k < len { idx[k] += 1; if idx[k] == alpha.len() { idx[k] = 0; k += 1 } else { break } }
      if k == len { break }
    }
  }
  println!("unindent bad={bad}");
}
