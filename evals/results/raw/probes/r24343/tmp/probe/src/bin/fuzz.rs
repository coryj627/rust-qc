use probe::*;
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::sync::Mutex;
use zip::{ZipArchive, ZipWriter};
static LAST: Mutex<String> = Mutex::new(String::new());
struct Rng(u64);
impl Rng { fn n(&mut self)->u64{ let mut x=self.0; x^=x<<13; x^=x>>7; x^=x<<17; self.0=x; x } fn r(&mut self,m:usize)->usize{ (self.n()% (m as u64)) as usize } }
fn mutate(r:&mut Rng, d:&[u8])->Vec<u8>{ let mut v=d.to_vec(); if v.is_empty(){return v;}
    let k=1+r.r(4);
    for _ in 0..k { let i=r.r(v.len()); match r.r(6) {
        0=>v[i]=r.n() as u8, 1=>v[i]^=1<<r.r(8), 2=>{ for j in i..(i+4).min(v.len()){v[j]=0xff;} }, 3=>{ for j in i..(i+2).min(v.len()){v[j]=0;} },
        4=>{ let mg:&[&[u8]]=&[b"PK\x01\x02",b"PK\x03\x04",b"PK\x05\x06",b"PK\x06\x06",b"PK\x06\x07"]; let m=mg[r.r(5)]; for (j,b) in m.iter().enumerate(){ if i+j<v.len(){v[i+j]=*b;} } },
        _=>{ // target central fields
            if let Some(p)=v.windows(4).rposition(|w|w==b"PK\x01\x02"){ let f=[8usize,10,20,24,28,30,32,38,42][r.r(9)]; if p+f+1<v.len(){ let val=[0u16,1,2,3,4,5,6,8,9,12,14,93,95,98,99,0xffff,0xa000,0x8000][r.r(18)].to_le_bytes(); v[p+f]=val[0]; v[p+f+1]=val[1]; } }
        } } }
    v }
fn exercise(b:Vec<u8>, deep:bool){
    let mut c=Cursor::new(b.clone());
    { let mut c2=Cursor::new(b.clone()); for _ in 0..8 { match zip::read::read_zipfile_from_stream(&mut c2){ Ok(Some(mut f))=>{ let mut v=vec![]; let _=(&mut f).take(1<<20).read_to_end(&mut v); let _=f.options(); } _=>break } } }
    let Ok(mut a)=ZipArchive::new(&mut c) else { return };
    let _=a.decompressed_size();
    let _=a.root_dir(zip::read::root_dir_common_filter);
    let n=a.len().min(12);
    for i in 0..n {
        if let Ok(mut f)=a.by_index(i){ let mut v=vec![]; let _=(&mut f).take(1<<20).read_to_end(&mut v); let _=f.enclosed_name(); let _=f.mangled_name(); let _=f.options(); let _=f.is_symlink(); let _=f.extra_data_fields().count(); let _=f.last_modified(); }
        if let Ok(mut f)=a.by_index_decrypt(i,b"password"){ let mut v=vec![]; let _=(&mut f).take(1<<20).read_to_end(&mut v); }
        if let Ok(mut f)=a.by_index_seek(i){ let mut v=vec![]; let _=(&mut f).take(1<<16).read_to_end(&mut v); use std::io::{Seek,SeekFrom}; let _=f.seek(SeekFrom::End(-1)); let _=f.seek(SeekFrom::Current(5)); let _=f.read(&mut [0u8;4]); }
        let _=a.get_aes_verification_key_and_salt(i);
    }
    if deep {
        let _=a.has_overlapping_files();
        let mut w=ZipWriter::new(Cursor::new(vec![]));
        for i in 0..n { if let Ok(f)=a.by_index_raw(i){ if f.compressed_size()<(1<<20) { let _=w.raw_copy_file(f); } } }
        let _=w.finish();
        if let Ok(mut w)=ZipWriter::new_append(Cursor::new(b.clone())){ let _=w.start_file("appended",zip::write::SimpleFileOptions::default()); let _=w.write_all(b"x");
            let names:Vec<String>=w.get_comment().map(|_| vec![]).unwrap_or_default(); let _=names; let _=w.finish(); }
        let d=tempdir("fz"); let _=a.extract(&d); let _=std::fs::remove_dir_all(&d);
    }
}
fn main(){
    let args:Vec<String>=std::env::args().collect();
    if args[1]=="replay" { let b=std::fs::read(&args[2]).unwrap(); exercise(b,true); println!("replay ok"); return; }
    let seed:u64=args[1].parse().unwrap(); let secs:u64=args[2].parse().unwrap();
    std::panic::set_hook(Box::new(|i|{ let l=i.location().map(|l| format!("{}:{}",l.file(),l.line())).unwrap_or_default(); let m=i.payload().downcast_ref::<String>().cloned().or(i.payload().downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_default(); *LAST.lock().unwrap()=format!("{l} {}",m.chars().take(90).collect::<String>()); }));
    let mut fx:Vec<(String,Vec<u8>)>=vec![];
    for dir in ["/work/crate/tests/data","/work/crate/tests/data/legacy"] { for e in std::fs::read_dir(dir).unwrap(){ let p=e.unwrap().path(); if p.extension().is_some_and(|x|x=="zip") { let b=std::fs::read(&p).unwrap(); if b.len()<70000 { fx.push((p.file_name().unwrap().to_string_lossy().to_string(),b)); } } } }
    println!("fixtures={}",fx.len());
    let mut r=Rng(seed|1); let t=std::time::Instant::now(); let mut iters=0u64; let mut seen:BTreeMap<String,(u64,String)>=BTreeMap::new();
    let mut slow=0;
    while t.elapsed().as_secs()<secs {
        let (name,base)=&fx[r.r(fx.len())]; let m=mutate(&mut r,base); iters+=1;
        std::fs::write(format!("/work/tmp/last-{seed}.bin"),&m).unwrap();
        let deep=iters%8==0; let m2=m.clone(); let t0=std::time::Instant::now();
        let res=std::panic::catch_unwind(move || exercise(m2,deep));
        if t0.elapsed().as_secs()>=3 { slow+=1; if slow<5 { println!("SLOW {:?} from {name}",t0.elapsed()); std::fs::write(format!("/work/tmp/slow-{seed}-{slow}.bin"),&m).unwrap(); } }
        if res.is_err(){ let k=LAST.lock().unwrap().clone(); let e=seen.entry(k.clone()).or_insert((0,name.clone())); e.0+=1; if e.0==1 { let f=format!("/work/tmp/crash-{}.bin",seen.len()); std::fs::write(&f,&m).unwrap(); println!("NEW {k} from {name} -> {f}"); } }
    }
    println!("iters={iters}"); for (k,(c,n)) in &seen { println!("{c:6} {k} [{n}]"); }
}
