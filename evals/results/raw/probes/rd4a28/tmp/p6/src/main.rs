// Differential fuzz of simd_json vs serde_json/std parse; no dependencies beyond those fetched.
use simd_json::prelude::*;
struct R(u64); impl R { fn n(&mut self)->u64{ let mut x=self.0; x^=x<<13; x^=x>>7; x^=x<<17; self.0=x; x } fn r(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize} }
fn conv(v:&simd_json::OwnedValue)->serde_json::Value{ use simd_json::OwnedValue as O; use simd_json::StaticNode as S; match v { O::Static(S::Null)=>serde_json::Value::Null, O::Static(S::Bool(b))=>(*b).into(), O::Static(S::I64(i))=>(*i).into(), O::Static(S::U64(i))=>(*i).into(), O::Static(S::F64(f))=>serde_json::json!(*f), O::String(s)=>s.clone().into(), O::Array(a)=>a.iter().map(conv).collect(), O::Object(o)=>{ let mut m=serde_json::Map::new(); for (k,v) in o.iter(){m.insert(k.clone(),conv(v));} m.into() } } }
fn numeq(a:&serde_json::Value,b:&serde_json::Value)->bool{ use serde_json::Value as V; match (a,b){ (V::Number(x),V::Number(y))=>{ if x==y {return true;} match (x.as_f64(),y.as_f64()){(Some(p),Some(q))=>p==q && (x.is_f64()==y.is_f64() || p==0.0),_=>false} }, (V::Array(x),V::Array(y))=>x.len()==y.len()&&x.iter().zip(y).all(|(p,q)|numeq(p,q)), (V::Object(x),V::Object(y))=>x.len()==y.len()&&x.iter().all(|(k,p)|y.get(k).is_some_and(|q|numeq(p,q))), _=>a==b } }
fn check(s:&[u8], cats:&mut std::collections::BTreeMap<String,(usize,String)>){
  let mut b=s.to_vec(); let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||simd_json::to_owned_value(&mut b).map_err(|e|format!("{:?}",e.error()))));
  let sj:Result<serde_json::Value,_>=serde_json::from_slice(s);
  let show=String::from_utf8_lossy(s).chars().take(90).collect::<String>();
  let mut note=|k:&str|{ let e=cats.entry(k.to_string()).or_insert((0,show.clone())); e.0+=1; if show.len()<e.1.len(){e.1=show.clone();} };
  match r { Err(_)=>note("PANIC"), Ok(Ok(v))=>{ match sj { Ok(j)=>{ let c=conv(&v); if !numeq(&c,&j) { note("value-diff"); } }, Err(e)=>{ let m=e.to_string(); let k=m.split(" at line").next().unwrap_or("").to_string(); note(&format!("simd-accepts sj-rejects: {k}")); } } }, Ok(Err(e))=>{ if sj.is_ok(){ note(&format!("simd-rejects sj-accepts: {e}")); } } }
}
fn main(){
  let mut cats=std::collections::BTreeMap::new();
  let mut rng=R(0x1234_5678_9abc_def1);
  // 1. exhaustive short over alphabet
  let alpha:&[u8]=b"[]{},:\"\\ 0189-+.eEtfnu\x00\n\xc3\xa9a";
  let al=alpha.len();
  for len in 1..=4 { let total=al.pow(len as u32); for mut i in 0..total { let mut v=Vec::with_capacity(len); for _ in 0..len { v.push(alpha[i%al]); i/=al; } check(&v,&mut cats); } }
  eprintln!("short done");
  // 2. number generator, with random leading whitespace offset
  let digs=b"0123456789";
  for _ in 0..700_000 {
    let mut s=String::new(); if rng.r(2)==0 {s.push('-');}
    let il=[1,1,2,5,16,17,18,19,20,21,25,40][rng.r(12)];
    for k in 0..il { let d=digs[rng.r(10)] as char; s.push(if k==0 && il>1 && d=='0' {'1'} else {d}); }
    if rng.r(3)>0 { s.push('.'); let fl=[1,2,3,8,9,15,16,17,18,19,20,30][rng.r(12)]; for _ in 0..fl { s.push(digs[rng.r(10)] as char);} }
    if rng.r(3)==0 { s.push(['e','E'][rng.r(2)]); match rng.r(3){0=>s.push('-'),1=>s.push('+'),_=>{}}; let e=[0,1,5,22,23,100,290,308,309,324,325,330,400][rng.r(13)]; s+=&e.to_string(); }
    let doc=match rng.r(4){0=>s.clone(),1=>format!("[{s}]"),2=>format!("{{\"k\":{s}}}"),_=>format!("{}{s}"," ".repeat(rng.r(80)+80))};
    let mut b=doc.clone().into_bytes(); let r=simd_json::to_owned_value(&mut b);
    let exact:Result<f64,_>=s.parse();
    let val=r.ok().map(|v|{ let v=match &v { simd_json::OwnedValue::Array(a)=>a[0].clone(), simd_json::OwnedValue::Object(o)=>o["k"].clone(), x=>x.clone()}; v });
    match (val,exact){ (Some(v),Ok(e))=>{ if let Some(f)=v.cast_f64(){ let isint=!s.contains(['.','e','E']); if isint { if v.as_f64().is_some() {} else if f!=e && !(s.len()>=19) { let k=cats.entry("int-diff".to_string()).or_insert((0,doc.clone())); k.0+=1; } } else if f.to_bits()!=e.to_bits() { let k=cats.entry("float-diff".to_string()).or_insert((0,format!("{doc} simd={f:e} std={e:e}"))); k.0+=1; if doc.len()<k.1.len() { k.1=format!("{doc} simd={f:e} std={e:e}"); } } } }, (None,Ok(e))=>{ if e.is_finite(){ let isint=!s.contains(['.','e','E']); if !isint { let k=cats.entry("float-rejected".to_string()).or_insert((0,doc.clone())); k.0+=1; } } }, _=>{} }
  }
  eprintln!("numbers done");
  // 3. boundary fuzz with mutation of realistic documents at 64-byte seams
  let seeds:Vec<String>=vec![r#"{"a":[1,2.5,-3e2,true,false,null,"s\"t\\r\né😀é😀"],"b":{"c":{}},"d":[],"é":"\\\\\\\""}"#.to_string(),"\"\\\\\\\\\\\\\\\"\\\\\"".to_string()];
  let muts:&[u8]=b"\\\"[]{},: \n\t0x\x00\x1f\x7f\xc3\xa9\xf0\x9f\x98\x80\xed\xa0\x80\xff";
  for _ in 0..700_000 {
    let mut v=seeds[rng.r(2)].clone().into_bytes(); let pad=rng.r(70); let mut d=vec![b' ';pad]; d.append(&mut v); let mut v=d; 
    if rng.r(3)==0 { let p=rng.r(v.len()); let n=rng.r(70); let ins=vec![b'x';n]; let q=v.iter().position(|c|*c==b'"').unwrap_or(0)+1; let _=p; v.splice(q..q,ins); }
    for _ in 0..rng.r(4) { let p=rng.r(v.len()); match rng.r(3){0=>{v[p]=muts[rng.r(muts.len())];},1=>{v.insert(p,muts[rng.r(muts.len())]);},_=>{ v.remove(p); if v.is_empty(){v.push(b' ');} }} }
    check(&v,&mut cats);
  }
  for (k,(n,ex)) in &cats { println!("{n:>9}  {k}  e.g. {ex:?}"); }
}
