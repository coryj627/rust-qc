use simd_json::prelude::*;
struct R(u64);
impl R { fn n(&mut self)->u64{ let mut x=self.0; x^=x<<13; x^=x>>7; x^=x<<17; self.0=x; x } fn b(&mut self,m:u64)->u64{ self.n()%m } }
fn cvt(v:&simd_json::OwnedValue)->serde_json::Value{
  use simd_json::OwnedValue as O; use simd_json::StaticNode as S;
  match v { O::Static(S::Null)=>serde_json::Value::Null, O::Static(S::Bool(b))=>(*b).into(), O::Static(S::I64(i))=>(*i).into(), O::Static(S::U64(i))=>(*i).into(),
    O::Static(S::F64(f))=>serde_json::json!(*f),
    #[allow(unreachable_patterns)] O::Static(_)=>serde_json::Value::String("big".into()),
    O::String(s)=>s.clone().into(), O::Array(a)=>serde_json::Value::Array(a.iter().map(cvt).collect()),
    O::Object(o)=>serde_json::Value::Object(o.iter().map(|(k,v)|(k.clone(),cvt(v))).collect()) }
}
fn gen_str(r:&mut R,out:&mut String){
  out.push('"');
  let n=match r.b(5){0=>r.b(5),1=>r.b(40),2=>28+r.b(10),3=>60+r.b(10),_=>r.b(200)};
  for _ in 0..n { match r.b(30){
     0=>out.push_str("\\\\"),1=>out.push_str("\\\""),2=>out.push_str("\\n"),3=>out.push_str("\\/"),4=>out.push_str("\\b"),5=>out.push_str("\\f"),6=>out.push_str("\\r"),7=>out.push_str("\\t"),
     8=>out.push_str("\\u00e9"),9=>out.push_str("\\u0041"),10=>out.push_str("\\ud83d\\ude00"),11=>out.push_str("\\u20ac"),12=>out.push('é'),13=>out.push('€'),14=>out.push('😀'),15=>out.push_str("\\u0000"),16=>out.push('{'),17=>out.push(','),18=>out.push(']'),
     _=>out.push((b'a'+r.b(26) as u8) as char) } }
  out.push('"');
}
static SHORT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn gen_num(r:&mut R,out:&mut String){
  if SHORT.load(std::sync::atomic::Ordering::Relaxed) { if r.b(3)==0 {out.push('-');} let n=r.n()%1000000000; out.push_str(&n.to_string()); if r.b(2)==0 { out.push('.'); out.push_str(&(r.b(100000)).to_string()); } if r.b(3)==0 { out.push_str("e"); out.push_str(&r.b(30).to_string()); } return; }
  if r.b(3)==0 {out.push('-');}
  let il=match r.b(4){0=>1,1=>r.b(5)+1,2=>r.b(20)+1,_=>r.b(45)+1};
  if r.b(6)==0 {out.push('0');} else { out.push((b'1'+r.b(9) as u8) as char); for _ in 1..il {out.push((b'0'+r.b(10) as u8) as char);} }
  if r.b(2)==0 { out.push('.'); let fl=match r.b(3){0=>r.b(4)+1,1=>r.b(20)+1,_=>r.b(70)+1}; if r.b(4)==0 { for _ in 0..r.b(30){out.push('0');} } for _ in 0..fl {out.push((b'0'+r.b(10) as u8) as char);} }
  if r.b(3)==0 { out.push(if r.b(2)==0{'e'}else{'E'}); match r.b(3){0=>out.push('-'),1=>out.push('+'),_=>{}}; let e=match r.b(4){0=>r.b(10),1=>r.b(30),2=>r.b(330),_=>r.b(400)}; out.push_str(&e.to_string()); }
}
fn gen(r:&mut R,d:u32,out:&mut String){
  let ws=|r:&mut R,out:&mut String|{ for _ in 0..(if r.b(4)==0 {r.b(4)} else {0}) { out.push([' ','\n','\t','\r'][r.b(4) as usize]); } };
  ws(r,out);
  match if d>5 {r.b(5)} else {r.b(8)} {
    0=>out.push_str("null"),1=>out.push_str("true"),2=>out.push_str("false"),3=>gen_str(r,out),4=>gen_num(r,out),
    5=>{ out.push('['); let n=r.b(6); for i in 0..n { if i>0 {out.push(',');} gen(r,d+1,out);} ws(r,out); out.push(']'); }
    _=>{ out.push('{'); let n=r.b(6); for i in 0..n { if i>0 {out.push(',');} ws(r,out); out.push('"'); out.push_str(&format!("k{}",i)); if r.b(5)==0 {out.push_str("\\n");} out.push('"'); ws(r,out); out.push(':'); gen(r,d+1,out);} ws(r,out); out.push('}'); }
  }
  ws(r,out);
}
fn has_dup(v:&simd_json::OwnedValue)->bool{ use simd_json::OwnedValue as O; match v { O::Array(a)=>a.iter().any(has_dup), O::Object(o)=>{ let mut h=std::collections::HashSet::new(); o.iter().any(|(k,v)|!h.insert(k.clone())||has_dup(v)) }, _=>false } }
fn norm(v:&serde_json::Value)->serde_json::Value{v.clone()}
fn eqv(a:&serde_json::Value,b:&serde_json::Value)->bool{ use serde_json::Value as V; match (a,b){ (V::Number(x),V::Number(y))=>{ let (p,q)=(x.as_f64().unwrap(),y.as_f64().unwrap()); p==q || ((p-q)/q).abs()<1e-12 }, (V::Array(x),V::Array(y))=>x.len()==y.len()&&x.iter().zip(y).all(|(m,n)|eqv(m,n)), (V::Object(x),V::Object(y))=>x.len()==y.len()&&x.iter().all(|(k,m)|y.get(k).is_some_and(|n|eqv(m,n))), _=>a==b } }
fn main(){
  let mode=std::env::args().nth(1).unwrap();
  let seed:u64=std::env::args().nth(2).unwrap().parse().unwrap();
  let iters:u64=std::env::args().nth(3).unwrap().parse().unwrap();
  let mut r=R(seed|1); for _ in 0..10 {r.n();}
  let mut fails=0;
  if mode=="float" {
    for _ in 0..iters {
      let mut s=String::new(); gen_num(&mut r,&mut s);
      let is_float=s.contains('.')||s.contains('e')||s.contains('E');
      if !is_float {continue;}
      let maxpad:u64=std::env::args().nth(4).map(|x|x.parse().unwrap()).unwrap_or(140); let pad=if maxpad==0 {0} else {r.b(maxpad)} as usize;
      let doc=format!("{}{}"," ".repeat(pad),s);
      let exp=s.parse::<f64>().unwrap();
      let mut b=doc.as_bytes().to_vec();
      match simd_json::to_owned_value(&mut b) {
        Ok(v)=>{ let g=v.as_f64(); let ok=g.map(|x|x.to_bits()==exp.to_bits()).unwrap_or(false);
          if !ok { fails+=1; if fails<25 {println!("FLOAT pad={} {} got {:?} expected {:?}",pad,s,g,exp);} } }
        Err(e)=>{ if !exp.is_infinite() { fails+=1; if fails<25 {println!("FLOATERR pad={} {} {:?} expected {:?}",pad,s,e,exp);} } }
      }
    }
    println!("float mismatches {fails}");
  } else {
    let mut okc=0;let mut errboth=0; SHORT.store(true,std::sync::atomic::Ordering::Relaxed);
    for _ in 0..iters {
      let mut s=String::new(); gen(&mut r,0,&mut s);
      let mutate=mode=="mut";
      let mut bytes=s.into_bytes();
      if mutate && !bytes.is_empty() { for _ in 0..1+r.b(2) { let i=r.b(bytes.len() as u64) as usize; match r.b(4){0=>{bytes.remove(i); if bytes.is_empty(){bytes.push(b'1');}},1=>{bytes.insert(i,b"{}[],:\"\\0-.eE +ntf\n"[r.b(19) as usize]);},2=>{bytes[i]=b"{}[],:\"\\0-.eE +ntf\n"[r.b(19) as usize];},_=>{bytes.truncate(i.max(1));}} } }
      let Ok(s)=String::from_utf8(bytes.clone()) else {continue};
      let sj:Result<serde_json::Value,_>=serde_json::from_str(&s);
      let mut b=bytes.clone();
      let si=simd_json::to_owned_value(&mut b);
      if let Ok(a)=&si { if has_dup(a) {continue;} }
      if std::env::var("NOSUR").is_ok() && s.contains("\\ud8") {continue;}
      let mut b2=bytes.clone();
      let sb=simd_json::to_borrowed_value(&mut b2).map(|v|simd_json::OwnedValue::from(v));
      if si.is_ok()!=sb.is_ok() || (si.is_ok() && si.as_ref().unwrap()!=sb.as_ref().unwrap()) {println!("OWN/BORROW {:?}",s);}
      match (&si,&sj){
        (Ok(a),Ok(c))=>{ okc+=1; let a2=cvt(a); if !eqv(&a2,c) {
            // tolerate big numbers
            fails+=1; if fails<30 {println!("DIFF {:?}\n  simd={}\n  sj={}",s,a2,c);} } }
        (Err(_),Err(_))=>{errboth+=1;}
        (Ok(a),Err(e))=>{ fails+=1; if fails<30 {println!("ACCEPT {:?} simd={:?} sj={}",s,a,e);} }
        (Err(e),Ok(c))=>{ let _=c; fails+=1; if fails<30 {println!("REJECT {:?} simd={:?}",s,e);} }
      }
      // tape encode roundtrip
      if let Ok(a)=&si { let mut b3=bytes.clone(); let t=simd_json::to_tape(&mut b3).unwrap(); let enc=t.as_value().encode(); let mut e=enc.clone().into_bytes(); match simd_json::to_owned_value(&mut e){ Ok(v)=>{ if &v!=a {println!("TAPE-ROUNDTRIP {:?} -> {:?}",s,enc);} }, Err(er)=>println!("TAPE-ENC-INVALID {:?} {:?} {:?}",s,enc,er)}
         let enc=a.encode(); let mut e=enc.clone().into_bytes(); match simd_json::to_owned_value(&mut e){ Ok(v)=>{ if &v!=a {println!("OWNED-ROUNDTRIP {:?} -> {:?}",s,enc);} }, Err(er)=>println!("OWNED-ENC-INVALID {:?} {:?} {:?}",s,enc,er)}
         let enc=simd_json::to_string(a).unwrap(); let mut e=enc.clone().into_bytes(); match simd_json::to_owned_value(&mut e){ Ok(v)=>{ if &v!=a {println!("SER-ROUNDTRIP {:?} -> {:?}",s,enc);} }, Err(er)=>println!("SER-ENC-INVALID {:?} {:?} {:?}",s,enc,er)}
         let enc=simd_json::to_string_pretty(a).unwrap(); let mut e=enc.clone().into_bytes(); match simd_json::to_owned_value(&mut e){ Ok(v)=>{ if &v!=a {println!("PP-ROUNDTRIP {:?} -> {:?}",s,enc);} }, Err(er)=>println!("PP-ENC-INVALID {:?} {:?} {:?}",s,enc,er)}
         let enc=a.encode_pp(); let mut e=enc.clone().into_bytes(); match simd_json::to_owned_value(&mut e){ Ok(v)=>{ if &v!=a {println!("ENCPP-ROUNDTRIP {:?} -> {:?}",s,enc);} }, Err(er)=>println!("ENCPP-INVALID {:?} {:?} {:?}",s,enc,er)}
         // serde path
         let mut b4=bytes.clone(); match simd_json::from_slice::<simd_json::OwnedValue>(&mut b4){ Ok(v)=>{ if &v!=a {println!("SERDE-DIFF {:?}",s);} }, Err(er)=>println!("SERDE-ERR {:?} {:?}",s,er)}
         let mut b5=bytes.clone(); match simd_json::from_slice::<serde_json::Value>(&mut b5){ Ok(v)=>{ if !eqv(&v,&cvt(a)) {println!("SERDE-SJ-DIFF {:?}",s);} }, Err(er)=>println!("SERDE-SJ-ERR {:?} {:?}",s,er)}
      }
    }
    println!("mode {mode}: ok {okc} errboth {errboth} mismatches {fails}");
  }
}
