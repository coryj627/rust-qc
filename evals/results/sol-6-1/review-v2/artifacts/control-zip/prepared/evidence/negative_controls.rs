//! Small public-API counterexamples for seven negative controls, zip 5.1.1.
//! In-memory archives only; no extraction or external input.
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

fn one_file(name: &str, payload: &[u8], method: CompressionMethod) -> Cursor<Vec<u8>> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file(name, stored().compression_method(method)).unwrap();
    writer.write_all(payload).unwrap();
    writer.finish().unwrap()
}

#[test]
fn decoy_1_zero_length_read_does_not_validate_partial_crc() {
    let mut archive = ZipArchive::new(one_file("entry", b"1234", CompressionMethod::Stored)).unwrap();
    let mut file = archive.by_name("entry").unwrap();
    assert_ne!(file.crc32(), 0);
    assert_eq!(file.read(&mut []).unwrap(), 0);
    let mut prefix = [0; 2];
    file.read_exact(&mut prefix).unwrap();
    assert_eq!(&prefix, b"12");
    // At this point the CRC accumulator covers only a prefix of the file.
    assert_eq!(file.read(&mut []).unwrap(), 0);
    let mut suffix = Vec::new();
    assert_eq!(file.read_to_end(&mut suffix).unwrap(), 2);
    assert_eq!(&suffix, b"34");
    assert_eq!(file.read(&mut [0]).unwrap(), 0);

    // A real checksum mismatch is still detected on a nonempty EOF read.
    let mut archive = ZipArchive::new(one_file("entry", b"1234", CompressionMethod::Stored)).unwrap();
    let data_start = archive.by_name("entry").unwrap().data_start() as usize;
    let mut bytes = archive.into_inner().into_inner();
    bytes[data_start] = b'X';
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name("entry").unwrap();
    let mut exact = [0; 4];
    file.read_exact(&mut exact).unwrap();
    let error = file.read(&mut [0]).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    println!("decoy-1: empty reads before/during data succeeded; remaining bytes and EOF correct; changed-byte checksum mismatch detected at nonempty EOF");
}

#[test]
fn decoy_2_gregorian_century_leap_days() {
    let leap = DateTime::from_date_and_time(2000, 2, 29, 12, 34, 56).unwrap();
    assert_eq!((leap.year(), leap.month(), leap.day()), (2000, 2, 29));
    assert!(leap.is_valid());
    assert!(DateTime::from_date_and_time(2100, 2, 29, 12, 34, 56).is_err());
    assert!(DateTime::from_date_and_time(2024, 2, 29, 0, 0, 0).is_ok());
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file("leap", stored().last_modified_time(leap)).unwrap();
    writer.write_all(b"ok").unwrap();
    let mut archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
    assert_eq!(archive.by_name("leap").unwrap().last_modified(), Some(leap));
    println!("decoy-2: 2000-02-29 accepted and archive-roundtripped; 2100-02-29 rejected; 2024-02-29 accepted");
}

#[test]
fn false_1_duplicate_name_rejected_without_replacing_original() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file("same.txt", stored()).unwrap();
    writer.write_all(b"original").unwrap();
    assert!(matches!(writer.start_file("same.txt", stored()), Err(ZipError::InvalidArchive(_))));
    writer.start_file("next.txt", stored()).unwrap();
    writer.write_all(b"next").unwrap();
    let mut archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
    assert_eq!(archive.len(), 2);
    let mut original = String::new();
    archive.by_name("same.txt").unwrap().read_to_string(&mut original).unwrap();
    assert_eq!(original, "original");
    let mut next = String::new();
    archive.by_name("next.txt").unwrap().read_to_string(&mut next).unwrap();
    assert_eq!(next, "next");
    println!("false-1: duplicate returned InvalidArchive; original contents preserved; next file remained writable/readable");
}

