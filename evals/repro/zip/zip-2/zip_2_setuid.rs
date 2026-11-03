//! zip-2 (wrong-behavior): ZipArchive::extract keeps the SUID/SGID/sticky bits
//! from an entry's external attributes. unix_mode() returns `external_attributes
//! >> 16` unmasked (src/types.rs:622), extract records that mode for regular
//! files (src/read.rs:919) and set_permissions applies it verbatim
//! (src/read.rs:942). A non-root user can set setuid on a file it owns, so the
//! extracted file keeps bit 0o4000.
//!
//! Correct behavior: the special bits are attacker-controlled and must be
//! cleared (the fix masks with !0o7000; `unzip` does not honour them). Exit 0
//! only if the extracted file carries the setuid bit it should not.

#![cfg(unix)]

use std::io::{Cursor, Write};
use std::os::unix::fs::PermissionsExt;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

fn archive_with_mode(mode: u32) -> Vec<u8> {
    let cur = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cur);
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);
    zip.start_file("suid_check", opts).unwrap();
    zip.write_all(b"binary_file").unwrap();
    let mut buf = zip.finish().unwrap().into_inner();

    // Patch the central directory external attributes to the requested mode.
    let sig = [0x50, 0x4b, 0x01, 0x02];
    for i in 0..buf.len().saturating_sub(46) {
        if buf[i..i + 4] == sig {
            let off = i + 38;
            buf[off..off + 4].copy_from_slice(&(mode << 16).to_le_bytes());
            break;
        }
    }
    buf
}

#[test]
fn extract_preserves_setuid_bit() {
    // regular file + suid + 0o755
    let data = archive_with_mode(0o104755);
    let dest = std::path::PathBuf::from("/tmp/zip2_dest");
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).unwrap();

    let mut zip = ZipArchive::new(Cursor::new(data)).unwrap();
    assert_eq!(zip.by_index(0).unwrap().unix_mode(), Some(0o104755));
    zip.extract(&dest).unwrap();

    let target = dest.join("suid_check");
    let mode = std::fs::metadata(&target).unwrap().permissions().mode();
    eprintln!("extracted mode = {:o}", mode);
    assert!(
        mode & 0o4000 != 0,
        "BUG NOT PRESENT: setuid bit was stripped (mode {:o})",
        mode
    );
    eprintln!("REPRODUCED: extract kept the setuid bit (0o4000) from external attributes");
}
