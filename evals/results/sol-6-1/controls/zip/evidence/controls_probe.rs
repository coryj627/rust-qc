use std::io::{Cursor, ErrorKind, Read, Write};
use zip::{AesMode, CompressionMethod, DateTime, ZipArchive, ZipWriter};
use zip::write::SimpleFileOptions;

fn stored_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

fn write_one(name: &str, payload: &[u8], options: SimpleFileOptions) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file(name, options).unwrap();
    writer.write_all(payload).unwrap();
    writer.finish().unwrap().into_inner()
}

fn u16le(v: &mut Vec<u8>, n: u16) { v.extend_from_slice(&n.to_le_bytes()); }
fn u32le(v: &mut Vec<u8>, n: u32) { v.extend_from_slice(&n.to_le_bytes()); }

// A minimal one-entry stored ZIP, with caller-supplied central extra data.
fn raw_stored(name: &[u8], payload: &[u8], central_extra: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    let crc = crc32fast::hash(payload);
    u32le(&mut v, 0x04034b50);
    for n in [20, 0, 0, 0, 33] { u16le(&mut v, n); }
    u32le(&mut v, crc);
    u32le(&mut v, payload.len() as u32);
    u32le(&mut v, payload.len() as u32);
    u16le(&mut v, name.len() as u16);
    u16le(&mut v, 0);
    v.extend_from_slice(name);
    v.extend_from_slice(payload);
    let central_start = v.len();
    u32le(&mut v, 0x02014b50);
    for n in [0x0314, 20, 0, 0, 0, 33] { u16le(&mut v, n); }
    u32le(&mut v, crc);
    u32le(&mut v, payload.len() as u32);
    u32le(&mut v, payload.len() as u32);
    u16le(&mut v, name.len() as u16);
    u16le(&mut v, central_extra.len() as u16);
    for n in [0, 0, 0] { u16le(&mut v, n); }
    u32le(&mut v, 0);
    u32le(&mut v, 0);
    v.extend_from_slice(name);
    v.extend_from_slice(central_extra);
    let central_size = v.len() - central_start;
    u32le(&mut v, 0x06054b50);
    for n in [0, 0, 1, 1] { u16le(&mut v, n); }
    u32le(&mut v, central_size as u32);
    u32le(&mut v, central_start as u32);
    u16le(&mut v, 0);
    v
}

#[test]
fn decoy_1_bulk_reads_validate_crc() {
    let payload = b"checksum-probe";
    let bytes = write_one("data.txt", payload, stored_options());
    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let mut valid = b"existing prefix:".to_vec();
    let start;
    {
        let mut file = archive.by_index(0).unwrap();
        start = file.data_start() as usize;
        assert_eq!(file.read_to_end(&mut valid).unwrap(), payload.len());
    }
    assert_eq!(valid, [b"existing prefix:".as_slice(), payload].concat());
    let mut corrupt = bytes;
    corrupt[start] ^= 1;
    for string_mode in [false, true] {
        let mut archive = ZipArchive::new(Cursor::new(corrupt.clone())).unwrap();
        let mut file = archive.by_index(0).unwrap();
        let err = if string_mode {
            file.read_to_string(&mut String::from("prefix:")).unwrap_err()
        } else {
            file.read_to_end(&mut b"prefix:".to_vec()).unwrap_err()
        };
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid checksum"));
        println!("decoy-1: read_to_{} rejects modified stored payload: {err}", if string_mode {"string"} else {"end"});
    }
    println!("decoy-1: valid read_to_end with prefilled destination succeeds");
}

#[test]
fn decoy_2_duplicate_names_are_rejected() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file("same.txt", stored_options()).unwrap();
    writer.write_all(b"original").unwrap();
    let err = writer.start_file("same.txt", stored_options()).unwrap_err();
    assert!(err.to_string().contains("Duplicate filename"));
    println!("decoy-2: fresh writer rejects duplicate: {err}");
    let bytes = writer.finish().unwrap();
    let mut append = ZipWriter::new_append(bytes).unwrap();
    let err = append.start_file("same.txt", stored_options()).unwrap_err();
    assert!(err.to_string().contains("Duplicate filename"));
    println!("decoy-2: append writer rejects existing name: {err}");
    let mut archive = ZipArchive::new(append.finish().unwrap()).unwrap();
    assert_eq!(archive.len(), 1);
    let mut out = String::new();
    archive.by_name("same.txt").unwrap().read_to_string(&mut out).unwrap();
    assert_eq!(out, "original");
    println!("decoy-2: original single entry remains readable after both rejected attempts");
}

