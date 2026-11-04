use std::io::{Cursor, Read, Write};
use zip::unstable::stream::{ZipStreamFileMetadata, ZipStreamReader, ZipStreamVisitor};
use zip::unstable::write::FileOptionsExt;
use zip::{write::SimpleFileOptions, AesMode, CompressionMethod, ZipArchive, ZipWriter};

fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
}
fn archive(options: SimpleFileOptions) -> Cursor<Vec<u8>> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", options).unwrap();
    w.write_all(b"hello").unwrap();
    w.finish().unwrap()
}

#[test]
fn finish_readable_zip64_preserves_sizes_and_contents() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let mut reopened = ZipArchive::new(a.clone().into_inner()).unwrap();
    assert_eq!(reopened.by_name("a").unwrap().size(), 5);
    let mut f = a.by_name("a").unwrap();
    let size = f.size();
    let mut s = Vec::new();
    let read_result = f.read_to_end(&mut s);
    eprintln!(
        "in-memory ZIP64 size={size}, read={read_result:?}, bytes={}",
        s.len()
    );
    assert_eq!(size, 5);
    read_result.unwrap();
    assert_eq!(s, b"hello");
}

#[test]
fn raw_copy_zipcrypto_preserves_encryption() {
    let mut a = ZipArchive::new(archive(stored().with_deprecated_encryption(b"password"))).unwrap();
    let mut expected = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut expected)
        .unwrap();
    assert_eq!(expected, b"hello");
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut copied = ZipArchive::new(w.finish().unwrap()).unwrap();
    let encrypted = copied.by_index_raw(0).unwrap().encrypted();
    let mut s = Vec::new();
    let read_result = copied
        .by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut s);
    eprintln!("copied ZipCrypto encrypted={encrypted}, read={read_result:?}");
    assert!(encrypted);
    read_result.unwrap();
    assert_eq!(s, b"hello");
}

#[test]
fn raw_copy_preserves_symlink_type() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_symlink("a", "target", stored()).unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(a.by_index(0).unwrap().is_symlink());
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(a.by_index(0).unwrap()).unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(a.by_index(0).unwrap().is_symlink());
}

#[test]
fn stream_writer_aes_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file(
        "a",
        stored().with_aes_encryption(AesMode::Aes128, "password"),
    )
    .unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut s = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut s)
        .unwrap();
    assert_eq!(s, b"hello");
}

#[test]
fn stream_writer_zip64_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("a", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert_eq!(a.by_index(0).unwrap().size(), 5);
}

#[derive(Default)]
struct Visitor {
    files: usize,
    metadata: usize,
}
impl ZipStreamVisitor for Visitor {
    fn visit_file<R: Read>(
        &mut self,
        f: &mut zip::read::ZipFile<'_, R>,
    ) -> zip::result::ZipResult<()> {
        let mut s = Vec::new();
        f.read_to_end(&mut s)?;
        assert_eq!(s, b"hello");
        self.files += 1;
        Ok(())
    }
    fn visit_additional_metadata(
        &mut self,
        _: &ZipStreamFileMetadata,
    ) -> zip::result::ZipResult<()> {
        self.metadata += 1;
        Ok(())
    }
}
#[test]
fn stream_reader_delivers_central_metadata() {
    let mut v = Visitor::default();
    ZipStreamReader::new(archive(stored()).into_inner().as_slice())
        .visit(&mut v)
        .unwrap();
    assert_eq!(v.files, 1);
    assert_eq!(v.metadata, 1);
}
#[test]
fn stream_reader_empty_archive() {
    let bytes = ZipWriter::new(Cursor::new(Vec::new()))
        .finish()
        .unwrap()
        .into_inner();
    ZipStreamReader::new(bytes.as_slice())
        .visit(&mut Visitor::default())
        .unwrap();
}
#[test]
fn streamed_file_data_start_is_documented_zero() {
    let bytes = archive(stored()).into_inner();
    let mut input = bytes.as_slice();
    let file = zip::read::read_zipfile_from_stream(&mut input)
        .unwrap()
        .unwrap();
    assert_eq!(file.data_start(), 0);
}
#[test]
fn long_filename_returns_error_without_panic() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.start_file("x".repeat(65536), stored())
    }));
    if result.is_err() {
        std::mem::forget(w);
    }
    assert!(result.is_ok(), "start_file panicked");
    assert!(result.unwrap().is_err());
}
#[test]
fn symlink_from_path_normalizes_target_as_documented() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_symlink_from_path("dir/link", "../target", stored())
        .unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut s = String::new();
    a.by_name("dir/link")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "target");
}
#[test]
fn deep_copy_after_stream_read_state_roundtrip() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.deep_copy_file("a", "b").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut s = String::new();
    a.by_name("b").unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn append_preserves_implode_compression_flags() {
    let bytes = std::fs::read("/work/crate/tests/data/legacy/implode.zip").unwrap();
    let mut original = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let mut expected = Vec::new();
    original
        .by_index(0)
        .unwrap()
        .read_to_end(&mut expected)
        .unwrap();
    let w = ZipWriter::new_append(Cursor::new(bytes)).unwrap();
    let mut appended = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut actual = Vec::new();
    appended
        .by_index(0)
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn raw_copy_preserves_implode_compression_flags() {
    let bytes = std::fs::read("/work/crate/tests/data/legacy/implode.zip").unwrap();
    let mut original = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(original.by_index_raw(0).unwrap()).unwrap();
    let mut copied = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut actual = Vec::new();
    copied
        .by_index(0)
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(
        actual,
        std::fs::read("/work/crate/tests/data/folder/first.txt").unwrap()
    );
}

#[test]
fn zipcrypto_stream_writer_roundtrip() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("a", stored().with_deprecated_encryption(b"password"))
        .unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut actual = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, b"hello");
}

