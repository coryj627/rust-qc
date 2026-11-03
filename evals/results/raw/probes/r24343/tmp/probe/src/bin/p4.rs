use probe::*;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::SimpleFileOptions};
fn stored() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }
fn snapshot<R: Read+Seek>(a:&mut ZipArchive<R>)->Vec<(String,Result<(u32,usize),String>,String,bool,Option<u32>)>{
    let mut out=vec![]; for i in 0..a.len(){ let enc=a.by_index_raw(i).map(|f| f.encrypted()).unwrap_or(false);
        let r = if enc { a.by_index_decrypt(i,b"helloworld").map(|mut f|{ let mut v=vec![]; f.read_to_end(&mut v).map(|_| (crc(&v),v.len())).map_err(|e|e.to_string()) }).unwrap_or_else(|e| Err(e.to_string())) } else { a.by_index(i).map(|mut f|{ let mut v=vec![]; f.read_to_end(&mut v).map(|_| (crc(&v),v.len())).map_err(|e|e.to_string()) }).unwrap_or_else(|e| Err(e.to_string())) };
        let f=a.by_index_raw(i); let (n,c,s,m)=match f { Ok(f)=>(f.name().to_string(),f.comment().to_string(),f.is_symlink(),f.unix_mode()), Err(_)=>("?".into(),String::new(),false,None) };
        out.push((n,r,c,s,m)); } out }
