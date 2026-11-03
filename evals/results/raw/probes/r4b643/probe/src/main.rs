#![allow(deprecated, unused)]
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use zip::read::{read_zipfile_from_stream, ZipFile};
use zip::unstable::stream::{ZipStreamFileMetadata, ZipStreamReader, ZipStreamVisitor};
use zip::unstable::write::FileOptionsExt;
use zip::write::{ExtendedFileOptions, FullFileOptions, SimpleFileOptions};
use zip::{AesMode, CompressionMethod, HasZipMetadata, ZipArchive, ZipWriter};

#[derive(Clone, Default)]
struct E {
    name: Vec<u8>,
    method: u16,
    flags: u16,
    crc: u32,
    csize: u32,
    usize_: u32,
    data: Vec<u8>,
    cextra: Vec<u8>,
    comment: Vec<u8>,
    made: u16,
    attr: u32,
}
fn e(name: &str, data: &[u8]) -> E {
    E {
        name: name.as_bytes().to_vec(),
        crc: crc32fast::hash(data),
        csize: data.len() as u32,
        usize_: data.len() as u32,
        data: data.to_vec(),
        made: 3 << 8 | 20,
        ..Default::default()
    }
}
fn build(prefix: &[u8], es: &[E]) -> Vec<u8> {
    let mut o = prefix.to_vec();
    let base = o.len();
    let mut offs = vec![];
    for x in es {
        offs.push((o.len() - base) as u32);
        o.extend(0x04034b50u32.to_le_bytes());
        o.extend(20u16.to_le_bytes());
        o.extend(x.flags.to_le_bytes());
        o.extend(x.method.to_le_bytes());
        o.extend([0x00, 0x60, 0x21, 0x50]);
        o.extend(x.crc.to_le_bytes());
        o.extend(x.csize.to_le_bytes());
        o.extend(x.usize_.to_le_bytes());
        o.extend((x.name.len() as u16).to_le_bytes());
        o.extend(0u16.to_le_bytes());
        o.extend(&x.name);
        o.extend(&x.data);
    }
    let cd = o.len() - base;
    for (i, x) in es.iter().enumerate() {
        o.extend(0x02014b50u32.to_le_bytes());
        o.extend(x.made.to_le_bytes());
        o.extend(20u16.to_le_bytes());
        o.extend(x.flags.to_le_bytes());
        o.extend(x.method.to_le_bytes());
        o.extend([0x00, 0x60, 0x21, 0x50]);
        o.extend(x.crc.to_le_bytes());
        o.extend(x.csize.to_le_bytes());
        o.extend(x.usize_.to_le_bytes());
        o.extend((x.name.len() as u16).to_le_bytes());
        o.extend((x.cextra.len() as u16).to_le_bytes());
        o.extend((x.comment.len() as u16).to_le_bytes());
        o.extend(0u16.to_le_bytes());
        o.extend(0u16.to_le_bytes());
        o.extend(x.attr.to_le_bytes());
        o.extend(offs[i].to_le_bytes());
        o.extend(&x.name);
        o.extend(&x.cextra);
        o.extend(&x.comment);
    }
    let cdlen = o.len() - base - cd;
    o.extend(0x06054b50u32.to_le_bytes());
    o.extend([0u8; 4]);
    o.extend((es.len() as u16).to_le_bytes());
    o.extend((es.len() as u16).to_le_bytes());
    o.extend((cdlen as u32).to_le_bytes());
    o.extend((cd as u32).to_le_bytes());
    o.extend(0u16.to_le_bytes());
    o
}
fn field(id: u16, body: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend((body.len() as u16).to_le_bytes());
    v.extend(body);
    v
}
fn z64(vals: &[u64]) -> Vec<u8> {
    let mut b = vec![];
    for x in vals {
        b.extend(x.to_le_bytes());
    }
    field(1, &b)
}
fn tmp(name: &str) -> PathBuf {
    let p = PathBuf::from(format!("/tmp/probe-{}-{}", std::process::id(), name));
    let _ = fs::create_dir_all(&p);
    p
}
fn open(b: Vec<u8>) -> zip::result::ZipResult<ZipArchive<Cursor<Vec<u8>>>> {
    ZipArchive::new(Cursor::new(b))
}
fn stored() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
fn data(path: &str) -> Vec<u8> {
    fs::read(format!("/work/crate/tests/data/{path}")).unwrap()
}