#[test]
fn false_2_append_preserves_non_utf8_comment() {
    let comment = [0xff, 0x00, b'A', 0x80];
    assert!(std::str::from_utf8(&comment).is_err());
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.set_raw_comment(comment.to_vec().into_boxed_slice());
    writer.start_file("before", stored()).unwrap();
    writer.write_all(b"before").unwrap();
    let mut writer = ZipWriter::new_append(writer.finish().unwrap()).unwrap();
    assert_eq!(writer.get_raw_comment(), &comment);
    writer.start_file("after", stored()).unwrap();
    writer.write_all(b"after").unwrap();
    let archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
    assert_eq!(archive.comment(), &comment);
    assert_eq!(archive.len(), 2);
    println!("false-2: append succeeded with ff 00 41 80 comment; all comment bytes survived finish/reopen; two entries retained");
}

#[test]
fn false_3_seek_from_end_is_relative_to_entry() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file("first", stored()).unwrap();
    writer.write_all(b"ABCDE").unwrap();
    writer.start_file("second", stored()).unwrap();
    writer.write_all(b"later entry").unwrap();
    let mut archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
    let mut file = archive.by_name_seek("first").unwrap();
    assert_eq!(file.seek(SeekFrom::End(-2)).unwrap(), 3);
    let mut tail = Vec::new();
    assert_eq!(file.read_to_end(&mut tail).unwrap(), 2);
    assert_eq!(&tail, b"DE");
    assert_eq!(file.seek(SeekFrom::End(0)).unwrap(), 5);
    assert_eq!(file.read(&mut [0; 8]).unwrap(), 0);
    assert_eq!(file.seek(SeekFrom::Start(0)).unwrap(), 0);
    let mut all = Vec::new();
    file.read_to_end(&mut all).unwrap();
    assert_eq!(&all, b"ABCDE");
    println!("false-3: End(-2) returned entry offset 3 and DE; End(0) returned 5 and EOF; subsequent entry/header bytes were not read");
}

#[test]
fn false_4_raw_copy_uses_compressed_bytes_from_normal_zipfile() {
    let payload = b"repeated payload repeated payload repeated payload repeated payload";
    let mut source = ZipArchive::new(one_file("a", payload, CompressionMethod::Deflated)).unwrap();
    let source_crc = source.by_name("a").unwrap().crc32();
    let mut source_raw = Vec::new();
    source.by_index_raw(0).unwrap().read_to_end(&mut source_raw).unwrap();
    // Deliberately pass the ordinary, transparently decompressing public reader.
    let file = source.by_name("a").unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.raw_copy_file_rename(file, "a-longer-destination-name.txt").unwrap();
    let mut copied = ZipArchive::new(writer.finish().unwrap()).unwrap();
    let mut destination_raw = Vec::new();
    copied.by_index_raw(0).unwrap().read_to_end(&mut destination_raw).unwrap();
    assert_eq!(destination_raw, source_raw);
    let mut file = copied.by_name("a-longer-destination-name.txt").unwrap();
    assert_eq!(file.compression(), CompressionMethod::Deflated);
    assert_eq!(file.crc32(), source_crc);
    assert_eq!(file.compressed_size(), source_raw.len() as u64);
    let mut actual = Vec::new();
    file.read_to_end(&mut actual).unwrap();
    assert_eq!(&actual, payload);
    println!("false-4: ordinary by_name ZipFile raw-copied and renamed; compressed bytes identical; Deflated metadata, CRC and decompressed payload correct");
}

#[test]
fn false_5_directory_forces_stored_method() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = stored().compression_method(CompressionMethod::Deflated);
    writer.add_directory("folder", options).unwrap();
    writer.start_file("folder/child", options).unwrap();
    writer.write_all(b"child data").unwrap();
    let mut archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
    let mut directory = archive.by_name("folder/").unwrap();
    assert!(directory.is_dir());
    assert_eq!(directory.compression(), CompressionMethod::Stored);
    assert_eq!(directory.size(), 0);
    assert_eq!(directory.compressed_size(), 0);
    let mut contents = Vec::new();
    assert_eq!(directory.read_to_end(&mut contents).unwrap(), 0);
    assert!(contents.is_empty());
    drop(directory);
    let mut child = String::new();
    archive.by_name("folder/child").unwrap().read_to_string(&mut child).unwrap();
    assert_eq!(child, "child data");
    println!("false-5: Deflated options yielded Stored empty directory with successful EOF; subsequent deflated child roundtripped");
}
