use probe::*;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod as M, ZipArchive, ZipWriter};
use zip::unstable::stream::ZipStreamReader;
fn stored() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(M::Stored) }
fn main() {
    let list = std::fs::read_dir("/work/crate/tests/data/legacy").unwrap();
    for f in list.flatten() {
        let p = f.path();
        if p.extension().map_or(true, |e| e != "zip") { continue; }
        let v = std::fs::read(&p).unwrap();
        let readall = |v: Vec<u8>| -> String { let mut a = match ZipArchive::new(Cursor::new(v)) { Ok(a) => a, Err(e) => return format!("open err {e}") }; let mut s = String::new(); for i in 0..a.len() { let mut o = vec![]; let r = a.by_index(i).map(|mut f| f.read_to_end(&mut o).map_err(|e| e.to_string())); if !matches!(r, Ok(Ok(_))) { s += &format!("[{i}:{:?}] ", r.map_err(|e| e.to_string())); } } if s.is_empty() { format!("ok({})", a.len()) } else { s.chars().take(200).collect() } };
        let orig = readall(v.clone());
        let ap = ZipWriter::new_append(Cursor::new(v.clone())).unwrap().finish().unwrap().into_inner();
        let after = readall(ap);
        let mut a = ZipArchive::new(Cursor::new(v.clone())).unwrap();
        let mut d = ZipWriter::new(Cursor::new(vec![]));
        for i in 0..a.len() { d.raw_copy_file(a.by_index_raw(i).unwrap()).unwrap(); }
        let raw = readall(d.finish().unwrap().into_inner());
        println!("{:?}: orig={orig} append={after} rawcopy={raw}", p.file_name().unwrap());
    }
    // prepended junk + append
    let mut w = ZipWriter::new(Cursor::new(vec![])); w.start_file("a", stored()).unwrap(); w.write_all(b"AAA").unwrap();
    let mut v = vec![b'#'; 500]; v.extend(w.finish().unwrap().into_inner());
    let mut w = ZipWriter::new_append(Cursor::new(v)).unwrap(); w.start_file("b", stored()).unwrap(); w.write_all(b"BBB").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut s = String::new(); a.by_name("a").unwrap().read_to_string(&mut s).unwrap(); a.by_name("b").unwrap().read_to_string(&mut s).unwrap();
    println!("prepended+append: {s:?} offset={}", a.offset());
    // unix attr with only DOS bits
    let mut x = e("f", b"x"); x.attr = 0x20; let t = tmp("attr");
    let r = ZipArchive::new(Cursor::new(build(&[x]))).unwrap().extract(&t).map_err(|e| e.to_string());
    println!("unix-made attr=0x20: extract={r:?} mode={:o}", std::fs::metadata(t.join("f")).unwrap().permissions().mode());
    // date 0 after append
    let mut x = e("f", b"x"); x.date = 0; x.time = 0; x.vmade = (19 << 8) | 30;
    let v = build(&[x]);
    let w = ZipWriter::new_append(Cursor::new(v)).unwrap(); let v2 = w.finish().unwrap().into_inner();
    let cd = v2.windows(4).rposition(|w| w == [0x50, 0x4b, 1, 2]).unwrap();
    println!("after append: made-by system byte={} (was 19) dos time/date={:02x?} (was 0)", v2[cd + 5], &v2[cd + 12..cd + 16]);
    // vacuous stream test
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.add_symlink("symlink/", "../dest-sibling/", SimpleFileOptions::default()).unwrap();
    writer.start_file("symlink/dest-file", SimpleFileOptions::default()).unwrap();
    let mut c = writer.finish().unwrap();
    let pos = c.position();
    let r_end = ZipStreamReader::new(c.clone()).extract(tmp("v1")).map_err(|e| e.to_string());
    c.seek(SeekFrom::Start(0)).unwrap();
    let t = tmp("v2");
    let r_start = ZipStreamReader::new(c).extract(&t).map_err(|e| e.to_string());
    println!("stream test: cursor pos after finish={pos}; extract as written in test={r_end:?}; after rewind={r_start:?} file_exists={}", t.join("symlink/dest-file").exists());
    // empty archive via stream
    let v = ZipWriter::new(Cursor::new(vec![])).finish().unwrap().into_inner();
    println!("empty zip len={} ZipArchive ok={} stream extract={:?}", v.len(), ZipArchive::new(Cursor::new(v.clone())).is_ok(), ZipStreamReader::new(Cursor::new(v)).extract(tmp("empty")).map_err(|e| e.to_string()));
    // SFX stub + nested stored zip at end
    let mut inner = ZipWriter::new(Cursor::new(vec![])); inner.start_file("inner-secret", stored()).unwrap(); inner.write_all(b"II").unwrap();
    let inner = inner.finish().unwrap().into_inner();
    let mut o = ZipWriter::new(Cursor::new(vec![])); o.start_file("inner.zip", stored()).unwrap(); o.write_all(&inner).unwrap();
    let outer = o.finish().unwrap().into_inner();
    let plain = ZipArchive::new(Cursor::new(outer.clone())).unwrap().file_names().map(String::from).collect::<Vec<_>>();
    let mut sfx = vec![0x7fu8; 4096]; sfx.extend(&outer);
    let r = ZipArchive::new(Cursor::new(sfx)).map(|a| (a.file_names().map(String::from).collect::<Vec<_>>(), a.offset())).map_err(|e| e.to_string());
    println!("outer names={plain:?}; with 4096-byte stub prepended => {r:?}");
}