fn cmp(tag:&str, fx:&str, a:&[(String,Result<(u32,usize),String>,String,bool,Option<u32>)], b:Result<Vec<(String,Result<(u32,usize),String>,String,bool,Option<u32>)>,String>){
    match b { Err(e)=>println!("  {fx} [{tag}] REOPEN FAILED: {e}"), Ok(b)=>{ if a.len()!=b.len(){ println!("  {fx} [{tag}] count {} -> {}",a.len(),b.len()); }
        let mut d=0; for (x,y) in a.iter().zip(b.iter()){ if x!=y { d+=1; if d<=2 { println!("  {fx} [{tag}] diff {:?}\n      -> {:?}", (&x.0,&x.1,&x.2,x.3,x.4.map(|m|format!("{m:o}"))), (&y.0,&y.1,&y.2,y.3,y.4.map(|m|format!("{m:o}")))); } } } if d>2 { println!("  {fx} [{tag}] +{} more diffs",d-2); } } }
}
struct Fail{ inner:Cursor<Vec<u8>>, left:usize, fail_seek:bool }
impl Write for Fail{ fn write(&mut self,b:&[u8])->io::Result<usize>{ if self.left==0 {return Err(io::Error::other("disk full"));} let n=b.len().min(self.left); self.left-=n; self.inner.write(&b[..n]) } fn flush(&mut self)->io::Result<()>{Ok(())} }
impl Seek for Fail{ fn seek(&mut self,p:SeekFrom)->io::Result<u64>{ if self.fail_seek&&self.left==0 {return Err(io::Error::other("seek fail"));} self.inner.seek(p) } }
impl Read for Fail{ fn read(&mut self,b:&mut [u8])->io::Result<usize>{ self.inner.read(b) } }
struct RFail{ inner:Cursor<Vec<u8>>, bad:std::ops::Range<u64> }
impl Read for RFail{ fn read(&mut self,b:&mut [u8])->io::Result<usize>{ let p=self.inner.position(); let e=p+b.len() as u64; if p<self.bad.end && e>self.bad.start { return Err(io::Error::other("EIO disk failure")); } self.inner.read(b) } }
impl Seek for RFail{ fn seek(&mut self,p:SeekFrom)->io::Result<u64>{ self.inner.seek(p) } }
fn main(){
    std::panic::set_hook(Box::new(|i|{ eprintln!("PANIC at {:?}", i.location().map(|l| format!("{}:{}",l.file(),l.line()))); }));
    let mut fx=vec![]; for dir in ["/work/crate/tests/data","/work/crate/tests/data/legacy"]{ for e in std::fs::read_dir(dir).unwrap(){ let p=e.unwrap().path(); if p.extension().is_some_and(|x|x=="zip"||x=="ja") { let b=std::fs::read(&p).unwrap(); if b.len()<3_000_000 { fx.push((p.file_name().unwrap().to_string_lossy().to_string(),b)); } } } }
    for (n,b) in &fx {
        let Ok(mut a)=ZipArchive::new(Cursor::new(b.clone())) else { continue };
        let base=snapshot(&mut a);
        let nm=n.clone(); let bb=b.clone(); let base2=base.clone();
        let _=std::panic::catch_unwind(move || {
            // new_append + finish
            let r=ZipWriter::new_append(Cursor::new(bb.clone())).map_err(|e|e.to_string()).and_then(|mut w|{ w.start_file("__new",stored()).map_err(|e|e.to_string())?; w.write_all(b"n").unwrap(); w.finish().map_err(|e|e.to_string()) });
            let s=r.and_then(|c| { let mut a=ZipArchive::new(Cursor::new(c.into_inner())).map_err(|e|e.to_string())?; let i=a.index_for_name("__new"); let mut s=snapshot(&mut a); if let Some(i)=i { s.remove(i); } Ok(s) });
            cmp("new_append",&nm,&base2,s);
            // merge
            let src=ZipArchive::new(Cursor::new(bb.clone())).unwrap(); let mut w=ZipWriter::new(Cursor::new(vec![0u8;0]));
            let s=w.merge_archive(src).map_err(|e|e.to_string()).and_then(|_| w.finish().map_err(|e|e.to_string())).and_then(|c| { let mut a=ZipArchive::new(Cursor::new(c.into_inner())).map_err(|e|e.to_string())?; Ok(snapshot(&mut a)) });
            cmp("merge",&nm,&base2,s);
            // raw copy all
            let mut src=ZipArchive::new(Cursor::new(bb.clone())).unwrap(); let mut w=ZipWriter::new(Cursor::new(vec![]));
            let mut er=None; for i in 0..src.len(){ if let Err(e)=src.by_index_raw(i).and_then(|f| w.raw_copy_file(f)) { er=Some(e.to_string()); break; } }
            let s=if let Some(e)=er { Err(e) } else { w.finish().map_err(|e|e.to_string()).and_then(|c| { let mut a=ZipArchive::new(Cursor::new(c.into_inner())).map_err(|e|e.to_string())?; Ok(snapshot(&mut a)) }) };
            cmp("raw_copy",&nm,&base2,s);
            // deep/shallow copy first
            if let Ok(mut w)=ZipWriter::new_append(Cursor::new(bb.clone())) { let first=ZipArchive::new(Cursor::new(bb.clone())).unwrap().file_names().next().map(String::from);
                if let Some(fnm)=first { let a=w.deep_copy_file(&fnm,"__deep").map_err(|e|e.to_string()); let b=w.shallow_copy_file(&fnm,"__shallow").map_err(|e|e.to_string());
                    let o=w.finish().map_err(|e|e.to_string()).and_then(|c| ZipArchive::new(Cursor::new(c.into_inner())).map(|a| a.len()).map_err(|e|e.to_string()));
                    if a.is_err()||b.is_err()||o.is_err() { println!("  {nm} [copy] deep={a:?} shallow={b:?} reopen={o:?}"); } } }
        });
    }
    println!("--- specific: unicode path + new_append");
    guard("new_append on chinese.zip", || {
        let b=std::fs::read("/work/crate/tests/data/chinese.zip").unwrap();
        let w=ZipWriter::new_append(Cursor::new(b)).unwrap();
        let o=w.finish().unwrap().into_inner();
        format!("{:?}", ZipArchive::new(Cursor::new(o)).map(|a| a.file_names().map(String::from).collect::<Vec<_>>()).map_err(|e|e.to_string()))
    });
    guard("deep/shallow copy entry with unicode path", || {
        let raw=b"caf\x82.txt"; let mut body=vec![1u8]; body.extend(crc(raw).to_le_bytes()); body.extend("caf\u{e9}.txt".as_bytes());
        let mut e=E::new("x",b"x"); e.name=raw.to_vec(); e.extra_central=field(0x7075,&body); let z=build(&[e]);
        let names:Vec<String>=ZipArchive::new(Cursor::new(z.clone())).unwrap().file_names().map(String::from).collect();
        let w=ZipWriter::new_append(Cursor::new(z.clone())).unwrap(); let o=w.finish().unwrap().into_inner();
        let r0=ZipArchive::new(Cursor::new(o)).map(|a| a.len()).map_err(|e|e.to_string());
        format!("names={names:?} after plain new_append reopen={r0:?}")
    });
    if std::env::var("FW").is_ok() { println!("--- failing writer");
    let mut panics=0;
    for mode in [false,true] { for n in 0..3000usize {
        let r=std::panic::catch_unwind(move || {
            let mut w=ZipWriter::new(Fail{inner:Cursor::new(vec![]),left:n,fail_seek:mode});
            let _=(||->zip::result::ZipResult<()>{
                w.start_file("a",SimpleFileOptions::default())?; w.write_all(&[1u8;500])?;
                w.start_file("b",SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes128,"p"))?; w.write_all(&[2u8;50])?;
                w.add_directory("d",stored())?; w.add_symlink("l","t",stored())?;
                w.start_file("big",stored().large_file(true))?; w.write_all(&[3u8;10])?;
                w.deep_copy_file("a","a2")?; w.shallow_copy_file("a","a3")?;
                w.start_file("c",SimpleFileOptions::default())?; w.write_all(b"x")?; w.abort_file()?;
                w.add_directory("e",stored())?;
                Ok(())
            })();
            let _=w.add_directory("post",stored()); let _=w.add_symlink("post2","t",stored()); let _=w.shallow_copy_file("a","a9");
            let _=w.finish();
        });
        if r.is_err() { panics+=1; if panics<5 { println!("panic at budget n={n} mode={mode}"); } }
    } }
    println!("panics={panics}"); }
    println!("--- failing reader in central directory");
    guard("EIO while reading central dir", || {
        let z=build(&[E::new("a",b"x"),E::new("b",b"y")]);
        let cd=z.windows(4).position(|w|w==b"PK\x01\x02").unwrap() as u64;
        // Fail only for reads overlapping the second central entry, not the initial EOCD/first-CDFH scan (done with 2KiB window) -> use large junk before.
        let _=(z,cd);
        let es:Vec<E>=(0..200).map(|i| E::new(&format!("{i:03}"),b"x")).collect(); let z=build(&es); let cd=z.windows(4).position(|w|w==b"PK\x01\x02").unwrap() as u64; let eo=z.len() as u64-22;
        let mid=cd+((eo-cd)/2); let bad=mid..(mid+1);
        let r=ZipArchive::new(RFail{inner:Cursor::new(z.clone()),bad:bad.clone()}).map(|a| a.len()).map_err(|e|format!("{e:?}"));
        let r2=std::fs::File::open("/nonexistent").map(|_| ()).map_err(|e|e.kind());
        format!("cd@{cd} eocd@{eo} bad={bad:?} -> {r:?} {r2:?}")
    });
}
