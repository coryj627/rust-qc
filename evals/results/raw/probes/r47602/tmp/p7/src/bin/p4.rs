use simd_json::prelude::*;
use simd_json::OwnedValue;
use std::collections::BTreeMap;
struct R(u64);
impl R {
    fn n(&mut self) -> u64 { let mut x = self.0; x ^= x << 13; x ^= x >> 7; x ^= x << 17; self.0 = x; x }
    fn u(&mut self, m: usize) -> usize { (self.n() % m as u64) as usize }
    fn pick<'a, T>(&mut self, s: &'a [T]) -> &'a T { let i = self.u(s.len()); &s[i] }
}
fn digits(r: &mut R, n: usize) -> String { (0..n).map(|_| (b'0' + r.u(10) as u8) as char).collect() }
fn num(r: &mut R) -> String {
    let mut s = String::new();
    if r.u(3)==0 { s.push('-'); }
    let il = match r.u(4) {0=>1,1=>r.u(6)+1,2=>r.u(20)+1,_=>r.u(30)+1};
    let d = digits(r, il);
    if r.u(4)==0 { s.push('0'); } else { let t = d.trim_start_matches('0'); s.push_str(if t.is_empty(){"1"} else {t}); }
    if r.u(2)==0 { s.push('.'); let fl = match r.u(4) {0=>1,1=>r.u(8)+1,2=>r.u(20)+1,_=>r.u(40)+1}; if r.u(3)==0 { s.push_str(&"0".repeat(r.u(30))); } s.push_str(&digits(r, fl)); }
    if r.u(2)==0 { s.push(*r.pick(&['e','E'])); match r.u(3) {0=>s.push('-'),1=>s.push('+'),_=>{}}; let e = match r.u(3) {0=>r.u(10),1=>r.u(330),_=>r.u(400)}; s.push_str(&e.to_string()); }
    s
}
fn val(r: &mut R, d: usize, out: &mut String) {
    match r.u(if d>4 {7} else {10}) {
        0 => out.push_str("null"), 1 => out.push_str("true"), 2 => out.push_str("false"),
        3 | 4 => out.push_str(&num(r)),
        5 | 6 => { out.push('"'); let n=r.u(70); for _ in 0..n { match r.u(30) {0=>out.push_str("\\n"),1=>out.push_str("\\\""),2=>out.push_str("\\\\"),3=>out.push_str(*r.pick(&["\\u0041","\\u00e9","\\u20ac","\\ud83d\\ude00","\\/","\\b"])),4=>out.push('é'),5=>out.push('€'),6=>out.push('😀'),_=>out.push((b'a'+r.u(26) as u8) as char)} } out.push('"'); }
        7 => { out.push('['); let n=r.u(5); for i in 0..n { if i>0 {out.push(',');} ws(r,out); val(r,d+1,out); ws(r,out);} out.push(']'); }
        _ => { out.push('{'); let n=r.u(5); for i in 0..n { if i>0 {out.push(',');} ws(r,out); out.push_str(&format!("\"k{i}\"")); ws(r,out); out.push(':'); val(r,d+1,out);} out.push('}'); }
    }
}
fn ws(r:&mut R,out:&mut String){ if r.u(5)==0 { out.push_str(*r.pick(&[" ","\n","\t","\r","  "])); } }
fn conv(v:&serde_json::Value)->String{ serde_json::to_string(v).unwrap() }
fn strip(v:&serde_json::Value)->serde_json::Value{ use serde_json::Value as V; match v { V::Number(_)=>V::Null, V::Array(a)=>V::Array(a.iter().map(strip).collect()), V::Object(o)=>V::Object(o.iter().map(|(k,v)|(k.clone(),strip(v))).collect()), x=>x.clone() } }
fn main(){
    let mut r = R(0x1234_5678_9abc_def1);
    // 1. numbers no padding: compare to std parse
    let mut bad: BTreeMap<String,Vec<String>> = BTreeMap::new();
    let mut note = |k:&str, s:String| { let e = bad.entry(k.to_string()).or_default(); e.push(s); };
    let t = 400_000;
    for _ in 0..t {
        let s = num(&mut r);
        let mut b = s.clone().into_bytes();
        let res = simd_json::to_owned_value(&mut b);
        let isf = s.contains(['.','e','E']);
        let std = s.parse::<f64>().unwrap();
        match res {
            Ok(v) => {
                if isf {
                    let got = v.as_f64().unwrap_or(f64::NAN);
                    if got.to_bits()!=std.to_bits() { note("float_mismatch", format!("{s} simd={got:e} std={std:e}")); }
                } else {
                    let ok = v.as_u64().map(|x| x.to_string()==s).unwrap_or(false) || v.as_i64().map(|x| x.to_string()==s || (s=="-0" && x==0)).unwrap_or(false);
                    if !ok { note("int_mismatch", format!("{s} got {v:?}")); }
                }
            }
            Err(e) => {
                if isf && std.is_finite() { note("float_rejected", format!("{s} {e}")); }
                if !isf { let big = s.trim_start_matches('-').len()>=19; if !big { note("int_rejected", format!("{s} {e}")); } }
            }
        }
    }
    // 2. number in array context with trailing structural, u8 pads
    for _ in 0..100_000 {
        let s = num(&mut r);
        let d = format!("[{s},{s}]");
        let mut b = d.clone().into_bytes();
        let res = simd_json::to_owned_value(&mut b);
        let std = s.parse::<f64>().unwrap();
        if let Ok(v) = res { let a=v.as_array().unwrap(); let isf = s.contains(['.','e','E']); if isf { let x=a[0].as_f64().unwrap(); if x.to_bits()!=std.to_bits() { note("float_arr_mismatch", format!("{d}: {x:e} vs {std:e}")); } } }
    }
    // 3. doc differential to serde_json
    let mut docs=0; let mut both=0;
    for _ in 0..300_000 {
        let mut s=String::new(); val(&mut r,0,&mut s);
        // mutate sometimes
        let mut bytes = s.clone().into_bytes();
        let mutated = r.u(2)==0;
        if mutated && !bytes.is_empty() {
            for _ in 0..(r.u(3)+1) {
            let i=r.u(bytes.len());
            match r.u(4) { 0=>{bytes[i]= *r.pick(b"{}[],:\"\\0123456789eE.-+ tfn\x01\x00\x7f\xc3\xff");},1=>{bytes.remove(i); if bytes.is_empty(){bytes.push(b' ');}},2=>{bytes.insert(i,*r.pick(b"{}[],:\"\\0 x\x00"));},_=>{let j=r.u(bytes.len()); bytes.swap(i,j);} }
            }
        }
        docs+=1;
        let orig=bytes.clone(); if std::str::from_utf8(&orig).is_err() { continue; }
        let mut b=bytes.clone();
        let simd=simd_json::to_owned_value(&mut b);
        let sj: Result<serde_json::Value,_> = serde_json::from_slice(&orig);
        let show = String::from_utf8_lossy(&orig).into_owned();
        match (&simd,&sj) {
            (Ok(a),Ok(b)) => {
                both+=1;
                let sa: Result<serde_json::Value,_> = a.clone().try_into();
                match sa { Ok(sa)=> { if conv(&sa)!=conv(b) {
                    // tolerate key order; compare via serde_json Value eq
                    if strip(&sa)!=strip(b) { note("value_diff", format!("{show} simd={sa} sj={b}")); } } }, Err(_)=> note("conv_fail",show.clone()) }
                // roundtrip: encode
                let enc=a.encode();
                let re: Result<serde_json::Value,_> = serde_json::from_str(&enc);
                match re { Ok(re)=> { if strip(&re)!=strip(b) { note("encode_diff", format!("{show} -> {enc}")); } }, Err(e)=>note("encode_invalid", format!("{show} -> {enc} {e}")) }
                let enc=simd_json::to_string(a).unwrap();
                let re: Result<serde_json::Value,_> = serde_json::from_str(&enc);
                match re { Ok(re)=> { if strip(&re)!=strip(b) { note("to_string_diff", format!("{show} -> {enc}")); } }, Err(e)=>note("to_string_invalid", format!("{show} -> {enc} {e}")) }
                let enc=simd_json::to_string_pretty(a).unwrap();
                let re: Result<serde_json::Value,_> = serde_json::from_str(&enc);
                match re { Ok(re)=> { if strip(&re)!=strip(b) { note("pretty_diff", format!("{show} -> {enc}")); } }, Err(e)=>note("pretty_invalid", format!("{show} -> {enc} {e}")) }
                let enc=a.encode_pp();
                let re: Result<serde_json::Value,_> = serde_json::from_str(&enc);
                match re { Ok(re)=> { if strip(&re)!=strip(b) { note("encode_pp_diff", format!("{show} -> {enc}")); } }, Err(e)=>note("encode_pp_invalid", format!("{show} -> {enc} {e}")) }
                // tape encode, borrowed
                let mut b2=orig.clone();
                let t=simd_json::to_tape(&mut b2).unwrap();
                let enc=t.as_value().encode();
                let re: Result<serde_json::Value,_> = serde_json::from_str(&enc);
                match re { Ok(re)=> { if strip(&re)!=strip(b) { note("tape_encode_diff", format!("{show} -> {enc}")); } }, Err(e)=>note("tape_encode_invalid", format!("{show} -> {enc} {e}")) }
                // serde path
                let mut b3=orig.clone();
                let o: Result<OwnedValue,_> = simd_json::from_slice(&mut b3);
                match o { Ok(o)=> if &o!=a { note("serde_owned_diff",show.clone()); }, Err(e)=>note("serde_owned_err",format!("{show} {e}")) }
                let mut b4=orig.clone();
                let bv=simd_json::to_borrowed_value(&mut b4).unwrap();
                if a!=&bv { note("borrowed_diff",show.clone()); }
                // serde_json::Value via simd deserializer
                let mut b5=orig.clone();
                let o: Result<serde_json::Value,_> = simd_json::from_slice(&mut b5);
                match o { Ok(o)=> if strip(&o)!=strip(b) { note("serde_sjv_diff",format!("{show} {o}")); }, Err(e)=>note("serde_sjv_err",format!("{show} {e}")) }
            }
            (Ok(a),Err(e)) => { let t = show.clone(); let nul = t.contains('\0'); let lone = t.contains("\\ud") || t.contains("\\uD") ; let es=e.to_string(); let cls = if nul {"acc_nul"} else if lone && (es.contains("hex")||es.contains("surrogate")) {"acc_surrogate"} else if es.contains("trailing char")||es.contains("expected `,`") {"acc_trailing"} else {"acc_other"}; note(cls, format!("{show:?} simd={a:?} sj={e}")) },
            (Err(e),Ok(b)) => { let es=format!("{e}"); let big = es.starts_with("InvalidNumber"); note(if big {"rej_invalidnumber"} else {"rej_other"}, format!("{show:?} {e} sj={b}")) },
            _=>{}
        }
    }
    println!("docs {docs} both-accepted {both}");
    // 4. long string around boundaries
    for n in 0..200 { for off in 0..70 { for esc in ["\\n","\\u00e9","\\ud83d\\ude00","\\\\","\\\"","é","😀"] {
        let lit = format!("{}{}{}", "a".repeat(n), esc, "b".repeat(off));
        let d=format!("\"{lit}\"");
        let mut b=d.clone().into_bytes();
        let s=simd_json::to_owned_value(&mut b);
        let sj:serde_json::Value=serde_json::from_str(&d).unwrap();
        match s { Ok(v)=>{ if v.as_str().unwrap()!=sj.as_str().unwrap() { note("str_boundary", d.clone()); } }, Err(e)=>note("str_boundary_err", format!("{d} {e}")) }
    }}}
    // 6. input mutated on failure? Same input reparse
    for (k,v) in &bad {
        println!("=== {k}: {}", v.len());
        for s in v[..].iter().rev().take(if k=="acc_nul"||k=="rej_invalidnumber"||k=="acc_trailing"||k=="borrowed_diff"||k=="serde_owned_diff"{4}else{25}) { let s: String = s.chars().take(200).collect(); println!("   {s}"); }
    }
}
