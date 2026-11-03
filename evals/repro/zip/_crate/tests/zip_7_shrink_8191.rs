//! zip-7 (wrong-behavior): the Shrink (unshrink) decoder never assigns LZW code
//! 8191 (MAX_CODE). CodeQueue::new fills CONTROL_CODE+1..=MAX_CODE-1
//! (src/legacy/shrink.rs:29) and unshrink_partial_clear refills
//! CONTROL_CODE+1..MAX_CODE (shrink.rs:91), both stopping one short of 8191,
//! while the comment says the queue holds codes up to MAX_CODE (shrink.rs:14).
//! Once an encoder's table reaches 8191, every later code is one off from the
//! decoder's table and decoding fails with "invalid code". Needs `legacy-zip`.
//!
//! Input: shrink_full_table.zip, a valid hwzip-made Shrink archive of the first
//! 26001 bytes of `seq 1 60000` (the smallest prefix whose stream uses code
//! 8191); Info-ZIP `unzip` extracts it. Correct behavior is to decode it to
//! exactly that 26001-byte text. Exit 0 only if the baseline decoder fails to
//! read this valid archive (an "invalid code" error or a short/garbled decode).

use std::io::{Cursor, Read};
use zip::ZipArchive;

const ARCHIVE: &[u8] = include_bytes!("../data/shrink_full_table.zip");

#[test]
fn shrink_full_table_fails_to_decode() {
    let mut want = String::new();
    for n in 1..=60000 {
        want.push_str(&format!("{n}\n"));
    }
    let want = &want.as_bytes()[..26001];

    let mut archive = ZipArchive::new(Cursor::new(ARCHIVE.to_vec())).expect("archive opens");
    let mut file = archive.by_index(0).expect("entry present");
    let mut content = Vec::new();
    let read_result = file.read_to_end(&mut content);

    match &read_result {
        Err(e) => eprintln!("read failed: {e}"),
        Ok(n) => eprintln!("read {n} bytes (expected 26001)"),
    }

    let correct = read_result.is_ok() && content.len() == 26001 && content.as_slice() == want;
    assert!(
        !correct,
        "BUG NOT PRESENT: the decoder produced the correct 26001-byte output"
    );
    eprintln!(
        "REPRODUCED: baseline Shrink decoder could not correctly decode a valid full-table archive (code 8191 skipped)"
    );
}
