use std::io::{Cursor, Read, Write};
use zip::{ZipArchive, ZipWriter, CompressionMethod};
use zip::write::SimpleFileOptions;

fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

#[test]
fn zip64_finish_into_readable() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut archive = w.finish_into_readable().unwrap();
    let mut file = archive.by_name("file").unwrap();
    assert_eq!(file.size(), 5);
    assert_eq!(file.compressed_size(), 5);
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn raw_copy_symlink() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_symlink("link", "target", stored()).unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(source.by_name("link").unwrap().is_symlink());
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(source.by_name("link").unwrap()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(result.by_name("link").unwrap().is_symlink());
}

#[test]
fn raw_copy_zipcrypto() {
    use zip::unstable::write::FileOptionsExt;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_deprecated_encryption(b"password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(source.by_index_raw(0).unwrap()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut file = result.by_name_decrypt("file", b"password").unwrap();
    assert!(file.encrypted());
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn stream_metadata_count() {
    use zip::unstable::stream::{ZipStreamReader, ZipStreamVisitor, ZipStreamFileMetadata};
    struct Visitor(usize, usize);
    impl ZipStreamVisitor for Visitor {
        fn visit_file<R: Read>(&mut self, _: &mut zip::read::ZipFile<'_, R>) -> zip::result::ZipResult<()> {
            self.0 += 1; Ok(())
        }
        fn visit_additional_metadata(&mut self, _: &ZipStreamFileMetadata) -> zip::result::ZipResult<()> {
            self.1 += 1; Ok(())
        }
    }
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("one", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    w.start_file("two", stored()).unwrap();
    w.write_all(b"world").unwrap();
    let mut visitor = Visitor(0, 0);
    let mut buffer = w.finish().unwrap();
    buffer.set_position(0);
    ZipStreamReader::new(buffer).visit(&mut visitor).unwrap();
    assert_eq!((visitor.0, visitor.1), (2, 2));
}

#[test]
fn stream_data_start() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut buffer = w.finish().unwrap();
    buffer.set_position(0);
    let file = zip::read::read_zipfile_from_stream(&mut buffer).unwrap().unwrap();
    assert_eq!(file.data_start(), 0);
}

#[test]
fn stream_raw_copy() {
    let mut source = ZipWriter::new(Cursor::new(Vec::new()));
    source.start_file("file", stored()).unwrap();
    source.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(source.finish().unwrap()).unwrap();
    let mut w = ZipWriter::new_stream(Vec::new());
    w.raw_copy_file(source.by_index_raw(0).unwrap()).unwrap();
    let mut result = ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
    let mut data = Vec::new();
    result.by_name("file").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
    // A descriptor is required when bit 3 is set.
    let buf = result.into_inner().into_inner();
    assert!(buf.windows(4).any(|bytes| bytes == b"PK\x07\x08"));
}

#[test]
fn stream_aes_current_seek_limitation() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("file", stored().with_aes_encryption(zip::AesMode::Aes128, "password")).unwrap();
    w.write_all(b"hello").unwrap();
    assert!(w.finish().is_err());
}

#[test]
fn documented_symlink_path_normalization() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_symlink_from_path("bin/link", "../lib/target", stored()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut target = String::new();
    result.by_name("bin/link").unwrap().read_to_string(&mut target).unwrap();
    assert_eq!(target, "lib/target");
}

#[test]
fn extract_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_directory("dir", stored().unix_permissions(0o500)).unwrap();
    w.start_file("dir/file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let mut archive = ZipArchive::new(w.finish().unwrap()).unwrap();
    let out = tempfile::tempdir().unwrap();
    archive.extract(out.path()).unwrap();
    assert_eq!(std::fs::metadata(out.path().join("dir")).unwrap().permissions().mode() & 0o777, 0o500);
}

#[test]
fn aes_finish_into_readable() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_aes_encryption(zip::AesMode::Aes128, "password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut result = w.finish_into_readable().unwrap();
    let mut data = Vec::new();
    result.by_name_decrypt("file", b"password").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn zip64_finish_into_readable_payload() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut archive = w.finish_into_readable().unwrap();
    let mut file = archive.by_name("file").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn too_long_filename_returns_error() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.start_file("x".repeat(65536), stored())));
    if result.is_err() { w.abort_file().unwrap(); }
    assert!(result.is_ok(), "start_file panicked instead of returning an error");
    assert!(result.unwrap().is_err());
}

