use std::io::{Cursor, Read, Write};
use zip::{write::SimpleFileOptions, CompressionMethod as M, ZipArchive, ZipWriter};
fn main() {
    for (m, n) in [(M::Xz, 0usize), (M::Xz, 1), (M::Deflated, 0), (M::Bzip2, 0), (M::Zstd, 0), (M::Ppmd, 0)] {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", SimpleFileOptions::default().compression_method(m)).unwrap();
        w.write_all(&vec![7u8; n]).unwrap();
        let v = w.finish().unwrap().into_inner();
        let mut a = ZipArchive::new(Cursor::new(v)).unwrap();
        let mut raw = vec![]; a.by_index_raw(0).unwrap().read_to_end(&mut raw).unwrap();
        let mut o = vec![]; let r = a.by_index(0).unwrap().read_to_end(&mut o).map_err(|e| e.to_string());
        println!("{m:?} n={n}: read={r:?} raw_hex={}", raw.iter().map(|b| format!("{b:02x}")).collect::<String>());
    }
}
