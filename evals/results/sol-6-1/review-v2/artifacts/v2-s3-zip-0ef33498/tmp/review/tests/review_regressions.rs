use std::io::{Cursor, Read, Write};
use zip::{ZipWriter, ZipArchive, CompressionMethod, AesMode};
use zip::write::SimpleFileOptions;
use zip::result::ZipResult;
use zip::unstable::stream::{ZipStreamReader, ZipStreamVisitor, ZipStreamFileMetadata};
use zip::read::ZipFile;
use zip::unstable::write::FileOptionsExt;

fn stored() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }

#[test]
fn readable_zip64_keeps_actual_size() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    assert_eq!(a.by_index(0).unwrap().size(), 5);
}

#[test]
fn readable_aes_roundtrip() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored().with_aes_encryption(AesMode::Aes128, "password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let mut actual = Vec::new();
    a.by_index_decrypt(0, b"password").unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}

#[derive(Default)]
struct Counts(usize, usize);
impl ZipStreamVisitor for Counts {
    fn visit_file<R: Read>(&mut self, _: &mut ZipFile<'_, R>) -> ZipResult<()> { self.0 += 1; Ok(()) }
    fn visit_additional_metadata(&mut self, _: &ZipStreamFileMetadata) -> ZipResult<()> { self.1 += 1; Ok(()) }
}
#[test]
fn stream_visits_all_metadata() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.start_file("b", stored()).unwrap();
    w.write_all(b"world").unwrap();
    let mut counts = Counts::default();
    let bytes = w.finish().unwrap().into_inner();
    ZipStreamReader::new(bytes.as_slice()).visit(&mut counts).unwrap();
    assert_eq!((counts.0, counts.1), (2, 2));
}

#[test]
fn raw_copy_preserves_zipcrypto() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored().with_deprecated_encryption(b"password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut dst = ZipWriter::new(Cursor::new(Vec::new()));
    dst.raw_copy_file(source.by_index_raw(0).unwrap()).unwrap();
    let mut copied = ZipArchive::new(dst.finish().unwrap()).unwrap();
    assert!(copied.by_index_raw(0).unwrap().encrypted());
    let mut actual = Vec::new();
    copied.by_index_decrypt(0, b"password").unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}

#[test]
fn raw_copy_preserves_symlink() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_symlink("link", "target", stored()).unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(source.by_index(0).unwrap().is_symlink());
    let mut dst = ZipWriter::new(Cursor::new(Vec::new()));
    dst.raw_copy_file(source.by_index(0).unwrap()).unwrap();
    let mut copied = ZipArchive::new(dst.finish().unwrap()).unwrap();
    assert!(copied.by_index(0).unwrap().is_symlink());
}

#[test]
fn streaming_aes_can_finish() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("a", stored().with_aes_encryption(AesMode::Aes128, "password")).unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut actual = Vec::new();
    a.by_index_decrypt(0, b"password").unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}

#[test]
fn streaming_zip64_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("a", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut actual = Vec::new();
    a.by_index(0).unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}

#[test]
fn streamed_file_data_start_is_documented_zero() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut bytes = w.finish().unwrap();
    bytes.set_position(0);
    let file = zip::read::read_zipfile_from_stream(&mut bytes).unwrap().unwrap();
    assert_eq!(file.data_start(), 0);
}

#[test]
fn streaming_zipcrypto_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("a", stored().last_modified_time(zip::DateTime::default()).with_deprecated_encryption(b"password")).unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut actual = Vec::new();
    a.by_index_decrypt(0, b"password").unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}

#[test]
fn too_long_name_returns_error() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.start_file("a".repeat(65536), stored())
    }));
    // Avoid invoking Drop on the writer left partially initialized by the panic.
    std::mem::forget(w);
    assert!(matches!(result, Ok(Err(_))), "expected ZipError for a filename exceeding the ZIP field, got {result:?}");
}

#[test]
fn ordinary_finish_controls() {
    for options in [stored(), stored().large_file(true), stored().with_aes_encryption(AesMode::Aes128, "password"), stored().with_deprecated_encryption(b"password")] {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        w.start_file("a", options).unwrap();
        w.write_all(b"hello").unwrap();
        let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
        let mut actual = Vec::new();
        let mut file = a.by_index_decrypt(0, b"password").unwrap();
        assert_eq!(file.size(), 5);
        file.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, b"hello");
    }
}

#[test]
fn streaming_plain_roundtrip_control() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
    let mut actual = Vec::new();
    a.by_index(0).unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}

#[test]
fn readable_zip64_reads_only_file_contents() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let mut actual = Vec::new();
    let result = a.by_index(0).unwrap().read_to_end(&mut actual);
    assert!(result.is_ok(), "read_to_end={result:?}, bytes read={}", actual.len());
    assert_eq!(actual, b"hello");
}

#[test]
fn raw_copy_zipcrypto_stays_readable() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored().with_deprecated_encryption(b"password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut dst = ZipWriter::new(Cursor::new(Vec::new()));
    dst.raw_copy_file(source.by_index_raw(0).unwrap()).unwrap();
    let mut copied = ZipArchive::new(dst.finish().unwrap()).unwrap();
    let mut actual = Vec::new();
    let result = copied.by_index_decrypt(0, b"password").unwrap().read_to_end(&mut actual);
    assert!(result.is_ok(), "read_to_end={result:?}, bytes read={}", actual.len());
    assert_eq!(actual, b"hello");
}

#[test]
fn raw_copy_legacy_archives() {
    for fixture in ["shrink.zip", "reduce.zip", "implode.zip"] {
        let bytes = std::fs::read(format!("tests/data/legacy/{fixture}")).unwrap();
        let mut src = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut dst = ZipWriter::new(Cursor::new(Vec::new()));
        let mut expected = Vec::new();
        for i in 0..src.len() {
            let mut contents = Vec::new();
            src.by_index(i).unwrap().read_to_end(&mut contents).unwrap();
            expected.push(contents);
            dst.raw_copy_file(src.by_index_raw(i).unwrap()).unwrap();
        }
        let mut dst = ZipArchive::new(dst.finish().unwrap()).unwrap();
        for (i, expected) in expected.into_iter().enumerate() {
            let mut actual = Vec::new();
            let result = dst.by_index(i).unwrap().read_to_end(&mut actual);
            assert!(result.is_ok(), "{fixture} entry {i}: {result:?}");
            assert_eq!(actual, expected, "{fixture} entry {i}");
        }
    }
}

#[test]
fn zip64_comment_roundtrip() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.set_zip64_comment(Some("archive metadata"));
    w.start_file("a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(a.zip64_comment(), Some(b"archive metadata".as_slice()));
}

#[test]
fn maximum_name_length_control() {
    let name = "a".repeat(65535);
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(&name, stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(a.by_index(0).unwrap().name(), name);
}
