use probe::*;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use zip::write::{FullFileOptions, SimpleFileOptions};
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};
use zip::unstable::stream::{ZipStreamFileMetadata, ZipStreamReader, ZipStreamVisitor};
use zip::read::ZipFile;

fn stored() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }
type R = Result<String, Box<dyn std::error::Error>>;
fn run(name: &str, f: fn() -> R) {
    print!("== {name}: "); std::io::stdout().flush().unwrap();
    match std::panic::catch_unwind(f) {
        Ok(Ok(s)) => println!("{s}"),
        Ok(Err(e)) => println!("TOP-ERR {e:?}"),
        Err(p) => println!("PANIC {:?}", p.downcast_ref::<String>().map(|s| s.as_str()).or(p.downcast_ref::<&str>().copied())),
    }
}

fn symlink_outside() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.add_symlink("abs", "/etc/passwd", stored())?;
    w.add_symlink("rel", "../../../../../etc/hostname", stored())?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let d = tmp("symout");
    let r = a.extract(&d);
    Ok(format!("extract={r:?} abs->{:?} rel->{:?} read_abs_len={:?}", std::fs::read_link(d.join("abs")), std::fs::read_link(d.join("rel")), std::fs::read(d.join("abs")).map(|v| v.len())))
}
fn symlink_size(sz: u64) -> R {
    let mut x = e("lnk", b"target"); x.attr = 0o120777 << 16; x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(sz, 6);
    let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
    let d = tmp("symsize");
    Ok(format!("{:?}", a.extract(&d)))
}
fn symlink_huge() -> R { symlink_size(u64::MAX) }
struct Cnt(u32, u32);
impl ZipStreamVisitor for Cnt {
    fn visit_file<R: Read>(&mut self, _f: &mut ZipFile<'_, R>) -> zip::result::ZipResult<()> { self.0 += 1; Ok(()) }
    fn visit_additional_metadata(&mut self, _m: &ZipStreamFileMetadata) -> zip::result::ZipResult<()> { self.1 += 1; Ok(()) }
}
fn stream_meta() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("a", stored().unix_permissions(0o751))?; w.write_all(b"aa")?;
    w.start_file("b", stored())?; w.write_all(b"bb")?;
    w.add_symlink("l", "a", stored())?;
    let v = w.finish()?.into_inner();
    let mut c = Cnt(0, 0);
    ZipStreamReader::new(Cursor::new(v.clone())).visit(&mut c)?;
    let d = tmp("streamx");
    let r = ZipStreamReader::new(Cursor::new(v)).extract(&d);
    Ok(format!("visit_file={} visit_additional_metadata={} extract={r:?} mode_a={:o} l_is_symlink={}", c.0, c.1, std::fs::metadata(d.join("a"))?.permissions().mode(), std::fs::symlink_metadata(d.join("l"))?.file_type().is_symlink()))
}
fn stream_data_start() -> R {
    let v = build(&[e("a", b"x")]);
    let mut c = Cursor::new(v);
    let f = zip::read::read_zipfile_from_stream(&mut c)?.unwrap();
    Ok(format!("data_start={}", f.data_start()))
}
fn overlap_overflow() -> R {
    let mut x = e("a", b"xx"); x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(1, u64::MAX);
    let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
    Ok(format!("{:?}", a.has_overlapping_files()))
}
fn aes_entry(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("s", stored().with_aes_encryption(AesMode::Aes256, "pw"))?; w.write_all(data)?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let mut raw = vec![]; a.by_index_raw(0)?.read_to_end(&mut raw)?; Ok(raw)
}
fn aes_x() -> Vec<u8> { vec![0x01, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 0, 0] }
fn aes_small() -> R {
    let mut x = e("s", &[0u8; 5]); x.method = 99; x.flags = 1; x.cx = aes_x();
    let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
    let v = a.get_aes_verification_key_and_salt(0).map(|o| o.is_some());
    let r = a.by_index_decrypt(0, b"pw").map(|_| ());
    Ok(format!("{v:?} {r:?}"))
}
fn read_dec(v: Vec<u8>) -> R {
    let mut a = ZipArchive::new(Cursor::new(v))?;
    let mut f = a.by_index_decrypt(0, b"pw")?;
    let mut o = vec![]; let r = f.read_to_end(&mut o);
    Ok(format!("{r:?} out={:?}", String::from_utf8_lossy(&o)))
}
fn aes_hmac() -> R {
    let pt = b"pay 100 dollars";
    let mut raw = aes_entry(pt)?;
    let ctl = read_dec({ let mut x = e("s", &raw); x.method = 99; x.flags = 1; x.crc = 0; x.cx = aes_x(); x.usize_ = 15; build(&[x]) })?;
    // flip the plaintext digit: ciphertext starts after salt(16)+verify(2)
    raw[18 + 4] ^= b'1' ^ b'9';
    let ctl2 = read_dec({ let mut x = e("s", &raw); x.method = 99; x.flags = 1; x.crc = 0; x.cx = aes_x(); x.usize_ = 15; build(&[x]) })?;
    // Bypass: drop auth code, claim one more byte so the reader hits EOF first; entry data is the last thing in the file.
    let body = raw[..raw.len() - 10].to_vec();
    let mut x = e("s", &body); x.method = 99; x.flags = 1; x.crc = 0; x.cx = aes_x(); x.usize_ = 15; x.csize = raw.len() as u32 + 1;
    let mut cd = vec![]; central(&mut cd, &x, 0); let total = cd.len() as u32 + 22; cd.clear(); central(&mut cd, &x, total);
    let mut v = cd.clone(); eocd(&mut v, 1, cd.len() as u32, 0, b""); local(&mut v, &x);
    let byp = read_dec(v)?;
    // empty truncation: csize == overhead
    let mut x = e("s", &raw[..28]); x.method = 99; x.flags = 1; x.crc = 0; x.cx = aes_x(); x.usize_ = 15;
    let emp = read_dec(build(&[x]))?;
    Ok(format!("\n  untampered: {ctl}\n  tampered(normal layout): {ctl2}\n  tampered+no auth code+csize past EOF: {byp}\n  csize=overhead (body removed): {emp}"))
}
fn big_entry() -> E { let mut x = e("big", b"hello"); x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(5 << 30, 5 << 30); x }
fn append_zip64() -> R {
    let v = build(&[big_entry()]);
    let a = ZipArchive::new(Cursor::new(v.clone()))?;
    let before = { let mut a = a; let s = a.by_index_raw(0)?.size(); s };
    let mut w = ZipWriter::new_append(Cursor::new(v.clone()))?;
    w.start_file("n", stored())?; w.write_all(b"n")?;
    let mut a2 = ZipArchive::new(w.finish()?)?;
    let after = a2.by_name("big").map(|f| (f.size(), f.compressed_size()));
    // merge
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.merge_archive(ZipArchive::new(Cursor::new(v))?)?;
    let mut a3 = ZipArchive::new(w.finish()?)?;
    let merged = a3.by_index_raw(0).map(|f| (f.size(), f.compressed_size()));
    Ok(format!("before={before} after_append={after:?} after_merge={merged:?}"))
}
fn large_readable() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("a", stored().large_file(true))?; w.write_all(b"hello")?;
    w.start_file("b", stored())?; w.write_all(b"world")?;
    let mut a = w.finish_into_readable()?;
    let (sz, cs) = { let f = a.by_name("a")?; (f.size(), f.compressed_size()) };
    let mut o = vec![]; let r = a.by_name("a")?.read_to_end(&mut o);
    Ok(format!("size={sz} csize={cs} read={r:?} out_len={}", o.len()))
}
fn large_deepcopy() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("a", stored().large_file(true))?; w.write_all(b"hello")?;
    w.start_file("b", stored())?; w.write_all(b"world")?;
    let r = w.deep_copy_file("a", "c");
    let mut a = ZipArchive::new(w.finish()?)?;
    let mut s = format!("deep_copy={r:?}");
    for n in ["a", "b", "c"] { let mut o = vec![]; let rr = a.by_name(n).and_then(|mut f| Ok((f.size(), f.compressed_size(), f.read_to_end(&mut o).map_err(|e| e.to_string())))); s += &format!(" {n}:{rr:?}"); }
    Ok(s)
}
fn raw_copy_crypto() -> R {
    use zip::unstable::write::FileOptionsExt;
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("c", stored().with_deprecated_encryption(b"pw"))?; w.write_all(b"secret data")?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(0)?)?;
    let mut b = ZipArchive::new(d.finish()?)?;
    let enc = b.by_index_raw(0)?.encrypted();
    let mut o = vec![]; let r = b.by_index_decrypt(0, b"pw")?.read_to_end(&mut o).map_err(|e| e.to_string());
    Ok(format!("src_encrypted=true dst_encrypted={enc} decrypt_read={r:?} out={:?}", String::from_utf8_lossy(&o)))
}
fn raw_copy_meta() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.add_symlink("l", "target", stored())?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let was = a.by_index_raw(0)?.is_symlink();
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(0)?)?;
    let mut b = ZipArchive::new(d.finish()?)?;
    let is = b.by_index_raw(0)?.is_symlink();
    let mut x = e("c", b"x"); x.comment = b"my comment".to_vec();
    let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(0)?)?;
    let mut b = ZipArchive::new(d.finish()?)?;
    let cm = b.by_index_raw(0)?.comment().to_string();
    Ok(format!("symlink src={was} dst={is} ; comment dst={cm:?}"))
}
fn raw_copy_big() -> R {
    let mut a = ZipArchive::new(Cursor::new(build(&[big_entry()])))?;
    let src = { let f = a.by_index_raw(0)?; (f.size(), f.compressed_size()) };
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.raw_copy_file(a.by_index_raw(0)?)?;
    let mut b = ZipArchive::new(d.finish()?)?;
    let f = b.by_index_raw(0)?;
    Ok(format!("src={src:?} dst=({},{})", f.size(), f.compressed_size()))
}
fn stream_crypto() -> R {
    use zip::unstable::write::FileOptionsExt;
    let mut out = vec![];
    { let mut w = ZipWriter::new_stream(&mut out);
      w.start_file("c", stored().with_deprecated_encryption(b"pw"))?; w.write_all(b"secret data")?; w.finish()?; }
    let mut a = ZipArchive::new(Cursor::new(out))?;
    let r = a.by_index_decrypt(0, b"pw").map(|_| ()).map_err(|e| e.to_string());
    let mut out = vec![];
    let aes = { let mut w = ZipWriter::new_stream(&mut out);
      w.start_file("c", stored().with_aes_encryption(AesMode::Aes256, "pw"))?; w.write_all(b"secret data")?; w.finish().map(|_| ()).map_err(|e| e.to_string()) };
    Ok(format!("zipcrypto read with right password={r:?}; aes new_stream finish={aes:?}"))
}
fn stream_large() -> R {
    let mut w = ZipWriter::new_stream(std::io::sink());
    w.start_file("big", stored().large_file(true))?;
    let chunk = vec![0u8; 1 << 20];
    for _ in 0..4097 { w.write_all(&chunk)?; }
    Ok(format!("large_file=true stream >4GiB finish={:?}", w.finish().map(|_| ()).map_err(|e| e.to_string())))
}
fn long_name() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    let r = w.start_file("a".repeat(70000), stored());
    Ok(format!("{r:?}"))
}
fn long_name_shallow() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("a", stored())?;
    w.shallow_copy_file("a", &"b".repeat(70000))?;
    let r = w.finish().map(|_| ());
    Ok(format!("{r:?}"))
}
fn comment_append() -> R {
    let mut x = e("a", b"x"); x.comment = b"caf\x82".to_vec();
    let mut u = e("u", b"y"); u.flags = 1 << 11; u.comment = "caf\u{e9}".as_bytes().to_vec();
    let v = build(&[x, u]);
    let mut a = ZipArchive::new(Cursor::new(v.clone()))?;
    let b1 = a.by_name("a")?.comment().to_string(); let b2 = a.by_name("u")?.comment().to_string();
    let w = ZipWriter::new_append(Cursor::new(v))?;
    let mut a2 = ZipArchive::new(w.finish()?)?;
    let a1 = a2.by_name("a")?.comment().to_string(); let a22 = a2.by_name("u")?.comment().to_string();
    Ok(format!("cp437 {b1:?} -> {a1:?}; utf8 {b2:?} -> {a22:?}"))
}
fn comment_panic() -> R {
    let mut x = e("a", b"x"); x.comment = vec![0xB0; 40000];
    let w = ZipWriter::new_append(Cursor::new(build(&[x])))?;
    let r = w.finish().map(|_| ());
    Ok(format!("{r:?}"))
}
fn unicode_comment() -> R {
    let mk = |c: &[u8]| { let mut x = e("a", b"x"); x.comment = c.to_vec(); let mut xf = vec![0x75, 0x63, 0, 0, 1]; xf.extend_from_slice(&crc(c).to_le_bytes()); xf.extend_from_slice("caf\u{e9}".as_bytes()); let l = (xf.len() - 4) as u16; xf[2..4].copy_from_slice(&l.to_le_bytes()); x.cx = xf; build(&[x]) };
    let mut a = ZipArchive::new(Cursor::new(mk(b"cafe")))?;
    let ok = a.by_index_raw(0)?.comment().to_string();
    let bad = ZipArchive::new(Cursor::new(mk(b"caf\x82"))).map(|_| "opened").map_err(|e| e.to_string());
    Ok(format!("ascii std comment => {ok:?}; cp437 std comment (correct crc) => {bad:?}"))
}
fn ntfs_tag() -> R {
    let mut x = e("a", b"x"); let mut f = vec![0x0a, 0, 12, 0, 0, 0, 0, 0, 2, 0, 4, 0, 1, 2, 3, 4]; f.truncate(16); x.cx = f;
    let r = ZipArchive::new(Cursor::new(build(&[x]))).map(|a| a.len()).map_err(|e| e.to_string());
    let v = { let x = e("a", b"x"); let mut v = build(&[x]); let n = v.len(); v[n - 22 + 4] = 1; v };
    let md = ZipArchive::new(Cursor::new(v)).map(|a| a.len()).map_err(|e| e.to_string());
    Ok(format!("NTFS tag 2 => {r:?}; multi-disk EOCD => {md:?}"))
}
fn perms() -> R {
    let mut s = e("suid", b"#!/bin/sh\n"); s.attr = 0o106755 << 16;
    let mut d = e("dir/", b""); d.attr = (0o040700 << 16) | 0x10;
    let mut f = e("dir/f", b"x"); f.attr = 0o100600 << 16;
    let mut a = ZipArchive::new(Cursor::new(build(&[s, d, f])))?;
    let t = tmp("perms");
    let r = a.extract(&t);
    let m = |p: &str| std::fs::metadata(t.join(p)).map(|m| format!("{:o}", m.permissions().mode())).unwrap_or_default();
    Ok(format!("uid_probe={:?} extract={r:?} suid={} dir(archive 0700)={} f={}", std::fs::read_to_string("/proc/self/status").ok().and_then(|s| s.lines().find(|l| l.starts_with("Uid")).map(|l| l.to_string())), m("suid"), m("dir"), m("dir/f")))
}
fn secret_fail() -> R {
    let mut s = e("secret.key", b"TOPSECRET"); s.attr = 0o100600 << 16;
    let mut b = e("bad", b"xxxx"); b.crc ^= 1;
    let mut a = ZipArchive::new(Cursor::new(build(&[s, b])))?;
    let t = tmp("secret");
    let r = a.extract(&t).map_err(|e| e.to_string());
    Ok(format!("extract={r:?} secret.key mode={:o} content={:?}", std::fs::metadata(t.join("secret.key"))?.permissions().mode(), std::fs::read_to_string(t.join("secret.key"))?))
}
fn symlink_relative() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.add_directory("real", stored())?;
    w.add_symlink("d", "real", stored())?;
    w.start_file("d/f", stored())?; w.write_all(b"x")?;
    let v = w.finish()?;
    let t = tmp("symrel");
    let r1 = ZipArchive::new(v)?.extract(&t).map_err(|e| e.to_string());
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("f", stored())?; w.write_all(b"x")?;
    w.add_symlink("l", "f", stored())?;
    let v = w.finish()?.into_inner();
    let t = tmp("symtwice");
    let r2 = ZipArchive::new(Cursor::new(v.clone()))?.extract(&t).map_err(|e| e.to_string());
    let r3 = ZipArchive::new(Cursor::new(v))?.extract(&t).map_err(|e| e.to_string());
    Ok(format!("through in-tree relative dir link={r1:?}; first extract={r2:?}; second extract over it={r3:?}"))
}
fn rootdir() -> R {
    let mut a = ZipArchive::new(Cursor::new(build(&[e("a/../b/x", b"1"), e("a/../b/y", b"2")])))?;
    let t = tmp("rootdir");
    let r = a.extract_unwrapped_root_dir(&t, zip::read::root_dir_common_filter).map_err(|e| e.to_string());
    Ok(format!("{r:?}"))
}
fn seek_enc() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("s", stored().with_aes_encryption(AesMode::Aes256, "pw"))?; w.write_all(b"plaintext-plaintext-plain")?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let plain = a.by_name("s").map(|_| ()).map_err(|e| e.to_string());
    let mut f = a.by_name_seek("s")?;
    let mut o = vec![]; f.read_to_end(&mut o)?;
    Ok(format!("by_name={plain:?}; by_name_seek returned Ok with {} bytes (plaintext len 25)", o.len()))
}
fn options_aes() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("s", stored().with_aes_encryption(AesMode::Aes256, "pw"))?; w.write_all(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxx")?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let opts = a.by_index_raw(0)?.options();
    let mut d = ZipWriter::new(Cursor::new(vec![]));
    d.start_file("new", opts)?; d.write_all(b"THE-PLAINTEXT-MARKER-THE-PLAINTEXT")?;
    let v = d.finish()?.into_inner();
    let leak = v.windows(20).any(|w| w == b"THE-PLAINTEXT-MARKER");
    let mut b = ZipArchive::new(Cursor::new(v))?;
    let enc = b.by_index_raw(0)?.encrypted();
    Ok(format!("entry flagged encrypted={enc}, plaintext literally in file bytes={leak}"))
}
fn levels() -> R {
    let mut s = String::new();
    for (n, m) in [("deflate", CompressionMethod::Deflated), ("bzip2", CompressionMethod::Bzip2)] {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        let r = w.start_file("a", SimpleFileOptions::default().compression_method(m).compression_level(Some(0))).map_err(|e| e.to_string());
        s += &format!("{n} level0={r:?}; ");
    }
    Ok(s)
}
fn by_path_dir() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.add_directory("dir", stored())?;
    let a = ZipArchive::new(w.finish()?)?;
    Ok(format!("names={:?} index_for_path(dir/)={:?} index_for_path(dir)={:?}", a.file_names().collect::<Vec<_>>(), a.index_for_path("dir/"), a.index_for_path("dir")))
}
fn dos_mode() -> R {
    let mut x = e("r", b"x"); x.vmade = 20; x.attr = 1;
    let mut y = e("w", b"x"); y.vmade = 20; y.attr = 0x20;
    let mut a = ZipArchive::new(Cursor::new(build(&[x, y])))?;
    let r = a.by_name("r")?.unix_mode(); let w = a.by_name("w")?.unix_mode();
    Ok(format!("readonly dos={:o} normal dos={:o}", r.unwrap_or(0), w.unwrap_or(0)))
}
struct K(u32, u32, u32);
fn crc1(c: u32, b: u8) -> u32 { let mut x = (c ^ b as u32) & 0xff; for _ in 0..8 { x = if x & 1 == 1 { (x >> 1) ^ 0xedb88320 } else { x >> 1 }; } (c >> 8) ^ x }
impl K { fn new(p: &[u8]) -> K { let mut k = K(0x12345678, 0x23456789, 0x34567890); for b in p { k.up(*b); } k }
  fn up(&mut self, b: u8) { self.0 = crc1(self.0, b); self.1 = self.1.wrapping_add(self.0 & 0xff).wrapping_mul(0x08088405).wrapping_add(1); self.2 = crc1(self.2, (self.1 >> 24) as u8); }
  fn enc(&mut self, b: u8) -> u8 { let t = (self.2 as u16) | 3; let s = (t.wrapping_mul(t ^ 1) >> 8) as u8; self.up(b); s ^ b } }
