//! zip-3 (wrong-behavior): ZipArchive::extract creates symlinks from the
//! archive with no containment check. make_symlink passes the target straight to
//! the OS (src/read.rs:465) and extract calls it for symlink entries
//! (src/read.rs:908) without verifying the target stays inside the destination.
//! So extraction silently places a symlink inside the destination that resolves
//! to a file *outside* it; reading through that symlink reads the outside file.
//!
//! Correct behavior: extraction must not create a symlink that leaves the
//! destination directory (the fix rejects such an entry, and its regression
//! test `test_cannot_create_symlink_pointing_outside_destination` asserts
//! extract() returns Err and leaves no such link). The escape target here is
//! another folder under /tmp. Exit 0 only if extract leaves a symlink that
//! escapes the destination.
//!
//! Note: the two-hop *write* escape (writing a file through a chained symlink)
//! is already blocked at this baseline; only the unchecked symlink creation is
//! present here.

#![cfg(unix)]

use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[test]
fn extract_creates_symlink_escaping_destination() -> zip::result::ZipResult<()> {
    let root = std::path::PathBuf::from("/tmp/zip3b_root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let dest = root.join("dest");
    let outside = root.join("outside");
    std::fs::create_dir(&dest)?;
    std::fs::create_dir(&outside)?;
    // A file outside the destination directory.
    let secret = outside.join("secret.txt");
    std::fs::write(&secret, b"outside-secret")?;

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    // A symlink whose target is a file outside the destination directory.
    writer.add_symlink("evil", secret.to_str().unwrap(), options)?;
    let mut reader = writer.finish_into_readable()?;

    let res = reader.extract(&dest);
    let link = dest.join("evil");
    let is_link = link.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false);
    eprintln!("extract result = {res:?}; dest/evil is symlink = {is_link}");

    assert!(
        is_link,
        "BUG NOT PRESENT: no symlink was created at dest/evil (extract={res:?})"
    );
    // The symlink resolves outside the destination: reading it reads the outside file.
    let via_link = std::fs::read(&link).expect("read through the extracted symlink");
    let resolved = std::fs::canonicalize(&link).expect("canonicalize the symlink");
    assert_eq!(via_link, b"outside-secret");
    assert!(
        !resolved.starts_with(&dest),
        "BUG NOT PRESENT: the symlink resolves inside the destination ({resolved:?})"
    );
    eprintln!(
        "REPRODUCED: extract created dest/evil -> {} (outside the destination); extract returned {:?}",
        resolved.display(),
        res.is_ok()
    );
    Ok(())
}