fn symlink_capacity() {
    let mut x = e("l", b"target");
    x.attr = 0o120777 << 16;
    x.usize_ = u32::MAX;
    x.cextra = z64(&[1u64 << 62]);
    x.crc = crc32fast::hash(b"target");
    let mut a = open(build(&[], &[x])).unwrap();
    println!("opened; declared size={}", a.by_index(0).unwrap().size());
    println!("extract={:?}", a.extract(tmp("cap")));
}
fn overlap_overflow() {
    let mut x = e("a", b"abc");
    x.csize = u32::MAX;
    x.cextra = z64(&[u64::MAX]);
    let mut a = open(build(&[], &[x])).unwrap();
    println!("{:?}", a.has_overlapping_files());
}
fn aes_body(mode: u8) -> Vec<u8> {
    let mut b = 2u16.to_le_bytes().to_vec();
    b.extend(b"AE");
    b.push(mode);
    b.extend(0u16.to_le_bytes());
    field(0x9901, &b)
}
fn aes_underflow() {
    let mut x = e("a", b"tiny");
    x.method = 99;
    x.flags = 1;
    x.cextra = aes_body(3);
    let mut a = open(build(&[], &[x])).unwrap();
    println!("salt={:?}", a.get_aes_verification_key_and_salt(0));
    println!("decrypt={:?}", a.by_index_decrypt(0, b"pw").map(|_| ()));
}
fn aes_arch(pt: &[u8]) -> Vec<u8> {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("a", stored().with_aes_encryption(AesMode::Aes256, "pw")).unwrap();
    z.write_all(pt).unwrap();
    z.finish().unwrap().into_inner()
}
fn aes_empty_auth() {
    let mut b = aes_arch(b"");
    // tamper every byte of the 10-byte auth code at the end of the data
    let a = open(b.clone()).unwrap();
    let cd = a.central_directory_start() as usize;
    for i in cd - 10..cd {
        b[i] ^= 0xff;
    }
    let mut a = open(b).unwrap();
    let mut f = a.by_index_decrypt(0, b"pw").unwrap();
    let mut v = vec![];
    println!("tampered empty read={:?} len={}", f.read_to_end(&mut v), v.len());
    drop(f);
    // control: non-empty tampered
    let mut b = aes_arch(b"hello");
    let a = open(b.clone()).unwrap();
    let cd = a.central_directory_start() as usize;
    b[cd - 1] ^= 0xff;
    let mut a = open(b).unwrap();
    let mut f = a.by_index_decrypt(0, b"pw").unwrap();
    let mut v = vec![];
    println!("control tampered non-empty read={:?}", f.read_to_end(&mut v).map_err(|e| e.to_string()));
}
fn aes_truncate() {
    let b = aes_arch(b"0123456789");
    let a = open(b.clone()).unwrap();
    let cd = a.central_directory_start() as usize;
    let cdlen = b.len() - 22 - cd;
    // relayout: [CD][EOCD][LFH + truncated data]
    let mut cdb = b[cd..cd + cdlen].to_vec();
    let off = (cdlen + 22) as u32;
    cdb[42..46].copy_from_slice(&off.to_le_bytes());
    let mut eocd = b[b.len() - 22..].to_vec();
    eocd[16..20].copy_from_slice(&0u32.to_le_bytes());
    let keep = cd - 10 - 5; // drop auth code and 5 trailing ciphertext bytes
    let mut n = cdb;
    n.extend(eocd);
    n.extend(&b[..keep]);
    let mut a = open(n).unwrap();
    let mut f = a.by_index_decrypt(0, b"pw").unwrap();
    let mut v = vec![];
    let r = f.read_to_end(&mut v);
    println!("truncated read={:?} data={:?}", r.map_err(|e| e.to_string()), String::from_utf8_lossy(&v));
}
fn sfx_nested() {
    let mut inner = ZipWriter::new(Cursor::new(vec![]));
    inner.start_file("inner.txt", stored()).unwrap();
    inner.write_all(b"INNER").unwrap();
    let inner = inner.finish().unwrap().into_inner();
    let mut outer = ZipWriter::new(Cursor::new(vec![]));
    outer.start_file("outer.txt", stored()).unwrap();
    outer.write_all(b"OUTER").unwrap();
    outer.start_file("nested.jar", stored()).unwrap();
    outer.write_all(&inner).unwrap();
    let outer = outer.finish().unwrap().into_inner();
    let mut ctl = open(outer.clone()).unwrap();
    println!("no prefix: {:?}", ctl.file_names().collect::<Vec<_>>());
    let mut p = vec![b'#'; 300];
    p.extend(&outer);
    match open(p) {
        Ok(mut a) => {
            println!("with 300-byte prefix: {:?} offset={}", a.file_names().map(String::from).collect::<Vec<_>>(), a.offset());
            for i in 0..a.len() {
                let mut v = vec![];
                let r = a.by_index(i).and_then(|mut f| Ok(f.read_to_end(&mut v)?));
                println!("  read {i}: {:?} {:?}", r.map_err(|e| e.to_string()), String::from_utf8_lossy(&v));
            }
        }
        Err(er) => println!("with 300-byte prefix: Err {er}"),
    }
}
fn unicode_fields() {
    // comment in cp437 (0x82 = e-acute) with a correct 0x6375 field computed over raw comment bytes.
    let raw = b"caf\x82";
    let mut body = vec![1u8];
    body.extend(crc32fast::hash(raw).to_le_bytes());
    body.extend("café".as_bytes());
    let mut x = e("a", b"x");
    x.comment = raw.to_vec();
    x.cextra = field(0x6375, &body);
    println!("cp437 comment + valid 0x6375: {:?}", open(build(&[], &[x])).map(|a| a.len()));
    // Unicode Path field with stale CRC: spec says ignore the field
    let mut body = vec![1u8];
    body.extend(0xdeadbeefu32.to_le_bytes());
    body.extend("renamed".as_bytes());
    let mut x = e("a", b"x");
    x.cextra = field(0x7075, &body);
    println!("stale 0x7075: {:?}", open(build(&[], &[x])).map(|a| a.len()));
    // NTFS field with a second attribute tag
    let mut b = vec![0u8; 4];
    b.extend(1u16.to_le_bytes());
    b.extend(24u16.to_le_bytes());
    b.extend([0u8; 24]);
    b.extend(2u16.to_le_bytes());
    b.extend(0u16.to_le_bytes());
    let mut x = e("a", b"x");
    x.cextra = field(0x000a, &b);
    println!("NTFS tag1+tag2: {:?}", open(build(&[], &[x])).map(|a| a.len()));
    // Unknown NTFS tag only
    let mut b = vec![0u8; 4];
    b.extend(2u16.to_le_bytes());
    b.extend(24u16.to_le_bytes());
    b.extend([0u8; 24]);
    let mut x = e("a", b"x");
    x.cextra = field(0x000a, &b);
    println!("NTFS tag2 only: {:?}", open(build(&[], &[x])).map(|a| a.len()));
}
#[derive(Default)]
struct Cnt(u32, u32);
impl ZipStreamVisitor for Cnt {
    fn visit_file<R: Read>(&mut self, _f: &mut ZipFile<'_, R>) -> zip::result::ZipResult<()> {
        self.0 += 1;
        Ok(())
    }
    fn visit_additional_metadata(&mut self, m: &ZipStreamFileMetadata) -> zip::result::ZipResult<()> {
        self.1 += 1;
        println!("  meta {} {:?}", m.name(), m.unix_mode());
        Ok(())
    }
}
fn stream_meta() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("a.sh", stored().unix_permissions(0o755)).unwrap();
    z.write_all(b"a").unwrap();
    z.start_file("b.sh", stored().unix_permissions(0o700)).unwrap();
    z.write_all(b"b").unwrap();
    z.add_symlink("l", "a.sh", stored()).unwrap();
    let b = z.finish().unwrap().into_inner();
    let mut c = Cnt::default();
    ZipStreamReader::new(Cursor::new(b.clone())).visit(&mut c).unwrap();
    println!("files={} metadata_calls={}", c.0, c.1);
    let d = tmp("stream");
    ZipStreamReader::new(Cursor::new(b.clone())).extract(&d).unwrap();
    for n in ["a.sh", "b.sh", "l"] {
        let md = fs::symlink_metadata(d.join(n)).unwrap();
        println!("stream {n}: mode {:o} symlink={}", md.permissions().mode(), md.is_symlink());
    }
    let d = tmp("nonstream");
    open(b).unwrap().extract(&d).unwrap();
    for n in ["a.sh", "b.sh", "l"] {
        let md = fs::symlink_metadata(d.join(n)).unwrap();
        println!("archive {n}: mode {:o} symlink={}", md.permissions().mode(), md.is_symlink());
    }
}
fn stream_data_start() {
    let b = build(&[], &[e("a", b"abc")]);
    let mut c = Cursor::new(b);
    let f = read_zipfile_from_stream(&mut c).unwrap().unwrap();
    println!("data_start={}", f.data_start());
}
fn stream_empty() {
    let b = ZipWriter::new(Cursor::new(vec![])).finish().unwrap().into_inner();
    println!("ZipArchive: {:?}", open(b.clone()).map(|a| a.len()));
    println!("stream visit: {:?}", ZipStreamReader::new(Cursor::new(b)).visit(&mut Cnt::default()));
}
fn extract_symlink() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.add_symlink("evil", "/etc/passwd", stored()).unwrap();
    let b = z.finish().unwrap().into_inner();
    let d = tmp("sym");
    println!("extract: {:?}", open(b.clone()).unwrap().extract(&d));
    println!("created -> {:?}, readable bytes through link: {:?}", fs::read_link(d.join("evil")), fs::read(d.join("evil")).map(|v| v.len()));
    println!("re-extract over existing: {:?}", open(b).unwrap().extract(&d));

    // in-tree relative link then file through it
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.add_directory("real", stored()).unwrap();
    z.add_symlink("link", "real", stored()).unwrap();
    z.start_file("link/f", stored()).unwrap();
    z.write_all(b"x").unwrap();
    let b = z.finish().unwrap().into_inner();
    println!("in-tree link follow: {:?}", open(b).unwrap().extract(tmp("sym2")));

    // simple in-tree relative symlink, extract twice
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("t", stored()).unwrap();
    z.write_all(b"x").unwrap();
    z.add_symlink("link", "t", stored()).unwrap();
    let b = z.finish().unwrap().into_inner();
    let d = tmp("sym3");
    println!("first: {:?}", open(b.clone()).unwrap().extract(&d));
    println!("second: {:?}", open(b).unwrap().extract(&d));
}
fn modes() {
    let mut x = e("suid", b"#!/bin/sh\n");
    x.attr = 0o106755 << 16;
    let mut y = e("dosattr", b"x");
    y.attr = 0x20; // archive bit only, host=unix
    let d = tmp("modes");
    println!("{:?}", open(build(&[], &[x, y])).unwrap().extract(&d));
    for n in ["suid", "dosattr"] {
        println!("{n}: {:o}", fs::metadata(d.join(n)).unwrap().permissions().mode());
    }
    let mut x = e("ro", b"x");
    x.made = 20;
    x.attr = 1;
    let mut w = e("rw", b"x");
    w.made = 20;
    w.attr = 0x20;
    let mut a = open(build(&[], &[x, w])).unwrap();
    println!("dos ro mode {:o}", a.by_name("ro").unwrap().unix_mode().unwrap());
    println!("dos rw mode {:o}", a.by_name("rw").unwrap().unix_mode().unwrap());
}
fn finish_readable() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("big", stored().large_file(true)).unwrap();
    z.write_all(b"hello").unwrap();
    let mut a = z.finish_into_readable().unwrap();
    {
        let mut f = a.by_index(0).unwrap();
        println!("large_file: size={} csize={}", f.size(), f.compressed_size());
        let mut v = vec![];
        let r = f.read_to_end(&mut v);
        println!("  read={:?} len={}", r.map_err(|e| e.to_string()), v.len());
    }
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("aes", SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).with_aes_encryption(AesMode::Aes256, "pw")).unwrap();
    z.write_all(b"hello").unwrap();
    let mut a = z.finish_into_readable().unwrap();
    let r = a.by_index_decrypt(0, b"pw").map(|f| f.compression());
    println!("aes via finish_into_readable: {:?}", r);
    let b = a.into_inner().into_inner();
    let mut a = open(b).unwrap();
    println!("aes via ZipArchive::new: {:?}", a.by_index_decrypt(0, b"pw").map(|f| f.compression()));
}
fn raw_copy() {
    // zipcrypto
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("c", stored().with_deprecated_encryption(b"pw")).unwrap();
    z.write_all(b"secret secret").unwrap();
    let b = z.finish().unwrap().into_inner();
    let mut a = open(b).unwrap();
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut o = open(d.finish().unwrap().into_inner()).unwrap();
    println!("zipcrypto raw copy: encrypted={}", o.by_index_raw(0).unwrap().encrypted());
    let mut v = vec![];
    let r = o.by_index(0).map(|mut f| f.read_to_end(&mut v));
    println!("  by_index without password: {:?} {:?}", r.map(|x| x.map_err(|e| e.to_string())).map_err(|e| e.to_string()), String::from_utf8_lossy(&v));
    println!("  by_index_decrypt: {:?}", o.by_index_decrypt(0, b"pw").map(|_| ()));

    // symlink + comment + extra
    let mut x = e("l", b"tgt");
    x.attr = 0o120777 << 16;
    x.comment = b"keep me".to_vec();
    let mut ts = vec![1u8];
    ts.extend(1234567u32.to_le_bytes());
    x.cextra = field(0x5455, &ts);
    let mut a = open(build(&[], &[x])).unwrap();
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    { let f = a.by_index_raw(0).unwrap(); println!("source symlink={} comment={:?}", f.is_symlink(), f.comment()); }
    d.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut o = open(d.finish().unwrap().into_inner()).unwrap();
    let f = o.by_index_raw(0).unwrap();
    println!("dest symlink={} comment={:?} extra_fields={} mode={:?}", f.is_symlink(), f.comment(), f.extra_data_fields().count(), f.unix_mode().map(|m| format!("{m:o}")));
    drop(f);

    // zip64-declared size
    let body = b"\x03\x00"; // empty deflate
    let mut x = e("big", body);
    x.method = 8;
    x.usize_ = u32::MAX;
    x.cextra = z64(&[5u64 << 30]);
    let mut a = open(build(&[], &[x])).unwrap();
    println!("source size={}", a.by_index_raw(0).unwrap().size());
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
    let mut o = open(d.finish().unwrap().into_inner()).unwrap();
    let f = o.by_index_raw(0).unwrap();
    println!("dest size={} csize={}", f.size(), f.compressed_size());
}
fn long_name() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    let n = "a".repeat(70000);
    println!("start_file: {:?}", z.start_file(n, stored()));
    std::mem::forget(z);
}
fn long_name_drop() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    let n = "a".repeat(70000);
    println!("start_file: {:?}", z.start_file(n, stored()));
}
fn comment_expand() {
    let mut x = e("a", b"x");
    x.comment = vec![0xE0; 40000];
    let b = build(&[], &[x]);
    let a = open(b.clone()).unwrap();
    println!("opened");
    let mut w = ZipWriter::new_append(Cursor::new(b)).unwrap();
    println!("append opened");
    let r = w.finish();
    println!("finish: {:?}", r.map(|_| ()));
}
fn append_zip64() {
    let p = tmp("z64").join("a.zip");
    let mut f = fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&p).unwrap();
    // stored junk hole of 5 GiB before the first entry, as a real entry would be
    f.set_len(5 << 30).unwrap();
    f.seek(SeekFrom::Start(5 << 30)).unwrap();
    let mut z = ZipWriter::new(f);
    z.start_file("late", stored()).unwrap();
    z.write_all(b"LATE").unwrap();
    let f = z.finish().unwrap();
    let mut a = ZipArchive::new(f).unwrap();
    println!("before: header_start={} ", a.by_index(0).unwrap().header_start());
    let mut s = String::new();
    a.by_index(0).unwrap().read_to_string(&mut s).unwrap();
    println!("before read={s}");
    let f = a.into_inner();
    let mut w = ZipWriter::new_append(f).unwrap();
    w.start_file("added", stored()).unwrap();
    w.write_all(b"ADDED").unwrap();
    let f = w.finish().unwrap();
    let mut a = ZipArchive::new(f).unwrap();
    let hs = a.by_name_seek("late").map(|f| f.get_metadata().header_start);
    println!("after append: names={:?} header_start={:?}", a.file_names().map(String::from).collect::<Vec<_>>(), hs.map_err(|e| e.to_string()));
    let mut s = String::new();
    let r = a.by_name("late").and_then(|mut f| Ok(f.read_to_string(&mut s)?));
    println!("after read: {:?} {s}", r.map_err(|e| e.to_string()));
    let _ = fs::remove_file(&p);
}
fn append_metadata() {
    // implode flags
    let b = data("legacy/implode.zip");
    let mut a = open(b.clone()).unwrap();
    let mut s = vec![];
    a.by_name("first.txt").unwrap().read_to_end(&mut s).unwrap();
    println!("implode control ok len={} flags={:#x}", s.len(), a.by_name("first.txt").unwrap().get_metadata().flags);
    let mut w = ZipWriter::new_append(Cursor::new(b.clone())).unwrap();
    w.start_file("new", stored()).unwrap();
    w.write_all(b"n").unwrap();
    let c = w.finish().unwrap().into_inner();
    let mut a = open(c).unwrap();
    let mut v = vec![];
    let r = a.by_name("first.txt").and_then(|mut f| Ok(f.read_to_end(&mut v)?));
    println!("implode after append: {:?} flags={:#x} equal={}", r.map_err(|e| e.to_string()), a.by_name("first.txt").map(|f| f.get_metadata().flags).unwrap_or(0), v == s);
    // raw copy
    let mut a = open(b).unwrap();
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(a.index_for_name("first.txt").unwrap()).unwrap()).unwrap();
    let mut o = open(d.finish().unwrap().into_inner()).unwrap();
    let mut v = vec![];
    let r = o.by_name("first.txt").and_then(|mut f| Ok(f.read_to_end(&mut v)?));
    println!("implode raw copy: {:?} equal={}", r.map_err(|e| e.to_string()), v == s);

    // comment encodings, host, date
    let mut x = e("ascii", b"x");
    x.flags = 1 << 11;
    x.comment = "caf\u{e9}".as_bytes().to_vec();
    let mut y = e("cp437", b"y");
    y.comment = b"caf\x82".to_vec();
    let mut q = e("darwin", b"z");
    q.made = 19 << 8 | 20;
    let b = build(&[], &[x, y, q]);
    let mut a = open(b.clone()).unwrap();
    for i in 0..3 {
        let f = a.by_index_raw(i).unwrap();
        println!("before {} comment={:?} system={:?}", f.name(), f.comment(), f.get_metadata().system);
    }
    let w = ZipWriter::new_append(Cursor::new(b.clone())).unwrap();
    let c = w.finish().unwrap().into_inner();
    println!("no-op append size {} -> {}", b.len(), c.len());
    let mut a = open(c.clone()).unwrap();
    for i in 0..3 {
        let f = a.by_index_raw(i).unwrap();
        println!("after  {} comment={:?} system={:?}", f.name(), f.comment(), f.get_metadata().system);
    }
    let mut hdr = 0;
    println!("host byte of darwin entry after = {}", {
        let pos = c.windows(6).rposition(|w| w == b"darwin").unwrap();
        c[pos - 46 + 5]
    });
}
fn stream_writer() {
    use zip::write::StreamWriter;
    // ZipCrypto via streaming writer
    let mut buf = vec![];
    {
        let mut z = ZipWriter::new_stream(&mut buf);
        z.start_file("c", stored().with_deprecated_encryption(b"pw")).unwrap();
        z.write_all(b"secret").unwrap();
        z.finish().unwrap();
    }
    let mut a = open(buf).unwrap();
    println!("stream zipcrypto correct password: {:?}", a.by_index_decrypt(0, b"pw").map(|_| ()));

    // AES via streaming writer
    let mut buf = vec![];
    {
        let mut z = ZipWriter::new_stream(&mut buf);
        z.start_file("c", stored().with_aes_encryption(AesMode::Aes256, "pw")).unwrap();
        z.write_all(b"secret").unwrap();
        println!("stream aes finish: {:?}", z.finish().map(|_| ()).map_err(|e| e.to_string()));
    }

    // directory
    let mut buf = vec![];
    {
        let mut z = ZipWriter::new_stream(&mut buf);
        z.add_directory("d", stored()).unwrap();
        z.start_file("f", stored()).unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
    }
    let flags = u16::from_le_bytes([buf[6], buf[7]]);
    let next = &buf[32..36];
    println!("stream dir flags={flags:#x} bytes after dir header={:02x?} (PK0304={})", next, next == b"PK\x03\x04");

    // raw copy
    let src = build(&[], &[e("a", b"abc"), e("b", b"def")]);
    let mut a = open(src).unwrap();
    let mut buf = vec![];
    {
        let mut z = ZipWriter::new_stream(&mut buf);
        z.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
        z.raw_copy_file(a.by_index_raw(1).unwrap()).unwrap();
        z.finish().unwrap();
    }
    let flags = u16::from_le_bytes([buf[6], buf[7]]);
    let nl = u16::from_le_bytes([buf[26], buf[27]]) as usize;
    let el = u16::from_le_bytes([buf[28], buf[29]]) as usize;
    let p = 30 + nl + el + 3;
    println!("stream raw copy flags={flags:#x} csize_in_lfh={:?} next={:02x?}", &buf[18..22], &buf[p..p + 4]);
    let mut o = open(buf).unwrap();
    println!("  central read ok: {:?}", o.by_index(1).map(|mut f| {let mut s=String::new(); f.read_to_string(&mut s).unwrap(); s}));
}
struct Sink(u64);
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0 += b.len() as u64;
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn stream_large() {
    let mut z = ZipWriter::new_stream(Sink(0));
    z.start_file("big", stored().large_file(true)).unwrap();
    let chunk = vec![0u8; 1 << 20];
    for _ in 0..4097 {
        z.write_all(&chunk).unwrap();
    }
    println!("finish large_file(true) stream: {:?}", z.finish().map(|_| ()).map_err(|e| e.to_string()));
}
fn zipcrypto_det() {
    let run = || {
        let mut z = ZipWriter::new(Cursor::new(vec![]));
        z.start_file("c", stored().with_deprecated_encryption(b"pw").last_modified_time(zip::DateTime::default())).unwrap();
        z.write_all(b"same plaintext").unwrap();
        z.finish().unwrap().into_inner()
    };
    let (a, b) = (run(), run());
    println!("identical archives: {} first 12 ciphertext bytes {:02x?}", a == b, &a[31..43]);
}
fn seek_encrypted() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("c", stored().with_deprecated_encryption(b"pw")).unwrap();
    z.write_all(b"secret").unwrap();
    z.start_file("a", stored().with_aes_encryption(AesMode::Aes256, "pw")).unwrap();
    z.write_all(b"secret").unwrap();
    let mut a = open(z.finish().unwrap().into_inner()).unwrap();
    println!("by_name: {:?}", a.by_name("c").map(|_| ()).map_err(|e| e.to_string()));
    for n in ["c", "a"] {
        match a.by_name_seek(n) {
            Ok(mut f) => {
                let mut v = vec![];
                f.read_to_end(&mut v).unwrap();
                println!("by_name_seek {n}: Ok len={} bytes={:02x?}", v.len(), &v[..8]);
            }
            Err(er) => println!("by_name_seek {n}: Err {er}"),
        }
    }
}
fn symlink_path() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("c", stored()).unwrap();
    z.write_all(b"C").unwrap();
    z.add_directory("a", stored()).unwrap();
    z.add_symlink_from_path("a/b", "../c", stored()).unwrap();
    let mut a = open(z.finish().unwrap().into_inner()).unwrap();
    let mut s = String::new();
    a.by_name("a/b").unwrap().read_to_string(&mut s).unwrap();
    println!("stored target for a/b -> ../c: {s:?}");
}
fn dir_dot() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.add_directory_from_path(".", stored()).unwrap();
    z.start_file("f", stored()).unwrap();
    z.write_all(b"x").unwrap();
    let mut a = open(z.finish().unwrap().into_inner()).unwrap();
    println!("names {:?}", a.file_names().collect::<Vec<_>>());
    println!("extract {:?}", a.extract(tmp("dot")));
}
fn merge_dup() {
    let mk = |n: &str, c: &[u8]| {
        let mut z = ZipWriter::new(Cursor::new(vec![]));
        z.start_file(n, stored()).unwrap();
        z.write_all(c).unwrap();
        open(z.finish().unwrap().into_inner()).unwrap()
    };
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("x", stored()).unwrap();
    z.write_all(b"first").unwrap();
    println!("merge dup: {:?}", z.merge_archive(mk("x", b"second")).map_err(|e| e.to_string()));
    let mut a = open(z.finish().unwrap().into_inner()).unwrap();
    let mut s = String::new();
    a.by_name("x").unwrap().read_to_string(&mut s).unwrap();
    println!("names={:?} x={s}", a.file_names().collect::<Vec<_>>());
}
fn root_dbg() {
    let b = build(&[], &[e("a/x", b"1"), e("a/../b", b"2")]);
    let mut a = open(b).unwrap();
    println!("root_dir={:?}", a.root_dir(zip::read::root_dir_common_filter));
    let d = tmp("root");
    println!("{:?}", a.extract_unwrapped_root_dir(&d, zip::read::root_dir_common_filter));
    println!("{:?}", fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>());
}
fn legacy_reserve(method: u16) {
    let mut x = e("a", &[0u8; 64]);
    x.method = method;
    x.usize_ = u32::MAX;
    x.cextra = z64(&[1u64 << 62]);
    let mut a = open(build(&[], &[x])).unwrap();
    let mut f = a.by_index(0).unwrap();
    let mut b = [0u8; 16];
    println!("read: {:?}", f.read(&mut b).map_err(|e| e.to_string()));
}
fn bits(codes: impl Iterator<Item = u16>, width: u32) -> Vec<u8> {
    let mut out = vec![];
    let (mut acc, mut n) = (0u64, 0u32);
    for c in codes {
        acc |= (c as u64) << n;
        n += width;
        while n >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            n -= 8;
        }
    }
    if n > 0 {
        out.push(acc as u8);
    }
    out
}
fn shrink_recursion(n: usize) {
    let codes = std::iter::once(65u16).chain((0..n).flat_map(|_| [256u16, 2u16]));
    let data = bits(codes, 9);
    println!("compressed {} bytes", data.len());
    let mut x = e("a", &data);
    x.method = 1;
    x.usize_ = 100;
    let mut a = open(build(&[], &[x])).unwrap();
    let mut f = a.by_index(0).unwrap();
    let mut v = vec![];
    println!("read: {:?}", f.read_to_end(&mut v).map_err(|e| e.to_string()));
}
fn implode_ff() {
    let mut d = vec![0xffu8];
    d.extend([0u8; 512]);
    let mut x = e("a", &d);
    x.method = 6;
    x.usize_ = 10;
    let mut a = open(build(&[], &[x])).unwrap();
    let mut f = a.by_index(0).unwrap();
    let mut v = vec![];
    println!("read: {:?}", f.read_to_end(&mut v).map_err(|e| e.to_string()));
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}
static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
fn hook() {
    std::panic::set_hook(Box::new(|i| {
        *LAST.lock().unwrap() = format!("{} @ {}", i.payload().downcast_ref::<String>().cloned().or(i.payload().downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(), i.location().map(|l| l.to_string()).unwrap_or_default());
    }));
}
fn try_read(b: Vec<u8>, name: Option<&str>, limit: u64) -> Result<(), String> {
    let r = std::panic::catch_unwind(move || {
        if let Ok(mut a) = open(b) {
            for i in 0..a.len() {
                if let Ok(mut f) = a.by_index(i) {
                    let mut v = vec![];
                    let _ = std::io::Read::take(&mut f, limit).read_to_end(&mut v);
                }
            }
        }
    });
    r.map_err(|_| LAST.lock().unwrap().clone())
}
fn fuzz_shrink(iters: u64) {
    use std::collections::BTreeMap;
    hook();
    let mut seen: BTreeMap<String, u64> = BTreeMap::new();
    let mut r = Rng(0x1234567);
    for it in 0..iters {
        // generate plausible code stream: literals, recent codes, partial clears
        let mut codes = vec![(r.below(256)) as u16];
        let mut next = 257u16;
        let n = 20 + r.below(3000);
        for _ in 0..n {
            match r.below(100) {
                0..=39 => codes.push(r.below(256) as u16),
                40..=84 => codes.push(257 + r.below((next - 256).min(511) as u64) as u16),
                85..=89 => {
                    codes.push(256);
                    codes.push(2);
                    next = 257;
                }
                _ => {
                    codes.push(257 + r.below(254) as u16);
                }
            }
            if next < 511 {
                next += 1;
            }
        }
        let d = bits(codes.into_iter(), 9);
        let mut x = e("a", &d);
        x.method = 1;
        x.usize_ = 200000;
        if let Err(m) = try_read(build(&[], &[x]), None, u64::MAX) {
            *seen.entry(m).or_default() += 1;
        }
    }
    let _ = std::panic::take_hook();
    println!("shrink fuzz {iters} iters: {seen:#?}");
}
fn fuzz_mut(path: &str, iters: u64) {
    use std::collections::BTreeMap;
    hook();
    let orig = data(path);
    let a = open(orig.clone()).unwrap();
    let cd = a.central_directory_start() as usize;
    drop(a);
    let mut seen: BTreeMap<String, (u64, usize)> = BTreeMap::new();
    let mut r = Rng(0xabcdef + orig.len() as u64);
    for it in 0..iters {
        let mut b = orig.clone();
        let flips = 1 + r.below(8);
        let region = if it % 4 == 0 { b.len() } else { cd };
        for _ in 0..flips {
            let i = r.below(region as u64) as usize;
            match r.below(3) {
                0 => b[i] ^= 1 << r.below(8),
                1 => b[i] = r.next() as u8,
                _ => b[i] = [0u8, 0xff, 0x7f, 0x80][r.below(4) as usize],
            }
        }
        if let Err(m) = try_read(b, None, 1 << 24) {
            let en = seen.entry(m).or_default();
            en.0 += 1;
        }
    }
    let _ = std::panic::take_hook();
    println!("fuzz {path} {iters}: {seen:#?}");
}
fn compress_levels() {
    for (m, l) in [(CompressionMethod::Deflated, 0), (CompressionMethod::Bzip2, 0)] {
        let mut z = ZipWriter::new(Cursor::new(vec![]));
        let r = z.start_file("a", SimpleFileOptions::default().compression_method(m).compression_level(Some(l)));
        println!("{m:?} level {l}: {:?}", r.map_err(|e| e.to_string()));
    }
}
fn extra_data() {
    let mut o: FullFileOptions = FullFileOptions::default().compression_method(CompressionMethod::Stored);
    println!("add zip64 id: {:?}", o.add_extra_data(1, vec![0u8; 16].into_boxed_slice(), false).map_err(|e| e.to_string()));
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    println!("start_file after rejected add: {:?}", z.start_file("a", o.clone()).map_err(|e| e.to_string()));
    z.write_all(b"x").ok();
    let b = z.finish().unwrap().into_inner();
    println!("local extra len {}", u16::from_le_bytes([b[28], b[29]]));
    println!("add after rejected: {:?}", o.add_extra_data(0xbeef, vec![1].into_boxed_slice(), false).map_err(|e| e.to_string()));

    let mut o: FullFileOptions = FullFileOptions::default();
    let mut body = vec![1u8];
    body.extend(crc32fast::hash(b"name").to_le_bytes());
    body.extend("näme".as_bytes());
    println!("valid 0x7075 for 'name': {:?}", o.add_extra_data(0x7075, body.into_boxed_slice(), false).map_err(|e| e.to_string()));
}
fn lzma_dict() {
    let mut d = vec![0x09, 0x14, 0x05, 0x00, 0x5d];
    d.extend(0xFFFF_FFFFu32.to_le_bytes());
    d.extend([0u8; 32]);
    let mut x = e("a", &d);
    x.method = 14;
    x.usize_ = 100;
    let mut a = open(build(&[], &[x])).unwrap();
    let mut f = a.by_index(0).unwrap();
    let mut v = vec![0u8; 16];
    println!("read: {:?}", f.read(&mut v).map_err(|e| e.to_string()));
}
fn stream_drop() {
    // malformed LZMA then good entry; drop must skip to next entry in stream mode
    let mut d = vec![0x09, 0x14, 0x07, 0x00];
    d.extend([0u8; 16]);
    let mut x = e("bad", &d);
    x.method = 14;
    let b = build(&[], &[x, e("good", b"ok")]);
    let mut c = Cursor::new(b);
    {
        let mut f = read_zipfile_from_stream(&mut c).unwrap().unwrap();
        let mut v = vec![];
        println!("first read: {:?}", f.read_to_end(&mut v).map_err(|e| e.to_string()));
    }
    println!("pos after drop {}", c.position());
    println!("next: {:?}", read_zipfile_from_stream(&mut c).map(|o| o.map(|f| f.name().to_string())).map_err(|e| e.to_string()));
}

fn deep_copy_large() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("a", stored().large_file(true)).unwrap();
    z.write_all(b"hello").unwrap();
    z.start_file("b", stored()).unwrap();
    z.write_all(b"world, with more bytes").unwrap();
    println!("deep_copy: {:?}", z.deep_copy_file("a", "c").map_err(|e| e.to_string()));
    let mut o = open(z.finish().unwrap().into_inner()).unwrap();
    for n in ["a", "b", "c"] {
        let mut v = vec![];
        let r = o.by_name(n).and_then(|mut f| Ok(f.read_to_end(&mut v)?));
        let cs = o.by_name_seek(n).map(|f| f.get_metadata().compressed_size);
        println!("{n}: {:?} {:?} csize={:?}", r.map_err(|e| e.to_string()), String::from_utf8_lossy(&v), cs.map_err(|e| e.to_string()));
    }
    // control without large_file
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("a", stored()).unwrap();
    z.write_all(b"hello").unwrap();
    z.start_file("b", stored()).unwrap();
    z.write_all(b"world").unwrap();
    z.deep_copy_file("a", "c").unwrap();
    let mut o = open(z.finish().unwrap().into_inner()).unwrap();
    let mut s = String::new();
    o.by_name("c").unwrap().read_to_string(&mut s).unwrap();
    println!("control c={s}");
}
fn read_exact_crc() {
    let mut x = e("a", b"hello");
    x.crc ^= 1;
    let mut a = open(build(&[], &[x])).unwrap();
    let mut b = vec![0u8; 5];
    println!("read_exact bad crc: {:?}", a.by_index(0).unwrap().read_exact(&mut b).map_err(|e| e.to_string()));
    let mut v = vec![];
    println!("read_to_end bad crc: {:?}", a.by_index(0).unwrap().read_to_end(&mut v).map_err(|e| e.to_string()));
}
fn size_lie() {
    let mut x = e("a", &vec![b'A'; 100000]);
    x.usize_ = 3;
    let mut a = open(build(&[], &[x])).unwrap();
    let mut f = a.by_index(0).unwrap();
    println!("declared size={}", f.size());
    let mut v = vec![];
    println!("read: {:?} actual={}", f.read_to_end(&mut v).map_err(|e| e.to_string()), v.len());
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("bomb", SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).unwrap();
    z.write_all(&vec![0u8; 10_000_000]).unwrap();
    let b = z.finish().unwrap().into_inner();
    // patch uncompressed size in central directory to 1
    let cd = open(b.clone()).unwrap().central_directory_start() as usize;
    let mut b2 = b;
    b2[cd + 24..cd + 28].copy_from_slice(&1u32.to_le_bytes());
    let mut a = open(b2).unwrap();
    let mut f = a.by_index(0).unwrap();
    let mut sink = Sink(0);
    println!("deflate size()={} copied={:?}", f.size(), std::io::copy(&mut f, &mut sink).map_err(|e| e.to_string()));
}
fn dir_mode_doc() {
    let mut bad = e("sub/dir/x", b"abc");
    bad.crc ^= 1;
    let d = tmp("dmode");
    println!("extract: {:?}", open(build(&[], &[bad])).unwrap().extract(&d));
    let m = fs::metadata(d.join("sub/dir")).unwrap().permissions().mode();
    println!("partially-extracted dir mode {m:o}");
}
fn stream_drop2() {
    let mut d = vec![0x09, 0x14, 0x07, 0x00];
    d.extend(vec![0x41u8; 40000]);
    let mut x = e("bad", &d);
    x.method = 14;
    let b = build(&[], &[x, e("good", b"ok")]);
    let mut c = Cursor::new(b);
    {
        let mut f = read_zipfile_from_stream(&mut c).unwrap().unwrap();
        let mut v = vec![];
        println!("first read: {:?}", f.read_to_end(&mut v).map_err(|e| e.to_string()));
    }
    println!("pos after drop {} (next entry at {})", c.position(), 30 + 3 + 40004);
    println!("next: {:?}", read_zipfile_from_stream(&mut c).map(|o| o.map(|f| f.name().to_string())).map_err(|e| e.to_string()));
}

