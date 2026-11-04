use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};
struct SparseSink {
    file: File,
    pos: u64,
    length: u64,
}
impl Write for SparseSink {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if b.len() >= 4096 && b.iter().all(|&x| x == 0) {
            self.pos = self.file.seek(SeekFrom::Current(b.len() as i64))?;
        } else {
            self.file.write_all(b)?;
            self.pos += b.len() as u64;
        }
        self.length = self.length.max(self.pos);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
impl Seek for SparseSink {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let dest = match from {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
            SeekFrom::End(d) => self.length.checked_add_signed(d),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid seek"))?;
        self.pos = self.file.seek(SeekFrom::Start(dest))?;
        Ok(self.pos)
    }
}
fn sink(label: &str) -> SparseSink {
    SparseSink {
        file: OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("/work/tmp/zip-review/fixtures/{label}.zip"))
            .unwrap(),
        pos: 0,
        length: 0,
    }
}
fn write_large<W: Write + Seek>(mut writer: ZipWriter<W>, method: CompressionMethod) -> W {
    writer
        .start_file(
            "zeros",
            SimpleFileOptions::default()
                .compression_method(method)
                .large_file(true),
        )
        .unwrap();
    let block = vec![0; 1 << 20];
    for _ in 0..4096 {
        writer.write_all(&block).unwrap();
    }
    writer.finish().unwrap()
}
fn size(file: File) -> u64 {
    let mut a = ZipArchive::new(file).unwrap();
    let f = a.by_index_raw(0).unwrap();
    f.size()
}
#[test]
fn streaming_zip64_retains_four_gib_size() {
    let out = write_large(
        ZipWriter::new_stream(sink("stream-four-gib")),
        CompressionMethod::Stored,
    )
    .into_inner();
    assert_eq!(size(out.file), 1_u64 << 32);
}
#[test]
fn seekable_zip64_four_gib_control() {
    let out = write_large(
        ZipWriter::new(sink("seekable-four-gib")),
        CompressionMethod::Stored,
    );
    assert_eq!(size(out.file), 1_u64 << 32);
}

#[test]
fn streaming_zip64_compressed_retains_four_gib_size() {
    let out = write_large(
        ZipWriter::new_stream(sink("stream-deflated-four-gib")),
        CompressionMethod::Deflated,
    )
    .into_inner();
    assert_eq!(size(out.file), 1_u64 << 32);
}
