use std::io::Cursor;
use zip::{ZipWriter, CompressionMethod};
fn main() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let result = writer.start_file("x".repeat(65536), zip::write::SimpleFileOptions::default().compression_method(CompressionMethod::Stored));
    println!("{result:?}");
}
