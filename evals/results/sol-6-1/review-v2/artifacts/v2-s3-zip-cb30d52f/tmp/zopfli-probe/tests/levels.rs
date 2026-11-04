use std::io::Cursor;
use zip::{write::SimpleFileOptions, ZipWriter};

#[test]
fn unsupported_zopfli_levels_return_errors() {
    let mut panics = Vec::new();
    for level in [0, 1, 6, 8, 9, 10, 264, 265] {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            w.start_file(
                "a",
                SimpleFileOptions::default().compression_level(Some(level)),
            )
        }));
        match result {
            Err(_) => panics.push(level),
            Ok(result) => assert_eq!(result.is_ok(), (10..=264).contains(&level)),
        }
    }
    assert!(panics.is_empty(), "panicked for levels {panics:?}");
}
