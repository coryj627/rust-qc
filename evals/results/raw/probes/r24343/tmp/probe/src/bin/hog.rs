use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use zip::{ZipArchive, ZipWriter};
fn main(){
    let b=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mut c=Cursor::new(b.clone());
    match ZipArchive::new(&mut c){ Ok(mut a)=>{ eprintln!("open ok n={} offset={} cdstart={}",a.len(),a.offset(),a.central_directory_start());
        for i in 0..a.len().min(12){ if let Ok(f)=a.by_index_raw(i){ eprintln!("{i} {:?} m={:?} sz={} cs={} hs={} ds={}",f.name().chars().take(20).collect::<String>(),f.compression(),f.size(),f.compressed_size(),f.header_start(),f.data_start()); } }
        eprintln!("overlap {:?}", a.has_overlapping_files().map_err(|e|e.to_string())); }
      Err(e)=>eprintln!("open err {e}") }
    match ZipWriter::new_append(Cursor::new(b.clone())){ Ok(mut w)=>{ eprintln!("append pos: {:?}", ()); let r=w.start_file("appended",zip::write::SimpleFileOptions::default()).map_err(|e|e.to_string()); eprintln!("start {r:?}"); let _=w.write_all(b"x"); let o=w.finish().map(|c| c.into_inner().len()).map_err(|e|e.to_string()); eprintln!("finish {o:?}"); }
      Err(e)=>eprintln!("append err {e}") }
    let _=(SeekFrom::Start(0),); let mut _x=Cursor::new(vec![0u8]); let _=_x.seek(SeekFrom::Start(0)); let mut s=vec![]; let _=_x.read_to_end(&mut s);
}
