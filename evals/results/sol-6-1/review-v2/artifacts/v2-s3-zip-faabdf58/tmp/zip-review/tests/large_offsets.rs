use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};
fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
fn sparse(label: &str) -> File {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(format!("/work/tmp/zip-review/fixtures/{label}.zip"))
        .unwrap();
    f.seek(SeekFrom::Start((1_u64 << 32) + 100)).unwrap();
    f
}
fn expect_file(f: File, name: &str) {
    let mut a = ZipArchive::new(f).unwrap();
    let mut got = Vec::new();
    a.by_name(name).unwrap().read_to_end(&mut got).unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn writing_at_large_offset_control() {
    let mut w = ZipWriter::new(sparse("offset-control"));
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    expect_file(w.finish().unwrap(), "file");
}
#[test]
fn merge_at_large_offset_retains_offsets() {
    let mut src = ZipWriter::new(Cursor::new(Vec::new()));
    src.start_file("file", stored()).unwrap();
    src.write_all(b"hello").unwrap();
    let mut w = ZipWriter::new(sparse("offset-merge"));
    w.merge_archive(ZipArchive::new(src.finish().unwrap()).unwrap())
        .unwrap();
    expect_file(w.finish().unwrap(), "file");
}
#[test]
fn deepcopy_at_large_offset_retains_offsets() {
    let mut w = ZipWriter::new(sparse("offset-deepcopy"));
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.deep_copy_file("file", "copy").unwrap();
    expect_file(w.finish().unwrap(), "copy");
}
#[test]
fn finish_into_readable_preserves_header_compression_method_aes() {
    use zip::AesMode;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "file",
        stored().with_aes_encryption(AesMode::Aes128, "password"),
    )
    .unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let mut got = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut got)
        .unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn deepcopy_beyond_zip32_limit_has_distinct_local_header() {
    let mut w = ZipWriter::new(sparse("offset-deepcopy-distinct"));
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.deep_copy_file("file", "copy").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let original = a.by_name("file").unwrap().header_start();
    let copied = a.by_name("copy").unwrap().header_start();
    assert_ne!(copied, original, "deep copy points to original header");
}