#[test]
fn zip64_finish_reopen_control() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().large_file(true)).unwrap();
    w.write_all(b"hello").unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut data = Vec::new();
    assert_eq!(result.by_name("file").unwrap().size(), 5);
    result.by_name("file").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn aes_finish_reopen_control() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_aes_encryption(zip::AesMode::Aes128, "password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut data = Vec::new();
    result.by_name_decrypt("file", b"password").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn raw_copy_zipcrypto_payload() {
    use zip::unstable::write::FileOptionsExt;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_deprecated_encryption(b"password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(source.by_index_raw(0).unwrap()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut file = result.by_name_decrypt("file", b"password").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn raw_copy_metadata() {
    use zip::write::FullFileOptions;
    let mut opts = FullFileOptions::default().compression_method(CompressionMethod::Stored);
    opts.add_extra_data(0xbeef, Box::from(*b"test"), false).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", opts).unwrap();
    w.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    let expected_extra = source.by_name("file").unwrap().extra_data().unwrap().to_vec();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(source.by_name("file").unwrap()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert_eq!(result.by_name("file").unwrap().extra_data().unwrap(), expected_extra);
}

#[test]
fn raw_copy_decrypted_aes_file() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_aes_encryption(zip::AesMode::Aes128, "password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut source = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.raw_copy_file(source.by_name_decrypt("file", b"password").unwrap()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut data = Vec::new();
    result.by_name_decrypt("file", b"password").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn unicode_comment_with_cp437_original() {
    let mut archive = ZipArchive::new(Cursor::new(include_bytes!("data/review_unicode_comment.zip"))).unwrap();
    assert_eq!(archive.by_name("file").unwrap().comment(), "café");
}

#[test]
fn access_only_extended_timestamp() {
    let mut bytes = Cursor::new([2, 42, 0, 0, 0]);
    let timestamp = zip::extra_fields::ExtendedTimestamp::try_from_reader(&mut bytes, 5).unwrap();
    assert_eq!(timestamp.mod_time(), None);
    assert_eq!(timestamp.ac_time(), Some(42));
}

#[test]
fn seek_api_encrypted_entry() {
    use zip::unstable::write::FileOptionsExt;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file("file", stored().with_deprecated_encryption(b"password")).unwrap();
    w.write_all(b"hello").unwrap();
    let mut archive = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(archive.by_name("file").is_err());
    let mut file = archive.by_name_seek("file").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn directory_by_path() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.add_directory_from_path("dir/subdir/", stored()).unwrap();
    let mut archive = ZipArchive::new(w.finish().unwrap()).unwrap();
    assert!(archive.by_name("dir/subdir/").unwrap().is_dir());
    assert!(archive.by_path("dir/subdir/").unwrap().is_dir());
}

#[test]
fn append_preserves_implode_flags() {
    let original = include_bytes!("data/legacy/implode.zip").to_vec();
    let mut original_archive = ZipArchive::new(Cursor::new(original.clone())).unwrap();
    let mut expected = Vec::new();
    original_archive.by_index(0).unwrap().read_to_end(&mut expected).unwrap();
    let w = ZipWriter::new_append(Cursor::new(original)).unwrap();
    let mut archive = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut actual = Vec::new();
    archive.by_index(0).unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn merge_preserves_implode_flags() {
    let original = include_bytes!("data/legacy/implode.zip").to_vec();
    let source = ZipArchive::new(Cursor::new(original)).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.merge_archive(source).unwrap();
    let mut archive = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut actual = Vec::new();
    archive.by_index(0).unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, include_bytes!("data/folder/first.txt"));
}

#[test]
fn append_preserves_zip64_header_offset() {
    use std::io::{Seek, SeekFrom};
    let mut file = tempfile::tempfile_in("/work/tmp").unwrap();
    file.seek(SeekFrom::Start(zip::ZIP64_BYTES_THR + 1)).unwrap();
    let mut w = ZipWriter::new(file);
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let file = w.finish().unwrap();
    let mut original = ZipArchive::new(file).unwrap();
    let mut data = Vec::new();
    original.by_name("file").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
    let w = ZipWriter::new_append(original.into_inner()).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    data.clear();
    result.by_name("file").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn merge_generates_zip64_header_offset() {
    use std::io::{Seek, SeekFrom};
    let mut source = ZipWriter::new(Cursor::new(Vec::new()));
    source.start_file("file", stored()).unwrap();
    source.write_all(b"hello").unwrap();
    let source = ZipArchive::new(source.finish().unwrap()).unwrap();
    let mut file = tempfile::tempfile_in("/work/tmp").unwrap();
    file.seek(SeekFrom::Start(zip::ZIP64_BYTES_THR + 1)).unwrap();
    let mut w = ZipWriter::new(file);
    w.merge_archive(source).unwrap();
    let mut result = ZipArchive::new(w.finish().unwrap()).unwrap();
    let mut data = Vec::new();
    result.by_name("file").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn stream_directory_descriptor() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.add_directory("dir", stored()).unwrap();
    let bytes = w.finish().unwrap().into_inner();
    assert_eq!(u16::from_le_bytes(bytes[6..8].try_into().unwrap()) & 8, 8);
    assert!(bytes.windows(4).any(|bytes| bytes == b"PK\x07\x08"));
}

#[test]
fn stream_regular_file_descriptor_control() {
    let mut w = ZipWriter::new_stream(Vec::new());
    w.start_file("file", stored()).unwrap();
    w.write_all(b"hello").unwrap();
    let bytes = w.finish().unwrap().into_inner();
    assert_eq!(u16::from_le_bytes(bytes[6..8].try_into().unwrap()) & 8, 8);
    assert!(bytes.windows(4).any(|bytes| bytes == b"PK\x07\x08"));
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut actual = Vec::new();
    archive.by_name("file").unwrap().read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"hello");
}