fn shrink_run(codes: &[u16], widths9: bool) -> Result<String, String> {
    let d = bits(codes.iter().copied(), 9);
    let mut x = e("a", &d);
    x.method = 1;
    x.usize_ = 100000;
    let b = build(&[], &[x]);
    std::panic::catch_unwind(move || {
        let mut a = open(b).unwrap();
        let mut f = a.by_index(0).unwrap();
        let mut v = vec![];
        match f.read_to_end(&mut v) {
            Ok(_) => "ok".to_string(),
            Err(er) => er.to_string(),
        }
    })
    .map_err(|_| LAST.lock().unwrap().clone())
}
fn shrink_search(seqs: u64, steps: u64) {
    use std::collections::BTreeMap;
    hook();
    let mut r = Rng(0x9e3779b97f4a7c15);
    let mut seen: BTreeMap<String, (u64, Vec<u16>)> = BTreeMap::new();
    let mut errs: BTreeMap<String, u64> = BTreeMap::new();
    for s in 0..seqs {
        let mut codes = vec![r.below(256) as u16];
        let clearp = 2 + r.below(20);
        let span = 8 + r.below(120);
        'outer: for _ in 0..steps {
            let mut ok = false;
            for _try in 0..6 {
                let mut c = codes.clone();
                let p = r.below(100);
                if p < clearp {
                    c.push(256);
                    c.push(2);
                } else if p < clearp + 30 {
                    c.push(r.below(4) as u16 + 65);
                } else {
                    c.push(257 + r.below(span) as u16);
                }
                match shrink_run(&c, true) {
                    Err(m) => {
                        seen.entry(m).or_insert((0, c.clone())).0 += 1;
                        continue;
                    }
                    Ok(m) if m.contains("unexpected end") || m.contains("i/o") || m == "ok" => {
                        codes = c;
                        ok = true;
                        break;
                    }
                    Ok(m) => {
                        *errs.entry(m).or_default() += 1;
                    }
                }
            }
            if !ok {
                break 'outer;
            }
        }
    }
    let _ = std::panic::take_hook();
    for (k, v) in &seen {
        println!("PANIC {k} x{} seq_len={} seq={:?}", v.0, v.1.len(), &v.1);
    }
    println!("errs={errs:?}");
}

