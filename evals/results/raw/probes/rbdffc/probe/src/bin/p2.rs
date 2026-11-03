use probe::*;
use std::io::{Cursor, Read, Seek, SeekFrom};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod as M, ZipArchive, ZipWriter};
fn pathx(raw: &[u8], c: u32, u: &[u8]) -> Vec<u8> { let _ = raw; let mut x = vec![0x75, 0x70, 0, 0, 1]; x.extend_from_slice(&c.to_le_bytes()); x.extend_from_slice(u); let l = (x.len() - 4) as u16; x[2..4].copy_from_slice(&l.to_le_bytes()); x }
fn main() {
    // stale Unicode path field
    let mut x = e("renamed.txt", b"x"); x.cx = pathx(b"", crc(b"old.txt"), "old\u{e9}.txt".as_bytes());
    println!("stale path crc => {:?}", ZipArchive::new(Cursor::new(build(&[x]))).map(|a| a.file_names().map(String::from).collect::<Vec<_>>()).map_err(|e| e.to_string()));
    let mut x = e("a.txt", b"x"); x.cx = pathx(b"", crc(b"a.txt"), b"\xff\xfe");
    println!("invalid utf8 in path field => {:?}", ZipArchive::new(Cursor::new(build(&[x]))).map(|a| a.file_names().map(String::from).collect::<Vec<_>>()).map_err(|e| e.to_string()));
    // seek overflow
    let r = std::panic::catch_unwind(|| { let mut x = e("a", b"xx"); x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(1, u64::MAX);
        let mut a = ZipArchive::new(Cursor::new(build(&[x]))).unwrap(); let mut f = a.by_index_seek(0).unwrap(); f.seek(SeekFrom::End(0)).map_err(|e| e.to_string()) });
    println!("seek end with csize=u64::MAX => {:?}", r.map_err(|_| "PANIC"));
    let r = std::panic::catch_unwind(|| { let mut x = e("a", b"xx"); x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(1, u64::MAX);
        let mut a = ZipArchive::new(Cursor::new(build(&[x]))).unwrap(); a.has_overlapping_files().map_err(|e| e.to_string()) });
    println!("has_overlapping u64::MAX => {:?}", r.map_err(|_| "PANIC"));
    let r = std::panic::catch_unwind(|| { let a = e("a", b"aaaa"); let mut x = e("b", b"bbbb"); x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(1, u64::MAX); let mut z = ZipArchive::new(Cursor::new(build(&[x, a]))).unwrap(); z.has_overlapping_files().map_err(|e| e.to_string()) });
    println!("has_overlapping wrap with a real file after => {:?}", r.map_err(|_| "PANIC"));
    // symlink from path
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.add_symlink_from_path("a/b/link", "../c", SimpleFileOptions::default()).unwrap();
    w.add_symlink_from_path("abs", "/opt/tool", SimpleFileOptions::default()).unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    for n in ["a/b/link", "abs"] { let mut s = String::new(); a.by_name(n).unwrap().read_to_string(&mut s).unwrap(); println!("add_symlink_from_path {n} stored target={s:?}"); }
    // AES underflow in release vs debug + rootdir, implode
    let r = std::panic::catch_unwind(|| { let mut x = e("s", &[0u8; 5]); x.method = 99; x.flags = 1; x.cx = vec![0x01, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 0, 0]; let mut a = ZipArchive::new(Cursor::new(build(&[x]))).unwrap(); a.by_index_decrypt(0, b"pw").map(|_| ()).map_err(|e| e.to_string()) });
    println!("aes csize=5 => {:?}", r.map_err(|_| "PANIC"));
    let r = std::panic::catch_unwind(|| { let mut x = e("i", &[0xFFu8; 300]); x.method = 6; x.flags = 4; x.usize_ = 10; let mut a = ZipArchive::new(Cursor::new(build(&[x]))).unwrap(); let mut o = vec![]; let rr = a.by_index(0).unwrap().read_to_end(&mut o).map_err(|e| e.to_string()); rr });
    println!("implode 0xff => {:?}", r.map_err(|_| "PANIC"));
    let r = std::panic::catch_unwind(|| { let mut a = ZipArchive::new(Cursor::new(build(&[e("a/../b/x", b"1"), e("a/../b/y", b"2")]))).unwrap(); let t = tmp("rootd"); let r = a.extract_unwrapped_root_dir(&t, zip::read::root_dir_common_filter).map_err(|e| e.to_string()); (r, t.join("b/x").exists()) });
    println!("rootdir a/../b => {:?}", r.map_err(|_| "PANIC"));
    // LZMA with 4GiB dictionary
    let mut d = vec![0, 0, 5, 0, 0x5d, 0xff, 0xff, 0xff, 0xff]; d.extend_from_slice(&[0u8; 32]);
    let mut x = e("l", &d); x.method = 14; x.usize_ = 1 << 20;
    let mut a = ZipArchive::new(Cursor::new(build(&[x]))).unwrap();
    let mut o = vec![]; let t0 = std::time::Instant::now(); let r = a.by_index(0).unwrap().read_to_end(&mut o).map_err(|e| e.to_string());
    println!("lzma 4GiB dict: {r:?} len={} in {:?}; {}", o.len(), t0.elapsed(), std::fs::read_to_string("/proc/self/status").unwrap().lines().filter(|l| l.starts_with("VmHWM") || l.starts_with("VmPeak")).collect::<Vec<_>>().join(" "));
    let _ = M::Stored;
}
