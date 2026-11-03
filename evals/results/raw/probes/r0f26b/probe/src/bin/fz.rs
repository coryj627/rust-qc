use probe::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashSet;
use std::io::{Cursor, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
use zip::read::read_zipfile_from_stream;

struct A;
thread_local!{ static GUARD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
unsafe impl GlobalAlloc for A {
    unsafe fn alloc(&self,l:Layout)->*mut u8{ check(l.size()); System.alloc(l) }
    unsafe fn dealloc(&self,p:*mut u8,l:Layout){ System.dealloc(p,l) }
    unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8{ check(l.size()); System.alloc_zeroed(l) }
    unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{ check(n); System.realloc(p,l,n) }
}
fn check(s:usize){ if s>(1usize<<36) && !GUARD.with(|g|g.replace(true)) { GUARD.with(|g|g.set(false)); panic!("BIGALLOC {s}"); } }
#[global_allocator] static G:A=A;

struct Rng(u64);
impl Rng{fn n(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x} fn r(&mut self,m:usize)->usize{(self.n()%(m.max(1) as u64)) as usize}}
static SEEN:Mutex<Option<HashSet<String>>>=Mutex::new(None);
static STAGE:Mutex<&'static str>=Mutex::new("");
fn stage(s:&'static str){ if let Ok(mut g)=STAGE.try_lock(){*g=s;} }

fn mutate(r:&mut Rng,d:&mut Vec<u8>){
    let n=1+r.r(6);
    for _ in 0..n {
        if d.is_empty(){return;}
        let tail=r.r(2)==0;
        let i= if tail { let w=d.len().min(600); d.len()-1-r.r(w) } else { r.r(d.len()) };
        match r.r(7){
            0=>d[i]=r.n() as u8,
            1=>d[i]^=1<<r.r(8),
            2=>{for k in 0..4{ if i+k<d.len(){d[i+k]=0xff;}}},
            3=>{for k in 0..2{ if i+k<d.len(){d[i+k]=0;}}},
            4=>{for k in 0..8{ if i+k<d.len(){d[i+k]=if k==7{0x7f}else{0xff};}}},
            5=>{ let a=r.r(d.len()); let b=r.r(d.len()); let l=r.r(64); for k in 0..l { if a+k<d.len()&&b+k<d.len(){d[a+k]=d[b+k];}}},
            _=>{ if d.len()>64 { let l=r.r(32); let e=(i+l).min(d.len()); d.drain(i..e); } }
        }
    }
}
fn drain<R:Read>(f:&mut R){ let mut b=[0u8;16384]; let mut t=0usize; while t<(1<<22){ match f.read(&mut b){Ok(0)|Err(_)=>break,Ok(n)=>t+=n}} }
fn exercise(d:&[u8],it:u64,pid:u32){
    stage("new");
    if let Ok(mut a)=ZipArchive::new(Cursor::new(d.to_vec())){
        stage("meta");
        let _=a.decompressed_size(); let _=a.root_dir(zip::read::root_dir_common_filter); let _=a.has_overlapping_files();
        let n=a.len().min(32);
        for i in 0..n {
            stage("by_index_raw");
            if let Ok(f)=a.by_index_raw(i){ let _=(f.name().to_string(),f.mangled_name(),f.enclosed_name(),f.is_dir(),f.is_symlink(),f.unix_mode(),f.comment().len(),f.options(),f.version_made_by(),f.last_modified(),f.data_start()); for _ in f.extra_data_fields(){} }
            stage("by_index"); if let Ok(mut f)=a.by_index(i){ stage("read"); drain(&mut f); }
            stage("decrypt"); if let Ok(mut f)=a.by_index_decrypt(i,b"password"){ stage("readdec"); drain(&mut f); }
            stage("aesinfo"); let _=a.get_aes_verification_key_and_salt(i);
            stage("seek"); if let Ok(mut f)=a.by_index_seek(i){ drain(&mut f); }
        }
        stage("rawcopy");
        { let mut w=ZipWriter::new(Cursor::new(vec![])); for i in 0..n { if let Ok(f)=a.by_index_raw(i){ let _=w.raw_copy_file(f); if i>3 {break;} } } let _=w.finish(); }
        stage("merge");
        if d.len()<1<<20 { let mut w=ZipWriter::new(Cursor::new(vec![])); let _=w.merge_archive(a.clone()); let _=w.finish(); }
        if it%200==0 { stage("extract"); let p=format!("/work/tmp/fz/{pid}/{it}"); let _=std::fs::create_dir_all(&p); let _=a.extract(&p); }
    }
    stage("stream");
    let mut c=Cursor::new(d); let mut k=0; while k<64 { match read_zipfile_from_stream(&mut c){ Ok(Some(mut f))=>{ let _=f.name().len(); drain(&mut f);} _=>break } k+=1; }
    stage("append");
    let okapp=ZipArchive::new(Cursor::new(d.to_vec())).map(|a|a.central_directory_start()<=d.len() as u64 && a.len()<5000).unwrap_or(false);
    if okapp { if let Ok(mut w)=ZipWriter::new_append(Cursor::new(d.to_vec())){ if w.start_file("zzz_new",SimpleFileOptions::default()).is_ok(){ let _=w.write_all(b"n"); } let _=w.deep_copy_file("zzz_new","zzz2"); let _=w.finish(); } }
}
fn legacy_seed(r:&mut Rng)->Vec<u8>{
    let m=[1u16,2,3,4,5,6,14,98,95,9,12,93][r.r(12)];
    let l=20+r.r(1500); let mut d=vec![0u8;l];
    if r.r(2)==0 { for b in d.iter_mut(){*b=r.n() as u8;} } else { let c=r.n() as u8; for b in d.iter_mut(){*b= if r.r(8)==0 {r.n() as u8} else {c};} }
    if m==14 { d[0..9].copy_from_slice(&[0x10,0x02,5,0,0x5d,0,0,1,0]); }
    if m==98 { d[0]=0x03; d[1]=0x00; }
    let mut e=E::new("legacy",&d); e.method=m; e.flags=[0,2,4,6][r.r(4)]; e.uncomp=Some(r.r(20000) as u32);
    build(&[e])
}
fn main(){
    let seed:u64=std::env::args().nth(1).unwrap().parse().unwrap();
    let secs:u64=std::env::args().nth(2).unwrap().parse().unwrap();
    let pid=std::process::id();
    *SEEN.lock().unwrap()=Some(HashSet::new());
    std::panic::set_hook(Box::new(|i|{
        let loc=i.location().map(|l|format!("{}:{}",l.file(),l.line())).unwrap_or_default();
        let msg=i.payload().downcast_ref::<String>().cloned().or(i.payload().downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_default();
        let msg=msg.chars().take(100).collect::<String>();
        let st=STAGE.try_lock().map(|g|*g).unwrap_or("?");
        let key=format!("{loc} {}",msg.split(|c:char|c.is_ascii_digit()).next().unwrap_or(""));
        let mut printed=false;
        if let Ok(mut g)=SEEN.try_lock(){ if let Some(s)=g.as_mut(){ if s.insert(key){ printed=true; } } }
        if printed { let bt=std::backtrace::Backtrace::force_capture().to_string(); let cr:Vec<&str>=bt.lines().filter(|l|l.contains("/work/crate/")).take(4).map(|s|s.trim()).collect(); eprintln!("NEWPANIC stage={st} {loc} :: {msg} :: {cr:?}"); }
    }));
    let mut seeds:Vec<Vec<u8>>=vec![];
    for e in std::fs::read_dir("/work/crate/tests/data").unwrap(){ let p=e.unwrap().path(); if p.extension().is_some_and(|x|x=="zip"){ let b=std::fs::read(&p).unwrap(); if b.len()<300_000{seeds.push(b);} } }
    for e in std::fs::read_dir("/work/crate/tests/data/legacy").unwrap(){ let p=e.unwrap().path(); if p.extension().is_some_and(|x|x=="zip"){ let b=std::fs::read(&p).unwrap(); if b.len()<300_000{seeds.push(b);} } }
    let mut r=Rng(seed.wrapping_mul(0x9E3779B97F4A7C15)|1);
    let t=Instant::now(); let mut it=0u64;
    while t.elapsed()<Duration::from_secs(secs){
        it+=1;
        let mut d= if r.r(4)==0 { legacy_seed(&mut r) } else { seeds[r.r(seeds.len())].clone() };
        mutate(&mut r,&mut d);
        let _=std::io::stdout().flush();
        let _=catch_unwind(AssertUnwindSafe(||exercise(&d,it,pid)));
    }
    println!("seed {seed} done iters={it}");
}
