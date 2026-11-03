// Model-based random op sequences on ZipWriter.
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions, CompressionMethod, AesMode};
struct Rng(u64);
impl Rng{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn r(&mut self,m:usize)->usize{(self.n()%(m.max(1) as u64)) as usize}}
fn main(){
    let seed:u64=std::env::args().nth(1).unwrap().parse().unwrap();
    let iters:u64=std::env::args().nth(2).unwrap().parse().unwrap();
    std::panic::set_hook(Box::new(|_|{}));
    let mut r=Rng(seed.wrapping_mul(0x9E3779B97F4A7C15)|1);
    let mut seen:HashSet<String>=HashSet::new();
    for _ in 0..iters {
        let n_ops=1+r.r(10);
        let mut log:Vec<String>=vec![];
        let mut ops=vec![]; for _ in 0..n_ops { ops.push((r.r(9),r.r(6),r.r(4),r.r(8),r.n())); }
        let res=catch_unwind(AssertUnwindSafe(||-> Result<(),String> {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            let mut model:BTreeMap<String,Vec<u8>>=BTreeMap::new();
            let mut last:Option<String>=None; let mut writing=false;
            let mut log2=vec![];
            for (op,nm,ma,opt,s) in &ops {
                let name=format!("f{nm}");
                let method=[CompressionMethod::Stored,CompressionMethod::Deflated,CompressionMethod::Bzip2,CompressionMethod::Zstd][*ma];
                let mut o=SimpleFileOptions::default().compression_method(method);
                if opt&1!=0 && std::env::var("LARGE").is_ok() {o=o.large_file(true);} if opt&2!=0 {o=o.with_alignment(1+(*s as u16%64));}
                let aes=opt&4!=0 && *s%3==0; if aes {o=o.with_aes_encryption(AesMode::Aes128,"p");}
                let data:Vec<u8>=(0..(*s%300)).map(|i|(i as u8).wrapping_mul(*s as u8)).collect();
                match op {
                    0|1|2=>{ log2.push(format!("start {name} {method:?} opt={opt} aes={aes} len={}",data.len()));
                        if w.start_file(name.clone(),o).is_ok(){ writing=true; w.write_all(&data).map_err(|e|format!("write {e}"))?; model.insert(name.clone(),if aes {vec![0xAA]} else {data.clone()}); last=Some(name);} else { writing=false; } }
                    3=>{ log2.push("abort".into()); if last.is_some() { if w.abort_file().is_ok(){ if let Some(l)=last.take(){ model.remove(&l);} } writing=false; last=None; } }
                    4=>{ log2.push(format!("dir {name}")); let dn=format!("{name}/"); if w.add_directory(name.clone(),SimpleFileOptions::default()).is_ok(){ model.insert(dn.clone(),vec![]); last=Some(dn);} writing=false; }
                    5=>{ log2.push(format!("sym {name}")); if w.add_symlink(name.clone(),"t",SimpleFileOptions::default()).is_ok(){ model.insert(name.clone(),b"t".to_vec()); last=Some(name);} writing=false; }
                    6=>{ let d=format!("d{nm}"); let src=model.keys().nth(*s as usize%model.len().max(1)).cloned(); if let Some(src)=src { log2.push(format!("deep {src} {d}")); if w.deep_copy_file(&src,&d).is_ok(){ let c=model[&src].clone(); model.insert(d.clone(),c); last=Some(d);} writing=false; } }
                    7=>{ let d=format!("s{nm}"); let src=model.keys().nth(*s as usize%model.len().max(1)).cloned(); if let Some(src)=src { log2.push(format!("shallow {src} {d}")); if w.shallow_copy_file(&src,&d).is_ok(){ let c=model[&src].clone(); model.insert(d.clone(),c); last=Some(d);} writing=false; } }
                    _=>{ log2.push("write-more".into()); if writing { if let Some(l)=&last { if let Some(m)=model.get_mut(l) { if m!=&vec![0xAA] { if w.write_all(&data).is_ok(){ m.extend(&data);} } else { let _=w.write_all(&data);} } } } }
                }
            }
            let _=writing;
            let v=w.finish().map_err(|e|format!("finish {e} :: {log2:?}"))?.into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).map_err(|e|format!("reopen {e} :: {log2:?}"))?;
            for (n,c) in &model {
                let aes=c==&vec![0xAA];
                let mut f=if aes {a.by_name_decrypt(n,b"p")} else {a.by_name(n)}.map_err(|e|format!("open-entry {e} :: {log2:?}"))?;
                let mut s=vec![]; f.read_to_end(&mut s).map_err(|e|format!("read-entry {e} :: {log2:?}"))?;
                if !aes && &s!=c { return Err(format!("content mismatch {n} :: {log2:?}")); }
            }
            if a.len()!=model.len() { return Err(format!("len {} vs model {} :: {log2:?}",a.len(),model.len())); }
            Ok(())
        }));
        log.clear();
        match res { Ok(Ok(()))=>{}, Ok(Err(e))=>{ let k=e.split("::").next().unwrap().chars().filter(|c|!c.is_ascii_digit()).collect::<String>(); if seen.insert(k){ println!("FAIL {}",e.chars().take(600).collect::<String>()); } }
            Err(p)=>{ let m=p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_default(); if seen.insert(m.clone()){ println!("PANIC {m} ops={ops:?}"); } } }
    }
    println!("done");
}