fn quad(cd_entries: usize, eocds: usize) {
    let mut o = vec![];
    for i in 0..cd_entries {
        o.extend(0x02014b50u32.to_le_bytes());
        o.extend(20u16.to_le_bytes());
        o.extend(20u16.to_le_bytes());
        o.extend([0u8; 38]);
    }
    let cdlen = o.len();
    for _ in 0..eocds {
        o.extend(0x06054b50u32.to_le_bytes());
        o.extend([0u8; 4]);
        o.extend(0xfffeu16.to_le_bytes());
        o.extend(0xfffeu16.to_le_bytes());
        o.extend((cdlen as u32).to_le_bytes());
        o.extend(0u32.to_le_bytes());
        o.extend(0u16.to_le_bytes());
    }
    let n = o.len();
    let t = std::time::Instant::now();
    let r = open(o).map(|a| a.len()).map_err(|e| e.to_string());
    println!("file {} bytes, cd_entries={cd_entries} eocds={eocds}: {:?} in {:?}", n, r, t.elapsed());
}
fn deep_after_append() {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("a", stored().large_file(true)).unwrap();
    z.write_all(b"hello").unwrap();
    let b = z.finish().unwrap().into_inner();
    let mut w = ZipWriter::new_append(Cursor::new(b)).unwrap();
    w.deep_copy_file("a", "c").unwrap();
    let b = w.finish().unwrap().into_inner();
    let mut a = open(b.clone()).unwrap();
    let mut s = String::new();
    println!("central read c: {:?}", a.by_name("c").and_then(|mut f| Ok(f.read_to_string(&mut s)?)).map_err(|e| e.to_string()));
    let hs = a.by_name("c").unwrap().header_start() as usize;
    println!("lfh csize={:02x?} usize={:02x?} extra_len={}", &b[hs + 18..hs + 22], &b[hs + 22..hs + 26], u16::from_le_bytes([b[hs + 28], b[hs + 29]]));
}
fn deflate64_time() {
    hook();
    let orig = data("deflate64.zip");
    let a = open(orig.clone()).unwrap();
    let cd = a.central_directory_start() as usize;
    drop(a);
    println!("size {} cd {cd}", orig.len());
    let mut r = Rng(0xabcdef + orig.len() as u64);
    let mut worst = std::time::Duration::ZERO;
    for it in 0..40u64 {
        let mut b = orig.clone();
        for _ in 0..4 {
            let i = r.below(cd as u64) as usize;
            b[i] ^= 1 << r.below(8);
        }
        let t = std::time::Instant::now();
        let x = try_read(b, None, 1 << 24);
        let el = t.elapsed();
        if el > worst { worst = el; }
        if x.is_err() { println!("{x:?}"); }
    }
    println!("worst iter {worst:?}");
}