fn zc_date() -> R {
    let pt = b"hello"; let mk = |date: u16| { let mut k = K::new(b"pw"); let mut d = vec![]; for i in 0..12u8 { let b = if i == 11 { 0x50 } else { i }; d.push(k.enc(b)); } for b in pt { d.push(k.enc(*b)); } let mut x = e("z", &d); x.flags = 1 | 8; x.time = 0x5000; x.date = date; x.crc = crc(pt); x.usize_ = 5; build(&[x]) };
    let rd = |v: Vec<u8>| -> String { let mut a = ZipArchive::new(Cursor::new(v)).unwrap(); let x = match a.by_index_decrypt(0, b"pw") { Ok(mut f) => { let mut s = String::new(); let r = f.read_to_string(&mut s).map_err(|e| e.to_string()); format!("{r:?} {s}") } Err(e) => format!("ERR {e}") }; x };
    Ok(format!("valid date => {}; DOS date 0 => {}", rd(mk(0x4A21)), rd(mk(0))))
}
fn shrink_stream(extra: u32) -> (Vec<u8>, Vec<u8>) {
    let n = 7936usize; let lit = |i: usize| (i % 251) as u8; let mut b = Bits::new(); let mut out = vec![];
    for i in 0..n { b.put(lit(i) as u32, 9); out.push(lit(i)); }
    for s in 9..13 { b.put(256, s); b.put(1, s); }
    b.put(extra, 13); let j = (extra - 257) as usize; out.push(lit(j)); out.push(lit(j + 1));
    (b.finish(), out)
}
fn shrink_top() -> R {
    let mut s = String::new();
    for code in [8190u32, 8191u32] {
        let (c, o) = shrink_stream(code);
        let mut x = e("s", &c); x.method = 1; x.crc = crc(&o); x.usize_ = o.len() as u32;
        let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
        let mut got = vec![]; let r = a.by_index(0)?.read_to_end(&mut got).map_err(|e| e.to_string());
        s += &format!("code {code}: {r:?} match={}; ", got == o);
    }
    Ok(s)
}
fn legacy_huge_m(method: u16, sz: u64) -> R {
    let mut x = e("s", &[0u8; 32]); x.method = method; x.flags = 0; x.csize = u32::MAX; x.usize_ = u32::MAX; x.cx = zip64x(sz, 32);
    let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
    let mut f = a.by_index(0)?; let mut b = [0u8; 16];
    Ok(format!("{:?}", f.read(&mut b).map_err(|e| e.to_string())))
}
fn legacy_huge() -> R { let mut s = String::new(); for m in [1u16, 2, 6] { s += &match std::panic::catch_unwind(|| legacy_huge_m(m, u64::MAX)) { Ok(r) => format!("m{m}:{:?} ", r.map_err(|e| e.to_string())), Err(p) => format!("m{m}:PANIC {:?} ", p.downcast_ref::<String>().map(|s| s.as_str()).or(p.downcast_ref::<&str>().copied())) }; } Ok(s) }
fn implode_ff() -> R {
    let mut x = e("i", &[0xFFu8; 300]); x.method = 6; x.flags = 4; x.usize_ = 10;
    let mut a = ZipArchive::new(Cursor::new(build(&[x])))?;
    let mut o = vec![]; let r = a.by_index(0)?.read_to_end(&mut o).map_err(|e| e.to_string());
    Ok(format!("{r:?}"))
}
fn merge_dup() -> R {
    let mk = |c: &[u8]| -> Result<_, Box<dyn std::error::Error>> { let mut w = ZipWriter::new(Cursor::new(vec![])); w.start_file("x", stored())?; w.write_all(c)?; Ok(w.finish()?) };
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("x", stored())?; w.write_all(b"mine")?;
    let r = w.merge_archive(ZipArchive::new(mk(b"theirs")?)?).map_err(|e| e.to_string());
    let wr = w.is_writing_file();
    let stray = w.write(b"STRAY").map_err(|e| e.to_string());
    let mut a = ZipArchive::new(w.finish()?)?;
    let mut s = String::new(); a.by_name("x")?.read_to_string(&mut s)?;
    Ok(format!("merge={r:?} len={} x={s:?}; is_writing_file after merge={wr}; write after merge={stray:?}", a.len()))
}
fn stream_raw_dd() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![])); w.start_file("a", stored())?; w.write_all(b"hello")?;
    let mut a = ZipArchive::new(w.finish()?)?;
    let mut out = vec![];
    { let mut d = ZipWriter::new_stream(&mut out); d.raw_copy_file(a.by_index_raw(0)?)?; d.finish()?; }
    let flags = u16::from_le_bytes([out[6], out[7]]);
    let has_dd = out.windows(4).any(|w| w == [0x50, 0x4b, 7, 8]);
    Ok(format!("local flags={flags:#x} local sizes={:?} descriptor present={has_dd}", &out[18..26]))
}
fn zip64_comment() -> R {
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.set_zip64_comment(Some("z64"));
    w.start_file("a", stored())?; w.write_all(b"x")?;
    let v = w.finish()?;
    let a = ZipArchive::new(v)?;
    Ok(format!("zip64_comment after reopen={:?}", a.zip64_comment().map(|b| String::from_utf8_lossy(b).to_string())))
}
fn extra_sticky() -> R {
    let mut o = FullFileOptions::default().compression_method(CompressionMethod::Stored);
    let r1 = o.add_extra_data(0x9901, vec![1, 2, 3].into_boxed_slice(), false).map_err(|e| e.to_string());
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    let r2 = w.start_file("a", o).map_err(|e| e.to_string());
    Ok(format!("add_extra_data={r1:?}; start_file with same options afterwards={r2:?}"))
}
fn eocd_scan() -> R {
    // central dir with 60000 valid entries + declared one more than exist, followed by many EOCD candidates
    let n = 60000usize; let mut cd = vec![];
    let x = e("a", b"");
    let mut names = 0u32;
    for _ in 0..n { let mut y = x.clone(); y.name = format!("{names:06}").into_bytes(); names += 1; central(&mut cd, &y, 0); }
    let mut s = String::new();
    for m in [10u32, 100, 400] {
        let mut v = cd.clone();
        for _ in 0..m { eocd(&mut v, (n + 1) as u16, cd.len() as u32, 0, b""); }
        let t = std::time::Instant::now();
        let r = ZipArchive::new(Cursor::new(&v)).map(|a| a.len()).map_err(|e| e.to_string());
        s += &format!("\n  file {} bytes, {m} EOCDs: {:?} in {:?}", v.len(), r, t.elapsed());
    }
    Ok(s)
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let which = a.get(1).map(|s| s.as_str()).unwrap_or("all");
    match which {
        "symabort" => { println!("{:?}", symlink_size(1 << 44).map_err(|e| e.to_string())); return; }
        "legacyabort" => { println!("{:?}", legacy_huge_m(1, 1 << 44).map_err(|e| e.to_string())); return; }
        "longname" => { run("long_name", long_name); return; }
        "longshallow" => { run("long_name_shallow", long_name_shallow); return; }
        "commentpanic" => { run("comment_panic", comment_panic); return; }
        "legacyabort2" => { println!("{:?}", legacy_huge_m(1, 1 << 62).map_err(|e| e.to_string())); return; }
        "large" => { run("stream_large", stream_large); return; }
        "eocd" => { run("eocd_scan", eocd_scan); return; }
        _ => {}
    }
    let _ = Seek::seek(&mut Cursor::new(vec![0u8]), SeekFrom::Start(0));
    if which == "all" {
    run("symlink_outside", symlink_outside);
    run("symlink_huge", symlink_huge);
    run("stream_meta", stream_meta);
    run("stream_data_start", stream_data_start);
    run("overlap_overflow", overlap_overflow);
    run("aes_small", aes_small);
    run("aes_hmac", aes_hmac);
    run("append_zip64", append_zip64);
    run("large_readable", large_readable);
    run("large_deepcopy", large_deepcopy);
    run("raw_copy_crypto", raw_copy_crypto);
    run("raw_copy_meta", raw_copy_meta);
    run("raw_copy_big", raw_copy_big);
    run("stream_crypto", stream_crypto);
    run("comment_append", comment_append);
    }
    run("unicode_comment", unicode_comment);
    run("ntfs_tag", ntfs_tag);
    run("perms", perms);
    run("secret_fail", secret_fail);
    run("symlink_relative", symlink_relative);
    run("rootdir", rootdir);
    run("seek_enc", seek_enc);
    run("options_aes", options_aes);
    run("levels", levels);
    run("by_path_dir", by_path_dir);
    run("dos_mode", dos_mode);
    run("zc_date", zc_date);
    run("shrink_top", shrink_top);
    run("legacy_huge", legacy_huge);
    run("implode_ff", implode_ff);
    run("merge_dup", merge_dup);
    run("stream_raw_dd", stream_raw_dd);
    run("zip64_comment", zip64_comment);
    run("extra_sticky", extra_sticky);
}
