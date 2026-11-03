use probe::*;
use std::io::{Cursor, Read, Write, Seek};
use std::panic::{catch_unwind, AssertUnwindSafe};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions, CompressionMethod};
use zip::read::{read_zipfile_from_stream};
use zip::unstable::stream::{ZipStreamReader, ZipStreamVisitor, ZipStreamFileMetadata};
use zip::read::ZipFile;
use std::os::unix::fs::{PermissionsExt, MetadataExt};

fn t<F: FnOnce() -> String>(name: &str, f: F) {
    let r = catch_unwind(AssertUnwindSafe(f));
    match r { Ok(s) => println!("[{name}] {s}"),
        Err(e) => { let m = e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(); println!("[{name}] PANIC: {m}") } }
}
fn dest(n: &str) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(format!("/work/tmp/p1/{}/{n}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap(); p
}
fn zip64_sizes(u: u64, c: u64) -> Vec<u8> { let mut b = u.to_le_bytes().to_vec(); b.extend(c.to_le_bytes()); ext(1, &b) }

struct Cnt(u32,u32);
impl ZipStreamVisitor for Cnt {
    fn visit_file<R: Read>(&mut self, _f: &mut ZipFile<'_, R>) -> zip::result::ZipResult<()> { self.0+=1; Ok(()) }
    fn visit_additional_metadata(&mut self, _m: &ZipStreamFileMetadata) -> zip::result::ZipResult<()> { self.1+=1; Ok(()) }
}
fn simple() -> Vec<u8> {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    let o = SimpleFileOptions::default().compression_method(CompressionMethod::Stored).unix_permissions(0o750);
    w.start_file("a.txt", o).unwrap(); w.write_all(b"hello").unwrap();
    w.start_file("b.txt", o).unwrap(); w.write_all(b"world").unwrap();
    w.finish().unwrap().into_inner()
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let on = |n: &str| which.is_empty() || which == n;
    if on("symhuge") {
        t("extract: symlink with zip64 size u64::MAX", || {
            let mut e = E::new("l", b"target").unix(0o120777); e.uncomp = Some(u32::MAX); e.cextra = zip64_sizes(u64::MAX, 0).split_at(4+8).0.to_vec();
            e.cextra = { let b = u64::MAX.to_le_bytes(); ext(1,&b) };
            let d = build(&[e]);
            let mut a = ZipArchive::new(Cursor::new(d)).unwrap();
            format!("{:?}", a.extract(dest("symhuge")))
        });
    }
    if on("stream") {
        t("stream visit counts", || {
            let mut c = Cnt(0,0); let r = ZipStreamReader::new(Cursor::new(simple())).visit(&mut c);
            format!("r={r:?} files={} metadata_visits={}", c.0, c.1)
        });
        t("stream extract perms", || {
            let dd = dest("streamperm"); let r = ZipStreamReader::new(Cursor::new(simple())).extract(&dd);
            format!("r={r:?} mode={:o} (archive 750)", std::fs::metadata(dd.join("a.txt")).unwrap().permissions().mode())
        });
        t("stream extract symlink", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.add_symlink("lnk", "a.txt", SimpleFileOptions::default()).unwrap();
            let v = w.finish().unwrap().into_inner();
            let dd = dest("streamsym"); let r = ZipStreamReader::new(Cursor::new(v)).extract(&dd);
            let m = std::fs::symlink_metadata(dd.join("lnk")).unwrap();
            format!("r={r:?} is_symlink={} is_file={}", m.file_type().is_symlink(), m.file_type().is_file())
        });
        t("stream empty archive", || {
            let w = ZipWriter::new(Cursor::new(vec![]));
            let v = w.finish().unwrap().into_inner();
            let mut c = Cursor::new(v.clone());
            let r = read_zipfile_from_stream(&mut c).map(|x| x.is_some());
            let n = ZipArchive::new(Cursor::new(v)).map(|a| a.len());
            format!("seek reader len={n:?}; stream={r:?}")
        });
        t("stream data_start()", || {
            let mut c = Cursor::new(simple());
            let f = read_zipfile_from_stream(&mut c).unwrap().unwrap();
            format!("data_start={}", f.data_start())
        });
        t("repo test vacuous: Cursor from finish() at end", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("x", SimpleFileOptions::default()).unwrap();
            let c = w.finish().unwrap();
            let pos = c.position(); let len = c.get_ref().len();
            let r = ZipStreamReader::new(c).extract(dest("vac"));
            format!("pos={pos} len={len} extract={r:?}")
        });
    }
    if on("extras") {
        t("0x7075 stale CRC", || {
            let mut b = vec![1u8]; b.extend(0x12345678u32.to_le_bytes()); b.extend("é.txt".as_bytes());
            let mut e = E::new("e.txt", b"x"); e.flags=0; e.cextra = ext(0x7075,&b);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("0x7075 ok CRC, invalid utf8", || {
            let name=b"e.txt"; let mut b = vec![1u8]; b.extend(crc(name).to_le_bytes()); b.extend([0xff,0xfe]);
            let mut e = E::new("e.txt", b"x"); e.cextra = ext(0x7075,&b);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("0x6375 valid, CP437 comment", || {
            let raw=b"caf\x82"; let mut b = vec![1u8]; b.extend(crc(raw).to_le_bytes()); b.extend("café".as_bytes());
            let mut e = E::new("e.txt", b"x"); e.comment=raw.to_vec(); e.cextra = ext(0x6375,&b);
            let r = ZipArchive::new(Cursor::new(build(&[e])));
            match r { Ok(mut a)=>format!("ok comment={:?}", a.by_index(0).unwrap().comment().to_string()), Err(e)=>format!("ERR {e:?}") }
        });
        t("0x6375 valid, ascii comment", || {
            let raw=b"cafe"; let mut b = vec![1u8]; b.extend(crc(raw).to_le_bytes()); b.extend("café".as_bytes());
            let mut e = E::new("e.txt", b"x"); e.comment=raw.to_vec(); e.cextra = ext(0x6375,&b);
            let r = ZipArchive::new(Cursor::new(build(&[e])));
            match r { Ok(mut a)=>format!("ok comment={:?}", a.by_index(0).unwrap().comment().to_string()), Err(e)=>format!("ERR {e:?}") }
        });
        t("ntfs len 36 (extra attr)", || {
            let mut b = vec![0u8;4]; b.extend(1u16.to_le_bytes()); b.extend(24u16.to_le_bytes()); b.extend([1u8;24]); b.extend(2u16.to_le_bytes()); b.extend(0u16.to_le_bytes());
            let mut e = E::new("e.txt", b"x"); e.cextra = ext(0x000a,&b);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("ntfs reserved only (len 4)", || {
            let mut e = E::new("e.txt", b"x"); e.cextra = ext(0x000a,&[0u8;4]);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("UT len 9 flags 1", || {
            let mut b=vec![1u8]; b.extend([0u8;8]);
            let mut e = E::new("e.txt", b"x"); e.cextra = ext(0x5455,&b);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("UT len 1 flags 1 (no times in central)", || {
            let mut e = E::new("e.txt", b"x"); e.cextra = ext(0x5455,&[1u8]);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("UT len 5 flags 0x83 unknown bit", || {
            let mut b=vec![0x83u8]; b.extend([0u8;8]);
            let mut e = E::new("e.txt", b"x"); e.cextra = ext(0x5455,&b);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
        t("AES extra with unknown vendor on Stored unencrypted", || {
            let mut e = E::new("e.txt", b"x"); let mut b=vec![]; b.extend(2u16.to_le_bytes()); b.extend(b"XX"); b.push(3); b.extend(0u16.to_le_bytes()); e.cextra=ext(0x9901,&b);
            format!("{:?}", ZipArchive::new(Cursor::new(build(&[e]))).map(|a| a.len()))
        });
    }
    if on("errs") {
        t("error kept? multi-disk eocd", || {
            let mut v = build(&[E::new("a",b"x")]); let n=v.len(); v[n-22+4]=1; // disk number 1
            format!("{:?}", ZipArchive::new(Cursor::new(v)).map(|a| a.len()))
        });
        t("error kept? IO failure during CD read", || {
            struct R{c:Cursor<Vec<u8>>, fail_from:u64, cdstart:u64}
            impl Read for R { fn read(&mut self,b:&mut [u8])->std::io::Result<usize>{ let p=self.c.position(); if p>=self.cdstart && p<self.fail_from { return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied,"disk failure")) } self.c.read(b) } }
            impl Seek for R { fn seek(&mut self,s:std::io::SeekFrom)->std::io::Result<u64>{self.c.seek(s)} }
            let v=build(&[E::new("a",b"x"),E::new("b",b"y")]); let n=v.len() as u64;
            // second central header fails
            let cd=(n-22)-47*2+ (47-46)+46; 
            let r=R{c:Cursor::new(v),fail_from:n-22,cdstart:cd};
            format!("{:?}", ZipArchive::new(r).map(|a| a.len()))
        });
        t("truncated eocd comment", || {
            let mut v=build_c(&[E::new("a",b"x")],b"0123456789"); v.truncate(v.len()-5);
            format!("{:?}", ZipArchive::new(Cursor::new(v)).map(|a| a.len()))
        });
    }
    if on("sym") {
        t("extract twice over symlink", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("a.txt", SimpleFileOptions::default()).unwrap(); w.write_all(b"x").unwrap();
            w.add_symlink("lnk", "a.txt", SimpleFileOptions::default()).unwrap();
            let v=w.finish().unwrap().into_inner(); let dd=dest("twice");
            let r1=ZipArchive::new(Cursor::new(v.clone())).unwrap().extract(&dd);
            let r2=ZipArchive::new(Cursor::new(v)).unwrap().extract(&dd);
            format!("first={r1:?} second={r2:?}")
        });
        t("file through inner relative symlink", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.add_directory("real", SimpleFileOptions::default()).unwrap();
            w.add_symlink("lnk", "real", SimpleFileOptions::default()).unwrap();
            w.start_file("lnk/f.txt", SimpleFileOptions::default()).unwrap(); w.write_all(b"x").unwrap();
            let v=w.finish().unwrap().into_inner(); let dd=dest("inner");
            format!("{:?}", ZipArchive::new(Cursor::new(v)).unwrap().extract(&dd))
        });
        t("absolute symlink within dest", || {
            let dd=dest("absin");
            let canon=dd.canonicalize().unwrap();
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.add_directory("real", SimpleFileOptions::default()).unwrap();
            w.add_symlink("lnk", canon.join("real").to_str().unwrap(), SimpleFileOptions::default()).unwrap();
            w.start_file("lnk/f.txt", SimpleFileOptions::default()).unwrap(); w.write_all(b"x").unwrap();
            let v=w.finish().unwrap().into_inner();
            format!("{:?}", ZipArchive::new(Cursor::new(v)).unwrap().extract(&dd))
        });
        t("symlink to outside is created", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.add_symlink("evil", "/etc/passwd", SimpleFileOptions::default()).unwrap();
            w.add_symlink("evil2", "../../../../etc/hostname", SimpleFileOptions::default()).unwrap();
            let v=w.finish().unwrap().into_inner(); let dd=dest("outside");
            let r=ZipArchive::new(Cursor::new(v)).unwrap().extract(&dd);
            format!("r={r:?} evil->{:?} evil2->{:?}", std::fs::read_link(dd.join("evil")), std::fs::read_link(dd.join("evil2")))
        });
        t("setuid bits applied", || {
            let e=E::new("su",b"#!/bin/sh\n").unix(0o106755); let dd=dest("suid");
            let r=ZipArchive::new(Cursor::new(build(&[e]))).unwrap().extract(&dd);
            format!("r={r:?} mode={:o}", std::fs::metadata(dd.join("su")).unwrap().mode())
        });
        t("unix system, high attrs zero, DOS archive bit", || {
            let mut e=E::new("z",b"data"); e.ext=0x20; let dd=dest("mode0");
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let um=a.by_index(0).unwrap().unix_mode();
            let r=a.extract(&dd);
            format!("unix_mode={um:?} r={r:?} mode={:o}", std::fs::metadata(dd.join("z")).unwrap().mode()&0o7777)
        });
        t("dir mode from archive", || {
            let dd=dest("dirmode");
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.add_directory("secret", SimpleFileOptions::default().unix_permissions(0o700)).unwrap();
            w.add_directory("ro", SimpleFileOptions::default().unix_permissions(0o555)).unwrap();
            let v=w.finish().unwrap().into_inner();
            let r=ZipArchive::new(Cursor::new(v)).unwrap().extract(&dd);
            format!("r={r:?} secret={:o} ro={:o}", std::fs::metadata(dd.join("secret")).unwrap().mode()&0o7777, std::fs::metadata(dd.join("ro")).unwrap().mode()&0o7777)
        });
        t("dos read-only unix_mode", || {
            let mut e=E::new("r",b"x"); e.vmade=0x0014; e.ext=0x01; let mut f=E::new("w",b"x"); f.vmade=0x0014; f.ext=0x20;
            let mut a=ZipArchive::new(Cursor::new(build(&[e,f]))).unwrap();
            let x=a.by_index(0).unwrap().unix_mode(); let y=a.by_index(1).unwrap().unix_mode();
            format!("readonly={:o} writable={:o}", x.unwrap(), y.unwrap())
        });
        t("partial extraction mid-failure perms", || {
            let dd=dest("midfail");
            let mut bad=E::new("sub/deep/bad",b"oops"); bad.crc=Some(1);
            let good=E::new("sub/deep/good",b"ok");
            let r=ZipArchive::new(Cursor::new(build(&[good,bad]))).unwrap().extract(&dd);
            format!("r={r:?} sub={:o} deep={:o} good={:o}", std::fs::metadata(dd.join("sub")).unwrap().mode()&0o7777, std::fs::metadata(dd.join("sub/deep")).unwrap().mode()&0o7777, std::fs::metadata(dd.join("sub/deep/good")).unwrap().mode()&0o7777)
        });
        t("extract_unwrapped: a/../b", || {
            let dd=dest("unwrap");
            let r=ZipArchive::new(Cursor::new(build(&[E::new("a/../b",b"x")]))).unwrap().extract_unwrapped_root_dir(&dd, zip::read::root_dir_common_filter);
            format!("r={r:?}")
        });
    }
    if on("seek") {
        t("by_name_seek on encrypted", || {
            use zip::unstable::write::FileOptionsExt;
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_deprecated_encryption(b"pw")).unwrap();
            w.write_all(b"topsecret").unwrap();
            w.start_file("a", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_aes_encryption(zip::AesMode::Aes256,"pw")).unwrap();
            w.write_all(b"topsecret").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let noPw=a.by_name("s").map(|_|()).map_err(|e|e.to_string());
            let mut buf=vec![]; let r=a.by_name_seek("s").map(|mut f| f.read_to_end(&mut buf));
            let mut b2=vec![]; let r2=a.by_name_seek("a").map(|mut f| f.read_to_end(&mut b2));
            format!("by_name={noPw:?} seek_zc={:?} bytes={:02x?} seek_aes={:?} bytes={}", r.map_err(|e|e.to_string()), buf, r2.map_err(|e|e.to_string()), b2.len())
        });
    }
    if on("crypto") {
        t("zipcrypto: data descriptor, invalid date", || {
            // encrypt with crate, then patch flag + date
            use zip::unstable::write::FileOptionsExt;
            let mut w = ZipWriter::new_stream(Cursor::new(vec![]));
            w.start_file("s", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_deprecated_encryption(b"pw")).unwrap();
            w.write_all(b"hello").unwrap();
            let v=w.finish().unwrap().into_inner().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let r=a.by_index_decrypt(0,b"pw").map(|mut f|{let mut s=String::new(); f.read_to_string(&mut s).map(|_|s)}).map_err(|e|e.to_string());
            format!("new_stream + ZipCrypto, right password: {r:?}")
        });
        t("zipcrypto keystream reuse", || {
            use zip::unstable::write::FileOptionsExt;
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_deprecated_encryption(b"pw");
            w.start_file("a", o).unwrap(); w.write_all(b"AAAAAAAAAAAA").unwrap();
            w.start_file("b", o).unwrap(); w.write_all(b"AAAAAAAAAAAA").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut x=vec![]; a.by_index_raw(0).unwrap().read_to_end(&mut x).unwrap();
            let mut y=vec![]; a.by_index_raw(1).unwrap().read_to_end(&mut y).unwrap();
            format!("a={:02x?}\n    b={:02x?}\n    identical={}", x, y, x==y)
        });
        t("AES compressed_size too small", || {
            let mut b=vec![]; b.extend(2u16.to_le_bytes()); b.extend(b"AE"); b.push(3); b.extend(0u16.to_le_bytes());
            let mut e=E::new("x",b"short"); e.flags=1; e.method=99; e.crc=Some(0); e.cextra=ext(0x9901,&b); e.lextra=e.cextra.clone();
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            format!("{:?}", a.by_index_decrypt(0,b"pw").map(|_|()).map_err(|e|e.to_string()))
        });
        t("AES get_aes_verification_key_and_salt too small", || {
            let mut b=vec![]; b.extend(2u16.to_le_bytes()); b.extend(b"AE"); b.push(3); b.extend(0u16.to_le_bytes());
            let mut e=E::new("x",b"short"); e.flags=1; e.method=99; e.crc=Some(0); e.cextra=ext(0x9901,&b); e.lextra=e.cextra.clone();
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            format!("{:?}", a.get_aes_verification_key_and_salt(0).map(|_|()).map_err(|e|e.to_string()))
        });
        // AE-2 integrity
        fn ae2(data:&[u8])->(Vec<u8>,u64){
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("x", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_aes_encryption(zip::AesMode::Aes256,"pw")).unwrap();
            w.write_all(data).unwrap();
            let v=w.finish().unwrap().into_inner();
            let cdo=v.windows(4).rposition(|w|w==[0x50,0x4b,1,2]).unwrap() as u64; (v,cdo)
        }
        t("AE-2 compressed_size patched to header-only", || {
            let (mut v,cd)=ae2(b"short"); // <20 bytes => AE-2
            let cd=cd as usize; let csz=16+2+10u32; v[cd+20..cd+24].copy_from_slice(&csz.to_le_bytes());
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut s=vec![]; let r=a.by_index_decrypt(0,b"pw").map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            format!("declared size={} read={r:?} out={:?}", a.by_index_raw(0).unwrap().size(), String::from_utf8_lossy(&s))
        });
        t("AE-2 compressed_size past EOF", || {
            let (mut v,cd)=ae2(b"short");
            let cd=cd as usize; let csz=100000u32; v[cd+20..cd+24].copy_from_slice(&csz.to_le_bytes());
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut s=vec![]; let r=a.by_index_decrypt(0,b"pw").map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            format!("read={r:?} outlen={}", s.len())
        });
        t("AE-2 tampered ciphertext (control)", || {
            let (mut v,_)=ae2(b"short"); 
            let pos=v.windows(1).position(|_|true).unwrap(); let _=pos;
            let ds=30+1+11+16+2; v[ds]^=1;
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut s=vec![]; let r=a.by_index_decrypt(0,b"pw").map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            format!("read={r:?}")
        });
    }
    if on("legacy") {
        for (m,n) in [(1u16,"shrink"),(2,"reduce"),(6,"implode")] {
            t(&format!("{n}: zip64 uncompressed u64::MAX"), || {
                let mut e=E::new("x",&[0u8;400]); e.method=m; e.uncomp=Some(u32::MAX); e.cextra=ext(1,&u64::MAX.to_le_bytes());
                let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
                let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
                format!("{r:?}")
            });
        }
        t("implode: first byte 0xFF (256 code length bytes)", || {
            let mut d=vec![0xffu8]; d.extend([0u8;2000]);
            let mut e=E::new("x",&d); e.method=6; e.flags=4; e.uncomp=Some(10);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            format!("{r:?}")
        });
    }
    if on("lzma") {
        t("lzma dict 0xFFFFFFFF", || {
            let mut d=vec![0x10,0x02,5,0, 0x5d, 0xff,0xff,0xff,0xff]; d.extend([0u8;64]);
            let mut e=E::new("x",&d); e.method=14; e.uncomp=Some(100);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            format!("{r:?}")
        });
        t("has_overlapping_files huge size", || {
            let mut e=E::new("x",b"d"); e.comp=Some(u32::MAX); e.cextra=ext(1,&u64::MAX.to_le_bytes());
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            format!("{:?} size={}", a.has_overlapping_files().map_err(|e|e.to_string()), a.by_index_raw(0).unwrap().compressed_size())
        });
    }
}
