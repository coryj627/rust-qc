use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};
#[test]
fn empty_xz_cross_check() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "empty",
            SimpleFileOptions::default().compression_method(CompressionMethod::Xz),
        )
        .unwrap();
    writer.write_all(b"").unwrap();
    let bytes = writer.finish().unwrap();
    let mut archive = ZipArchive::new(bytes).unwrap();
    let mut raw = Vec::new();
    archive
        .by_index_raw(0)
        .unwrap()
        .read_to_end(&mut raw)
        .unwrap();
    std::fs::write("/work/tmp/zip-review/fixtures/written-empty.xz", &raw).unwrap();
    let mut data = Vec::new();
    archive.by_index(0).unwrap().read_to_end(&mut data).unwrap();
    assert!(data.is_empty());
}
