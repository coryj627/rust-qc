use std::io::{Cursor, Read};
use zip::ZipArchive;
fn main(){
    let p=std::env::args().nth(1).unwrap(); let b=std::fs::read(&p).unwrap();
    { let mut c=Cursor::new(b.clone()); for _ in 0..8 { match zip::read::read_zipfile_from_stream(&mut c){ Ok(Some(mut f))=>{ eprintln!("stream {} m={:?} sz={}",f.name(),f.compression(),f.size()); let mut v=vec![]; let _=(&mut f).take(1<<20).read_to_end(&mut v); } _=>break } } }
    let mut c=Cursor::new(b.clone());
    let Ok(mut a)=ZipArchive::new(&mut c) else { eprintln!("no open"); return };
    for i in 0..a.len().min(12) { match a.by_index_raw(i){ Ok(f)=>eprintln!("{i} {} m={:?} sz={} cs={} sym={}",f.name(),f.compression(),f.size(),f.compressed_size(),f.is_symlink()), Err(e)=>eprintln!("{i} raw err {e}") }
        eprintln!(" by_index"); if let Ok(mut f)=a.by_index(i){ let mut v=vec![]; let _=(&mut f).take(1<<20).read_to_end(&mut v); }
        eprintln!(" decrypt"); if let Ok(mut f)=a.by_index_decrypt(i,b"password"){ let mut v=vec![]; let _=(&mut f).take(1<<20).read_to_end(&mut v); }
        eprintln!(" done");
    }
    eprintln!("deep raw copy"); let mut w=zip::ZipWriter::new(Cursor::new(vec![]));
    for i in 0..a.len().min(12) { if let Ok(f)=a.by_index_raw(i){ if f.compressed_size()<(1<<20) { let _=w.raw_copy_file(f); } } }
    let _=w.finish();
    eprintln!("extract"); let d=std::env::temp_dir().join(format!("tr{}",std::process::id())); std::fs::create_dir_all(&d).unwrap(); let r=a.extract(&d); eprintln!("{r:?}");
    eprintln!("OK");
}
