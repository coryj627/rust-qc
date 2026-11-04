use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use zip::result::ZipResult;
use zip::write::SimpleFileOptions;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};

fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
fn archive(large: bool) -> Cursor<Vec<u8>> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(large)).unwrap();
    w.write_all(b"hello").unwrap();
    w.finish().unwrap()
}
#[test]
fn finish_into_readable_preserves_zip64_sizes() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let f = a.by_index(0).unwrap();
    assert_eq!((f.size(), f.compressed_size()), (5, 5));
}
#[test]
fn deep_copy_preserves_zip64_sizes() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    w.deep_copy_file("file", "copy").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let f = a.by_name("copy").unwrap();
    assert_eq!((f.size(), f.compressed_size()), (5, 5));
}
#[test]
fn stream_visitor_gets_all_metadata() {
    use zip::unstable::stream::{ZipStreamFileMetadata, ZipStreamReader, ZipStreamVisitor};
    struct Counter(usize, usize);
    impl ZipStreamVisitor for Counter {
        fn visit_file<R: Read>(&mut self, _: &mut zip::read::ZipFile<'_, R>) -> ZipResult<()> {
            self.0 += 1;
            Ok(())
        }
        fn visit_additional_metadata(&mut self, _: &ZipStreamFileMetadata) -> ZipResult<()> {
            self.1 += 1;
            Ok(())
        }
    }
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    for name in ["one", "two"] {
        w.start_file(name, stored()).unwrap();
        w.write_all(b"hello").unwrap();
    }
    let mut c = Counter(0, 0);
    let mut input = w.finish().unwrap();
    input.set_position(0);
    ZipStreamReader::new(input).visit(&mut c).unwrap();
    assert_eq!((c.0, c.1), (2, 2));
}
#[test]
fn streaming_data_start_matches_documented_zero() {
    let mut cursor = archive(false);
    cursor.set_position(0);
    let f = zip::read::read_zipfile_from_stream(&mut cursor)
        .unwrap()
        .unwrap();
    assert_eq!(f.data_start(), 0);
}
#[test]
fn stream_aes_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file(
        "file",
        stored().with_aes_encryption(AesMode::Aes128, "password"),
    )
    .unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut got = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut got)
        .unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn raw_copy_zipcrypto_retains_encryption() {
    use zip::unstable::write::FileOptionsExt;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_deprecated_encryption(b"password"))
        .unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut copy = ZipWriter::new(Cursor::new(Vec::new()));
    copy.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut a = ZipArchive::new(copy.finish().unwrap()).unwrap();
    assert!(
        a.by_index_raw(0).unwrap().encrypted(),
        "raw copy lost encryption flag"
    );
    let mut got = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut got)
        .unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn raw_copy_decrypted_aes_entry() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "file",
        stored().with_aes_encryption(AesMode::Aes128, "password"),
    )
    .unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut copy = ZipWriter::new(Cursor::new(Vec::new()));
    copy.raw_copy_file(a.by_index_decrypt(0, b"password").unwrap())
        .unwrap();
    let mut a = ZipArchive::new(copy.finish().unwrap()).unwrap();
    let mut got = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut got)
        .unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn raw_copy_preserves_symlink_type() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_symlink("link", "file", stored()).unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(a.by_index(0).unwrap().is_symlink());
    let mut copy = ZipWriter::new(Cursor::new(Vec::new()));
    copy.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut a = ZipArchive::new(copy.finish().unwrap()).unwrap();
    assert!(a.by_index(0).unwrap().is_symlink());
}
#[test]
fn stored_seek_can_go_past_eof() {
    let mut a = ZipArchive::new(archive(false)).unwrap();
    let mut f = a.by_index_seek(0).unwrap();
    assert_eq!(f.seek(SeekFrom::Start(8)).unwrap(), 8);
    assert_eq!(f.stream_position().unwrap(), 8);
}
#[test]
fn too_long_filename_returns_error() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.start_file("a".repeat(65536), stored())
    }));
    let _ = w.abort_file();
    assert!(
        matches!(result, Ok(Err(_))),
        "expected error, got {result:?}"
    );
}
#[test]
fn zip64_comment_roundtrip() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.set_zip64_comment(Some("hello"));
    w.start_file("file", stored()).unwrap();
    let a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(a.zip64_comment(), Some(b"hello".as_slice()));
}
#[test]
fn finish_into_readable_zip64_reads_only_file_content() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let mut got = Vec::new();
    let result = a.by_index(0).unwrap().read_to_end(&mut got);
    assert!(
        result.is_ok(),
        "read failed: {result:?}; actual length: {}",
        got.len()
    );
    assert_eq!(got, b"hello");
}
#[test]
fn raw_copy_preserves_extra_fields() {
    use zip::write::FullFileOptions;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let mut options = FullFileOptions::default().compression_method(CompressionMethod::Stored);
    options
        .add_extra_data(0xbeef, vec![1, 2, 3].into_boxed_slice(), false)
        .unwrap();
    w.start_file("file", options).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let expected = a.by_index_raw(0).unwrap().extra_data().unwrap().to_vec();
    let mut copy = ZipWriter::new(Cursor::new(Vec::new()));
    copy.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut a = ZipArchive::new(copy.finish().unwrap()).unwrap();
    assert_eq!(a.by_index_raw(0).unwrap().extra_data().unwrap(), expected);
}
#[test]
fn stream_zipcrypto_roundtrip() {
    use zip::unstable::write::FileOptionsExt;
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("file", stored().with_deprecated_encryption(b"password"))
        .unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
    let mut got = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut got)
        .unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn unsupported_options_leave_writer_usable() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    assert!(w
        .start_file("bad", stored().compression_level(Some(10)))
        .is_err());
    w.start_file("good", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut got = Vec::new();
    a.by_index(0).unwrap().read_to_end(&mut got).unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn filename_limit_is_in_bytes_and_accepts_maximum() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a".repeat(65535), stored()).unwrap();
    let a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(a.name_for_index(0).unwrap().len(), 65535);
}
#[test]
fn stream_unencrypted_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
    let mut got = Vec::new();
    a.by_index(0).unwrap().read_to_end(&mut got).unwrap();
    assert_eq!(got, b"hello");
}
#[test]
fn copy_before_and_after_read_reports_errors_instead_of_damaged_output() {
    for read_first in [false, true] {
        let mut a = ZipArchive::new(archive(false)).unwrap();
        let mut f = a.by_index(0).unwrap();
        if read_first {
            let mut b = [0];
            f.read_exact(&mut b).unwrap();
        }
        let mut copy = ZipWriter::new(Cursor::new(Vec::new()));
        let result = copy.raw_copy_file(f);
        if result.is_err() {
            assert!(read_first);
            continue;
        }
        let mut a = ZipArchive::new(copy.finish().unwrap()).unwrap();
        let mut got = Vec::new();
        a.by_index(0).unwrap().read_to_end(&mut got).unwrap();
        assert_eq!(got, b"hello", "read_first={read_first}");
    }
}
