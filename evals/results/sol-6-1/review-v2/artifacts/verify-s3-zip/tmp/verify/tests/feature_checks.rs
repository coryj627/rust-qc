use std::io::{Cursor,Write};
use zip::{ZipWriter,CompressionMethod};
use zip::write::SimpleFileOptions;
#[test]
#[cfg(all(feature="bzip2",not(feature="_deflate-any")))]
fn bzip_default(){let mut w=ZipWriter::new(Cursor::new(vec![]));w.start_file("a",SimpleFileOptions::default()).unwrap();w.write_all(b"hello").unwrap();let mut a=zip::ZipArchive::new(w.finish().unwrap()).unwrap();assert_eq!(a.by_index(0).unwrap().compression(),CompressionMethod::Stored);}
#[test]
#[cfg(all(feature="deflate-zopfli",not(feature="deflate-flate2")))]
fn zopfli_levels(){for level in [0,1,6,8,9,10,264,265]{let mut w=ZipWriter::new(Cursor::new(vec![]));let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||w.start_file("a",SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).compression_level(Some(level)))));match level{1|6|8|9=>assert!(r.is_err()),0|265=>assert!(r.unwrap().is_err()),_=>{r.unwrap().unwrap();w.write_all(b"x").unwrap();w.finish().unwrap();}}}}
