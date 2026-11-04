use std::io::Cursor;
use zip::ZipWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("/work/tmp/fixtures/cp437-long-comment.zip")?;
    let writer = ZipWriter::new_append(Cursor::new(bytes))?;
    writer.finish()?;
    Ok(())
}
