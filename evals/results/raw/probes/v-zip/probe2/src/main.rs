use std::io::{Cursor,Write};
use zip::{ZipWriter,CompressionMethod as CM};
use zip::write::SimpleFileOptions as O;
fn main(){ let l:i64=std::env::args().nth(1).unwrap().parse().unwrap(); let mut z=ZipWriter::new(Cursor::new(vec![])); println!("start {:?}",z.start_file("a",O::default().compression_method(CM::Deflated).compression_level(Some(l))).map_err(|e|e.to_string())); println!("w {:?}",z.write_all(b"hello").map_err(|e|e.to_string())); println!("fin {:?}",z.finish().map(|_|()).map_err(|e|e.to_string())); println!("OK level {l}"); }
