use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ExtraField, ZipArchive, ZipWriter};

#[test]
fn unicode_comment_uses_crc_of_original_bytes() {
    let mut archive = ZipArchive::new(Cursor::new(include_bytes!(
        "../fixtures/unicode-comment-cp437.zip"
    )))
    .unwrap();
    assert_eq!(archive.by_index(0).unwrap().comment(), "café");
}
#[test]
fn unicode_comment_ascii_control() {
    let mut archive = ZipArchive::new(Cursor::new(include_bytes!(
        "../fixtures/unicode-comment-ascii.zip"
    )))
    .unwrap();
    assert_eq!(archive.by_index(0).unwrap().comment(), "café");
}
#[test]
fn stale_unicode_path_uses_current_name() {
    let mut archive = ZipArchive::new(Cursor::new(include_bytes!(
        "../fixtures/unicode-path-stale.zip"
    )))
    .unwrap();
    assert_eq!(archive.by_index(0).unwrap().name(), "file");
}
#[test]
fn raw_copy_preserves_comment() {
    let mut source = ZipArchive::new(Cursor::new(include_bytes!(
        "../fixtures/extra-and-comment.zip"
    )))
    .unwrap();
    let mut copy = ZipWriter::new(Cursor::new(Vec::new()));
    copy.raw_copy_file(source.by_index_raw(0).unwrap()).unwrap();
    let mut archive = ZipArchive::new(copy.finish().unwrap()).unwrap();
    assert_eq!(archive.by_index(0).unwrap().comment(), "file comment");
}
#[test]
fn local_access_timestamp_is_not_modification_timestamp() {
    let mut cursor = Cursor::new(include_bytes!("../fixtures/access-time-only.zip"));
    let file = zip::read::read_zipfile_from_stream(&mut cursor)
        .unwrap()
        .unwrap();
    let ts = file
        .extra_data_fields()
        .find_map(|x| match x {
            ExtraField::ExtendedTimestamp(x) => Some(x),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        (ts.mod_time(), ts.ac_time(), ts.cr_time()),
        (None, Some(123456789), None)
    );
}
#[test]
fn writer_compression_matrix_roundtrip() {
    let methods = [
        CompressionMethod::Stored,
        CompressionMethod::Deflated,
        CompressionMethod::Bzip2,
        CompressionMethod::Zstd,
        CompressionMethod::Xz,
        CompressionMethod::Ppmd,
    ];
    let mut failures = Vec::new();
    for method in methods {
        for bytes in [b"".as_slice(), b"hello", "héllo, 世界".as_bytes()] {
            for large in [false, true] {
                let mut w = ZipWriter::new(Cursor::new(Vec::new()));
                w.start_file(
                    "file",
                    SimpleFileOptions::default()
                        .compression_method(method)
                        .large_file(large),
                )
                .unwrap();
                w.write_all(bytes).unwrap();
                let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
                let mut f = a.by_index(0).unwrap();
                assert_eq!(f.size(), bytes.len() as u64, "{method:?}, large={large}");
                let mut got = Vec::new();
                let res = f.read_to_end(&mut got);
                if res.is_err() {
                    failures.push(format!(
                        "{method:?}, large={large}, len={}: {res:?}",
                        bytes.len()
                    ));
                    continue;
                }
                assert_eq!(got, bytes, "{method:?}, large={large}");
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn append_preserves_aes_wire_method() {
    use zip::AesMode;
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "file",
        SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .with_aes_encryption(AesMode::Aes128, "password"),
    )
    .unwrap();
    w.write_all(b"hello").unwrap();
    let before = w.finish().unwrap();
    fn central_method(bytes: &[u8]) -> u16 {
        let pos = bytes.windows(4).rposition(|s| s == b"PK\x01\x02").unwrap();
        u16::from_le_bytes(bytes[pos + 10..pos + 12].try_into().unwrap())
    }
    assert_eq!(central_method(before.get_ref()), 99);
    let after = ZipWriter::new_append(before).unwrap().finish().unwrap();
    assert_eq!(central_method(after.get_ref()), 99);
}
#[test]
fn invalid_compression_levels_match_documented_zero_boundary() {
    for method in [CompressionMethod::Deflated, CompressionMethod::Bzip2] {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        w.start_file(
            "file",
            SimpleFileOptions::default()
                .compression_method(method)
                .compression_level(Some(0)),
        )
        .unwrap();
        w.write_all(b"hello").unwrap();
        let mut a = ZipArchive::new(w.finish().unwrap()).unwrap();
        let mut got = Vec::new();
        a.by_index(0).unwrap().read_to_end(&mut got).unwrap();
        assert_eq!(got, b"hello");
    }
}