#[test]
fn false_1_enclosed_name_rejects_transient_underflow() {
    for name in ["../escape", "a/../../escape", "a/../../b/c", "/absolute", "nul\0name"] {
        let bytes = raw_stored(name.as_bytes(), b"x", &[]);
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert!(archive.by_index(0).unwrap().enclosed_name().is_none(), "{name:?}");
        println!("false-1: enclosed_name rejects {name:?}");
    }
    let mut archive = ZipArchive::new(Cursor::new(raw_stored(b"a/../inside", b"x", &[]))).unwrap();
    assert!(archive.by_index(0).unwrap().enclosed_name().is_some());
    println!("false-1: contained a/../inside is accepted");
}

#[test]
fn false_2_unicode_path_requires_matching_crc() {
    let original = b"plain.txt";
    let replacement = b"renamed.txt";
    let checksum = crc32fast::hash(original);
    for mismatch in [false, true] {
        let mut extra = Vec::new();
        u16le(&mut extra, 0x7075);
        u16le(&mut extra, (5 + replacement.len()) as u16);
        extra.push(1);
        u32le(&mut extra, checksum ^ u32::from(mismatch));
        extra.extend_from_slice(replacement);
        let result = ZipArchive::new(Cursor::new(raw_stored(original, b"payload", &extra)));
        if mismatch {
            let err = result.unwrap_err();
            assert!(matches!(err, zip::result::ZipError::InvalidArchive(_)));
            println!("false-2: identical Unicode field with mismatched CRC rejects archive: {err}");
        } else {
            let mut archive = result.unwrap();
            let mut file = archive.by_name("renamed.txt").unwrap();
            assert_eq!(file.name(), "renamed.txt");
            let mut out = String::new();
            file.read_to_string(&mut out).unwrap();
            assert_eq!(out, "payload");
            println!("false-2: matching CRC accepts replacement name and reads correct payload");
        }
    }
}

#[test]
fn false_3_century_leap_year_validation() {
    assert!(DateTime::from_date_and_time(2100, 2, 29, 0, 0, 0).is_err());
    assert!(DateTime::from_date_and_time(2100, 2, 28, 0, 0, 0).is_ok());
    assert!(DateTime::from_date_and_time(2000, 2, 29, 0, 0, 0).is_ok());
    assert!(DateTime::from_date_and_time(2024, 2, 29, 0, 0, 0).is_ok());
    let invalid_datepart = ((2100u16 - 1980) << 9) | (2 << 5) | 29;
    assert!(DateTime::try_from_msdos(invalid_datepart, 0).is_err());
    println!("false-3: both constructors reject 2100-02-29; 2100-02-28, 2000-02-29, 2024-02-29 accepted");
}

#[test]
fn false_4_ae2_checks_authentication_tag() {
    let payload = b"tag-probe";
    let bytes = write_one("secret.txt", payload,
        stored_options().with_aes_encryption(AesMode::Aes256, "test-password"));
    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let start;
    let size;
    {
        let mut file = archive.by_index_decrypt(0, b"test-password").unwrap();
        assert!(file.encrypted());
        assert_eq!(file.crc32(), 0); // This writer emits AE-2.
        start = file.data_start() as usize;
        size = file.compressed_size() as usize;
        let mut out = Vec::new();
        file.read_to_end(&mut out).unwrap();
        assert_eq!(out, payload);
    }
    let mut corrupt = bytes;
    corrupt[start + size - 1] ^= 1; // Alter only the final authentication-code byte.
    let mut archive = ZipArchive::new(Cursor::new(corrupt)).unwrap();
    let mut file = archive.by_index_decrypt(0, b"test-password").unwrap();
    let err = file.read_to_end(&mut Vec::new()).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(err.to_string().contains("Invalid authentication code"));
    println!("false-4: valid nonempty AE-2 entry round-trips with CRC=0; changing only tag rejects full read: {err}");
}

#[test]
fn false_5_alignment_padding_includes_extra_header() {
    let name = "elevenchars";
    assert_eq!(name.len(), 11);
    let payload = b"aligned payload";
    let bytes = write_one(name, payload, stored_options().with_alignment(21));
    let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
    assert_eq!(extra_len, 22);
    assert_eq!(&bytes[41..43], &[0x1e, 0xa1]);
    assert_eq!(u16::from_le_bytes([bytes[43], bytes[44]]), 18);
    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let mut file = archive.by_index(0).unwrap();
    assert_eq!(file.data_start(), 63);
    assert_eq!(file.data_start() % 21, 0);
    assert_eq!(&bytes[63..63 + payload.len()], payload);
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    assert_eq!(out, payload);
    println!("false-5: header+name=41, full padding field=22 (4 header + 18 body), data_start=63 divisible by 21; exact payload round-trips");
}