#[derive(Clone, Debug)]
struct M { name: String, content: Vec<u8>, pw: u8, align: u16, sym: bool }
fn write_model(seed: u64, trace: &mut Vec<String>) -> Result<(), String> {
    let mut r = Rng(seed | 1);
    for _ in 0..5 { r.next(); }
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    let mut model: Vec<M> = vec![];
    let mut cnt = 0;
    let methods = [CompressionMethod::Stored, CompressionMethod::Deflated, CompressionMethod::Bzip2, CompressionMethod::Zstd, CompressionMethod::Xz, CompressionMethod::Ppmd];
    let ops = 2 + r.below(12);
    for _ in 0..ops {
        match r.below(12) {
            0..=4 => {
                cnt += 1;
                let name = format!("f{cnt}");
                let m = methods[r.below(6) as usize];
                let large = r.below(4) == 0 && std::env::var("NOLARGE").is_err();
                let align = [1u16, 2, 4, 512, 4096][r.below(5) as usize];
                let pw = r.below(4);
                let len = r.below(3000) as usize;
                let content: Vec<u8> = (0..len).map(|i| ((i as u64 * (seed % 7 + 1)) % 251) as u8).collect();
                let mut o = SimpleFileOptions::default().compression_method(m).large_file(large).with_alignment(align);
                trace.push(format!("start {name} {m:?} large={large} align={align} pw={pw} len={len}"));
                let (res, pwv) = match pw {
                    1 => (z.start_file(name.clone(), o.with_aes_encryption(AesMode::Aes128, "pw")), 1),
                    2 => (z.start_file(name.clone(), o.with_deprecated_encryption(b"pw")), 2),
                    _ => (z.start_file(name.clone(), o), 0),
                };
                res.map_err(|e| format!("start_file: {e}"))?;
                z.write_all(&content).map_err(|e| format!("write: {e}"))?;
                model.push(M { name, content, pw: pwv, align, sym: false });
            }
            5 => {
                if z.is_writing_file() || !model.is_empty() {
                    trace.push("abort".into());
                    if z.abort_file().is_ok() { model.pop(); }
                }
            }
            6 => {
                if let Some(src) = model.get(r.below(model.len().max(1) as u64) as usize).cloned() {
                    cnt += 1;
                    let name = format!("s{cnt}");
                    trace.push(format!("shallow {} -> {name}", src.name));
                    z.shallow_copy_file(&src.name, &name).map_err(|e| format!("shallow: {e}"))?;
                    model.push(M { name, align: 1, ..src });
                }
            }
            7 => {
                if let Some(src) = model.get(r.below(model.len().max(1) as u64) as usize).cloned() {
                    cnt += 1;
                    let name = format!("d{cnt}");
                    trace.push(format!("deep {} -> {name}", src.name));
                    z.deep_copy_file(&src.name, &name).map_err(|e| format!("deep: {e}"))?;
                    model.push(M { name, align: 1, ..src });
                }
            }
            8 => {
                cnt += 1;
                let name = format!("dir{cnt}");
                trace.push(format!("dir {name}"));
                z.add_directory(name.clone(), SimpleFileOptions::default()).map_err(|e| format!("dir: {e}"))?;
                model.push(M { name: name + "/", content: vec![], pw: 0, align: 1, sym: false });
            }
            9 => {
                cnt += 1;
                let name = format!("l{cnt}");
                trace.push(format!("symlink {name}"));
                z.add_symlink(name.clone(), "target", SimpleFileOptions::default()).map_err(|e| format!("sym: {e}"))?;
                model.push(M { name, content: b"target".to_vec(), pw: 0, align: 1, sym: true });
            }
            10 => {
                cnt += 1;
                let name = format!("m{cnt}");
                let mut s = ZipWriter::new(Cursor::new(vec![]));
                s.start_file(name.clone(), SimpleFileOptions::default()).unwrap();
                s.write_all(b"merged").unwrap();
                let src = open(s.finish().unwrap().into_inner()).unwrap();
                trace.push(format!("merge {name}"));
                z.merge_archive(src).map_err(|e| format!("merge: {e}"))?;
                model.push(M { name, content: b"merged".to_vec(), pw: 0, align: 1, sym: false });
            }
            _ => {
                trace.push("reopen-append".into());
                let b = std::mem::replace(&mut z, ZipWriter::new(Cursor::new(vec![]))).finish().map_err(|e| format!("finish: {e}"))?;
                z = ZipWriter::new_append(b).map_err(|e| format!("new_append: {e}"))?;
            }
        }
    }
    let b = z.finish().map_err(|e| format!("finish: {e}"))?.into_inner();
    let mut a = open(b.clone()).map_err(|e| format!("reopen: {e}"))?;
    if a.len() != model.len() {
        return Err(format!("count {} vs model {}", a.len(), model.len()));
    }
    for m in &model {
        let mut v = vec![];
        let pw = if m.pw == 0 { None } else { Some(&b"pw"[..]) };
        let mut f = match pw {
            Some(p) => a.by_name_decrypt(&m.name, p),
            None => a.by_name(&m.name),
        }.map_err(|e| format!("open entry({}): {e}", m.pw))?;
        f.read_to_end(&mut v).map_err(|e| format!("read entry: {e}"))?;
        if v != m.content {
            return Err(format!("content mismatch pw={}", m.pw));
        }
        if f.is_symlink() != m.sym {
            return Err("symlink flag".into());
        }
        if m.align > 1 && f.data_start() % m.align as u64 != 0 {
            return Err(format!("alignment"));
        }
    }
    Ok(())
}
fn write_fuzz(n: u64) {
    use std::collections::BTreeMap;
    hook();
    let mut seen: BTreeMap<String, (u64, u64, Vec<String>)> = BTreeMap::new();
    for seed in 0..n {
        let tr = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let t2 = tr.clone();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut t = vec![];
            let r = write_model(seed * 2654435761 + 11, &mut t);
            *t2.lock().unwrap() = t;
            r
        }));
        let msg = match res {
            Ok(Ok(())) => continue,
            Ok(Err(m)) => m,
            Err(_) => format!("PANIC {}", LAST.lock().unwrap().clone()),
        };
        let t = tr.lock().unwrap().clone();
        let en = seen.entry(msg).or_insert((0, seed, t.clone()));
        en.0 += 1;
        if t.len() < en.2.len() || en.2.is_empty() {
            en.1 = seed;
            en.2 = t;
        }
    }
    for (k, v) in seen {
        println!("{k}: x{} seed={} trace={:?}", v.0, v.1, v.2);
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n = |i: usize, d: u64| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    match a[1].as_str() {
        "symlink_capacity" => symlink_capacity(),
        "overlap_overflow" => overlap_overflow(),
        "aes_underflow" => aes_underflow(),
        "aes_empty_auth" => aes_empty_auth(),
        "aes_truncate" => aes_truncate(),
        "sfx_nested" => sfx_nested(),
        "unicode_fields" => unicode_fields(),
        "stream_meta" => stream_meta(),
        "stream_data_start" => stream_data_start(),
        "stream_empty" => stream_empty(),
        "extract_symlink" => extract_symlink(),
        "modes" => modes(),
        "finish_readable" => finish_readable(),
        "raw_copy" => raw_copy(),
        "long_name" => long_name(),
        "long_name_drop" => long_name_drop(),
        "comment_expand" => comment_expand(),
        "append_zip64" => append_zip64(),
        "append_metadata" => append_metadata(),
        "stream_writer" => stream_writer(),
        "stream_large" => stream_large(),
        "zipcrypto_det" => zipcrypto_det(),
        "seek_encrypted" => seek_encrypted(),
        "symlink_path" => symlink_path(),
        "dir_dot" => dir_dot(),
        "merge_dup" => merge_dup(),
        "root_dbg" => root_dbg(),
        "legacy_reserve" => legacy_reserve(n(2, 1) as u16),
        "shrink_recursion" => shrink_recursion(n(2, 200000) as usize),
        "implode_ff" => implode_ff(),
        "fuzz_shrink" => fuzz_shrink(n(2, 2000)),
        "fuzz_mut" => fuzz_mut(&a[2], n(3, 3000)),
        "compress_levels" => compress_levels(),
        "extra_data" => extra_data(),
        "lzma_dict" => lzma_dict(),
        "stream_drop" => stream_drop(),
        "write_fuzz" => write_fuzz(n(2, 500)),
        "quad" => quad(n(2, 4000) as usize, n(3, 900) as usize),
        "deep_after_append" => deep_after_append(),
        "deflate64_time" => deflate64_time(),
        "shrink_search" => shrink_search(n(2, 200), n(3, 150)),
        "deep_copy_large" => deep_copy_large(),
        "read_exact_crc" => read_exact_crc(),
        "size_lie" => size_lie(),
        "dir_mode_doc" => dir_mode_doc(),
        "stream_drop2" => stream_drop2(),
        o => panic!("unknown {o}"),
    }
}