#[test]
fn unicode_comment_checksum_uses_original_bytes() {
    let mut a =
        ZipArchive::new(std::fs::File::open("/work/tmp/fixtures/ascii-comment.zip").unwrap())
            .unwrap();
    assert_eq!(a.by_index(0).unwrap().comment(), "comment-é");
    let mut a =
        ZipArchive::new(std::fs::File::open("/work/tmp/fixtures/cp437-comment.zip").unwrap())
            .unwrap();
    assert_eq!(a.by_index(0).unwrap().comment(), "comment-é");
}

#[test]
fn append_preserves_zip64_offset() {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/work/tmp/fixtures/sparse-offset.zip")
        .unwrap();
    let mut a = ZipArchive::new(file).unwrap();
    assert_eq!(a.by_index(0).unwrap().header_start(), 0x100000000);
    let mut s = String::new();
    a.by_index(0).unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello");
    let file = ZipWriter::new_append(a.into_inner())
        .unwrap()
        .finish()
        .unwrap();
    let mut a = ZipArchive::new(file).unwrap();
    let mut s = String::new();
    a.by_index(0).unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn zip64_comment_survives_reopening_small_archive() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.set_zip64_comment(Some("zip64 comment"));
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(a.by_index(0).unwrap().size(), 5);
    assert_eq!(a.zip64_comment(), Some(b"zip64 comment".as_slice()));
}

#[test]
fn extraction_preserves_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_directory("sub", stored().unix_permissions(0o500))
        .unwrap();
    w.start_file("sub/a", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let output = "/work/tmp/permissions-extract";
    std::fs::create_dir_all(output).unwrap();
    a.extract(output).unwrap();
    assert_eq!(
        std::fs::metadata(format!("{output}/sub"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o500
    );
}

#[test]
fn supported_writing_methods_roundtrip_small_data() {
    let mut failures = Vec::new();
    let methods = [
        CompressionMethod::Stored,
        CompressionMethod::Deflated,
        CompressionMethod::Bzip2,
        CompressionMethod::Zstd,
        CompressionMethod::Xz,
        CompressionMethod::Ppmd,
    ];
    for method in methods {
        for size in [0, 1, 31, 8193] {
            let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let mut w = ZipWriter::new(Cursor::new(Vec::new()));
            w.start_file("é/文.txt", stored().compression_method(method))
                .unwrap();
            for chunk in data.chunks(7) {
                w.write_all(chunk).unwrap();
            }
            let bytes = w.finish().unwrap();
            if method == CompressionMethod::Xz {
                std::fs::write(format!("/work/tmp/fixtures/xz-{size}.zip"), bytes.get_ref())
                    .unwrap();
            }
            let mut a = ZipArchive::new(bytes).unwrap();
            let mut s = Vec::new();
            if let Err(e) = a.by_name("é/文.txt").unwrap().read_to_end(&mut s) {
                failures.push(format!("{method:?}, {size}: {e}"));
            }
            assert_eq!(s, data, "{method:?}, {size}");
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn append_preserves_cp437_file_comment_text() {
    let bytes = std::fs::read("/work/tmp/fixtures/cp437-short-comment.zip").unwrap();
    let mut original = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    assert_eq!(original.by_index(0).unwrap().comment(), "é");
    let w = ZipWriter::new_append(Cursor::new(bytes)).unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(a.by_index(0).unwrap().comment(), "é");
}

#[test]
fn central_directory_io_error_is_preserved() {
    use std::io::{self, Seek, SeekFrom};
    struct FallibleReader {
        cursor: Cursor<Vec<u8>>,
        central: u64,
        failed: bool,
    }
    impl Read for FallibleReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.cursor.position() == self.central && buf.len() == 46 && !self.failed {
                self.failed = true;
                return Err(io::Error::other("review transient I/O error"));
            }
            self.cursor.read(buf)
        }
    }
    impl Seek for FallibleReader {
        fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
            self.cursor.seek(from)
        }
    }
    let cursor = archive(stored());
    let central = ZipArchive::new(cursor.clone())
        .unwrap()
        .central_directory_start();
    let result = ZipArchive::new(FallibleReader {
        cursor,
        central,
        failed: false,
    });
    if let Err(e) = &result {
        eprintln!("reported error: {e:?}");
    }
    assert!(
        matches!(result, Err(zip::result::ZipError::Io(ref e)) if e.to_string() == "review transient I/O error"),
        "I/O error was discarded"
    );
}

#[test]
fn finish_readable_aes_can_decrypt_contents() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "a",
        stored().with_aes_encryption(AesMode::Aes128, "password"),
    )
    .unwrap();
    w.write_all(b"hello").unwrap();
    let mut a = w.finish_into_readable().unwrap();
    let mut reopened = ZipArchive::new(a.clone().into_inner()).unwrap();
    let mut expected = Vec::new();
    reopened
        .by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut expected)
        .unwrap();
    assert_eq!(expected, b"hello");
    let mut actual = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn raw_copy_aes_from_decrypted_file_preserves_raw_contents() {
    let mut src = ZipArchive::new(archive(
        stored().with_aes_encryption(AesMode::Aes128, "password"),
    ))
    .unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(src.by_index_decrypt(0, b"password").unwrap())
        .unwrap();
    let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut actual = Vec::new();
    a.by_index_decrypt(0, b"password")
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, b"hello");
}
