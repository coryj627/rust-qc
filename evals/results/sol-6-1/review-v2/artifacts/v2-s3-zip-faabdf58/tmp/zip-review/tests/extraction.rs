#![cfg(unix)]
use std::io::{Cursor, Write};
use std::os::unix::fs::PermissionsExt;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};
fn destination(label: &str) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/work/tmp/zip-review/fixtures/{label}-{stamp}")
}
fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
#[test]
fn extraction_follows_existing_relative_link_within_destination() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_directory("real/", stored()).unwrap();
    w.add_symlink("link", "real", stored()).unwrap();
    w.start_file("link/file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let dest = destination("extract-relative-link");
    a.extract(&dest).unwrap();
    assert_eq!(
        std::fs::read(format!("{dest}/real/file")).unwrap(),
        b"hello"
    );
}
#[test]
fn extraction_preserves_explicit_directory_permissions() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_directory("dir/", stored().unix_permissions(0o555))
        .unwrap();
    w.start_file("dir/file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(
        a.by_name("dir/").unwrap().unix_mode().unwrap() & 0o777,
        0o555
    );
    let dest = destination("extract-permissions");
    a.extract(&dest).unwrap();
    assert_eq!(
        std::fs::metadata(format!("{dest}/dir"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o555
    );
}
#[test]
fn extraction_overwrites_an_existing_symlink_entry() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.add_symlink("link", "file", stored()).unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let dest = destination("extract-twice-link");
    a.extract(&dest).unwrap();
    a.extract(&dest).unwrap();
}
