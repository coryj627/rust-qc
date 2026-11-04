#[cfg(all(feature = "bzip2", not(feature = "_deflate-any")))]
#[test]
fn bzip2_only_default_is_documented_compression() {
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, ZipArchive, CompressionMethod};
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", zip::write::SimpleFileOptions::default()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut archive = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(archive.by_name("file").unwrap().compression(), CompressionMethod::Bzip2);
}
