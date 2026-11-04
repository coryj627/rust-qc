use std::io::Cursor;
use zip::{ZipWriter,write::SimpleFileOptions};
fn main(){match std::env::args().nth(1).as_deref(){Some("comment")=>{let d=std::fs::read("/work/tmp/fixtures/long_comment.zip").unwrap();let w=ZipWriter::new_append(Cursor::new(d)).unwrap();let _=w.finish();},_=>{let mut w=ZipWriter::new(Cursor::new(Vec::new()));let _=w.start_file("a".repeat(65536),SimpleFileOptions::default());}}}
