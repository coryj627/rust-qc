use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};
fn main() {
    for lvl in [24i64, 10, 9, 5, 1] {
        let r = std::panic::catch_unwind(|| {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            let r = w.start_file("a", SimpleFileOptions::default().compression_level(Some(lvl))).map_err(|e| e.to_string());
            if r.is_ok() { w.write_all(b"hello").unwrap(); }
            let ok = w.finish().is_ok(); (r, ok)
        });
        println!("zopfli-only level {lvl}: {:?}", r.map_err(|_| "PANIC"));
    }
}
