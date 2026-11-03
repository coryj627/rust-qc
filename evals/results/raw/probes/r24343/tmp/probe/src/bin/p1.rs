use probe::*;
use std::io::{Cursor, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::SimpleFileOptions};
fn open(b: Vec<u8>) -> zip::result::ZipResult<ZipArchive<Cursor<Vec<u8>>>> { ZipArchive::new(Cursor::new(b)) }
fn zip64_extra(us: u64, cs: u64) -> Vec<u8> { let mut b = us.to_le_bytes().to_vec(); b.extend(cs.to_le_bytes()); field(1, &b) }
fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    match which.as_str() {
    "symhuge" => guard("symlink u64::MAX size", || {
        let mut e = E::new("s", b"target").mode(0o120777);
        e.usize32 = Some(u32::MAX); e.csize32 = Some(6); e.extra_central = field(1, &u64::MAX.to_le_bytes());
        let mut a = open(build(&[e])).unwrap();
        println!("size={}", a.by_index(0).unwrap().size());
        format!("{:?}", a.extract(tempdir("sym")))
    }),
    "symbig" => guard("symlink 2^45 size", || {
        let mut e = E::new("s", b"target").mode(0o120777);
        e.usize32 = Some(u32::MAX); e.csize32 = Some(6); e.extra_central = field(1, &(1u64<<45).to_le_bytes());
        let mut a = open(build(&[e])).unwrap();
        format!("{:?}", a.extract(tempdir("sym")))
    }),
    "sym" => {
        guard("symlink escaping created", || {
            let d = tempdir("o");
            let mut a = open(build(&[E::new("evil", b"/etc/passwd").mode(0o120777), E::new("evil2", b"../../../etc").mode(0o120777)])).unwrap();
            let r = a.extract(&d);
            format!("{r:?}; evil -> {:?}, evil2 -> {:?}, read through={:?}", std::fs::read_link(d.join("evil")), std::fs::read_link(d.join("evil2")), std::fs::read_to_string(d.join("evil")).map(|s| s.len()))
        });
        guard("write through escaping link", || {
            let d = tempdir("o2"); let outside = tempdir("out");
            let mut a = open(build(&[E::new("l", outside.to_str().unwrap().as_bytes()).mode(0o120777), E::new("l/x", b"pwn").mode(0o100644)])).unwrap();
            let r = a.extract(&d);
            format!("{r:?}; outside/x exists={}", outside.join("x").exists())
        });
        guard("benign relative symlink + re-extract", || {
            let d = tempdir("o3");
            let z = build(&[E::new("real.txt", b"hi").mode(0o100644), E::new("link", b"real.txt").mode(0o120777)]);
            let r1 = open(z.clone()).unwrap().extract(&d);
            let r2 = open(z).unwrap().extract(&d);
            format!("first {r1:?}; second {r2:?}")
        });
        guard("file through benign relative dir symlink", || {
            let d = tempdir("o4");
            let z = build(&[E::new("real/", b"").mode(0o40755), E::new("cur", b"real").mode(0o120777), E::new("cur/x", b"hi").mode(0o100644)]);
            format!("{:?}", open(z).unwrap().extract(&d))
        });
    }
    "modes" => {
        guard("dir mode and setuid", || {
            let d = tempdir("m");
            let z = build(&[E::new("priv/", b"").mode(0o40700), E::new("priv/secret", b"x").mode(0o100600), E::new("suid", b"x").mode(0o104755), E::new("ro/", b"").mode(0o40500), E::new("zero", b"z")]);
            let r = open(z).unwrap().extract(&d);
            let m = |p: &str| std::fs::metadata(d.join(p)).map(|m| format!("{:o}", m.mode())).unwrap_or_else(|e| e.to_string());
            format!("{r:?} priv={} secret={} suid={} ro={}", m("priv"), m("priv/secret"), m("suid"), m("ro"))
        });
        guard("unix attrs with only low dos bits", || {
            let d = tempdir("m2");
            let mut e = E::new("f.txt", b"x"); e.ext_attr = 0x20; e.made_by = 0x0314;
            let mut a = open(build(&[e])).unwrap();
            let um = a.by_index(0).unwrap().unix_mode();
            let r = a.extract(&d);
            format!("unix_mode={um:?} {r:?} on-disk={:o}", std::fs::metadata(d.join("f.txt")).unwrap().permissions().mode())
        });
        guard("DOS readonly unix_mode", || {
            let mut e = E::new("f.txt", b"x"); e.ext_attr = 0x01; e.made_by = 0x0014;
            let mut a = open(build(&[e])).unwrap();
            let f = a.by_index(0).unwrap();
            format!("unix_mode={:o}", f.unix_mode().unwrap_or(0))
        });
    }
    "unicode" => {
        guard("0x7075 CRC mismatch (stale)", || {
            let mut body = vec![1u8]; body.extend(0xdeadbeefu32.to_le_bytes()); body.extend("new.txt".as_bytes());
            let mut e = E::new("renamed.txt", b"x"); e.extra_central = field(0x7075, &body);
            let f = E::new("other.txt", b"y");
            format!("{:?}", open(build(&[f, e])).map(|a| a.file_names().map(String::from).collect::<Vec<_>>()))
        });
        guard("0x6375 valid CRC over raw CP437 comment", || {
            let raw = b"caf\x82"; // cafe in cp437
            let mut body = vec![1u8]; body.extend(crc(raw).to_le_bytes()); body.extend("caf\u{e9}".as_bytes());
            let mut e = E::new("a.txt", b"x"); e.comment = raw.to_vec(); e.extra_central = field(0x6375, &body);
            format!("{:?}", open(build(&[e])).map(|mut a| a.by_index(0).unwrap().comment().to_string()))
        });
        guard("0x7075 valid CRC invalid UTF-8 payload", || {
            let mut body = vec![1u8]; body.extend(crc(b"a.txt").to_le_bytes()); body.extend(b"\xff\xfe");
            let mut e = E::new("a.txt", b"x"); e.extra_central = field(0x7075, &body);
            format!("{:?}", open(build(&[e])).map(|a| a.len()))
        });
        guard("0x7075 valid over CP437 name", || {
            let raw = b"caf\x82.txt";
            let mut body = vec![1u8]; body.extend(crc(raw).to_le_bytes()); body.extend("caf\u{e9}.txt".as_bytes());
            let mut e = E::new("x", b"x"); e.name = raw.to_vec(); e.extra_central = field(0x7075, &body);
            format!("{:?}", open(build(&[e])).map(|a| a.file_names().map(String::from).collect::<Vec<_>>()))
        });
    }
    "extras" => {
        let t = |name: &str, x: Vec<u8>| { let n = name.to_string(); guard(name, move || { let mut e = E::new("a.txt", b"x"); e.extra_central = x;
            let f = E::new("ok.txt", b"y"); let _ = &n;
            format!("{:?}", open(build(&[f, e])).map(|a| a.len()).map_err(|e| e.to_string())) }) };
        t("ntfs short len 8", field(0x000a, &[0u8; 8]));
        t("ntfs extra attr (len 40)", field(0x000a, &[0u8; 40]));
        { let mut b = vec![0u8;4]; b.extend(2u16.to_le_bytes()); b.extend(24u16.to_le_bytes()); b.extend([0u8;24]); t("ntfs unknown tag", field(0x000a, &b)); }
        t("ut zero len", field(0x5455, &[]));
        t("ut flags=3 len=5 (central)", field(0x5455, &[3,1,2,3,4]));
        t("ut flags=1 len=13", field(0x5455, &[1,1,2,3,4,5,6,7,8,9,10,11,12]));
        t("ut flags=0xff len=13", field(0x5455, &[0xff,1,2,3,4,5,6,7,8,9,10,11,12]));
        t("aes len 8", field(0x9901, &[2,0,0x41,0x45,3,0,0,0]));
        t("truncated generic extra", vec![0x99,0x99,0x10,0x00,1,2]);
        t("zip64 extra too short", field(1, &[1,2,3]));
        { let mut e = E::new("a.txt", b"x"); e.usize32=Some(u32::MAX); e.extra_central = field(1,&[1,2,3]);
          guard("zip64 required but short", move || format!("{:?}", open(build(&[e])).map(|a| a.len()).map_err(|e| e.to_string()))); }
        // multi-disk error text
        guard("multi-disk message", || {
            let mut z = build(&[E::new("a", b"x")]); let n = z.len(); z[n-22+4] = 1; // disk number
            format!("{:?}", open(z).map(|a| a.len()).map_err(|e| e.to_string()))
        });
        guard("bad CD entry message", || {
            let mut z = build(&[E::new("a", b"x")]); let n = z.len(); z[n-22-47+0] = 0; // break first CDH magic? 
            format!("{:?}", open(z).map(|a| a.len()).map_err(|e| e.to_string()))
        });
        guard("truncated EOCD comment", || {
            let mut z = build(&[E::new("a", b"x")]); let n = z.len(); z[n-2] = 5;
            format!("{:?}", open(z).map(|a| a.len()).map_err(|e| e.to_string()))
        });
    }
    "sfx" => {
        guard("sfx prefix + stored nested zip", || {
            let inner = { let mut w = ZipWriter::new(Cursor::new(vec![]));
                w.start_file("inner.txt", SimpleFileOptions::default().compression_method(CompressionMethod::Stored)).unwrap(); w.write_all(b"INNER").unwrap(); w.finish().unwrap().into_inner() };
            let z = build(&[E::new("a.txt", b"outer a"), E::new("nested.zip", &inner)]);
            let plain = open(z.clone()).map(|a| a.file_names().map(String::from).collect::<Vec<_>>());
            let mut sfx = vec![0x90u8; 4096]; sfx.extend(&z);
            let r = open(sfx).map(|a| (a.offset(), a.file_names().map(String::from).collect::<Vec<_>>())).map_err(|e| e.to_string());
            format!("plain={plain:?}\n  with 4096B stub={r:?}")
        });
        guard("sfx prefix + stored nested zip single entry", || {
            let inner = { let mut w = ZipWriter::new(Cursor::new(vec![]));
                w.start_file("inner.txt", SimpleFileOptions::default().compression_method(CompressionMethod::Stored)).unwrap(); w.write_all(b"INNER").unwrap(); w.finish().unwrap().into_inner() };
            let z = build(&[E::new("nested.zip", &inner)]);
            let mut sfx = vec![0x90u8; 4096]; sfx.extend(&z);
            let r = open(sfx).map(|mut a| { let n: Vec<String> = a.file_names().map(String::from).collect(); let mut s = vec![]; let rr = a.by_index(0).unwrap().read_to_end(&mut s).map_err(|e| e.to_string()); (a.offset(), n, rr, s.len()) }).map_err(|e| e.to_string());
            format!("{r:?}")
        });
        guard("comment contains EOCD with 0 files", || {
            let mut c = 0x06054b50u32.to_le_bytes().to_vec(); c.extend([0u8; 18]);
            let (mut o, cd) = sections(&[E::new("a.txt", b"x")], 0); let off = o.len() as u32; o.extend(&cd); o.extend(eocd(1, cd.len() as u32, off, &c));
            format!("{:?}", open(o).map(|a| a.len()).map_err(|e| e.to_string()))
        });
        guard("comment contains text with EOCD magic plain", || {
            let c = b"see PK\x05\x06 for details".to_vec();
            let (mut o, cd) = sections(&[E::new("a.txt", b"x")], 0); let off = o.len() as u32; o.extend(&cd); o.extend(eocd(1, cd.len() as u32, off, &c));
            format!("{:?}", open(o).map(|a| a.len()).map_err(|e| e.to_string()))
        });
    }
    "seek" => {
        guard("by_index_seek on zipcrypto entry", || {
            use zip::unstable::write::FileOptionsExt;
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s.txt", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_deprecated_encryption(b"pw")).unwrap();
            w.write_all(b"secret plaintext").unwrap();
            let mut a = ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
            let mut f = a.by_index_seek(0).map_err(|e| e.to_string()).unwrap();
            let mut s = vec![]; f.read_to_end(&mut s).unwrap();
            format!("returned {} bytes, looks like ciphertext: {:02x?}", s.len(), &s[..8])
        });
        guard("by_index_seek on AES entry", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s.txt", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_aes_encryption(zip::AesMode::Aes256, "pw")).unwrap();
            w.write_all(b"secret plaintext").unwrap();
            let mut a = ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
            let mut f = a.by_index_seek(0).map_err(|e| e.to_string()).unwrap();
            let mut s = vec![]; f.read_to_end(&mut s).unwrap();
            format!("returned {} bytes for 16-byte plaintext", s.len())
        });
    }
    "aes" => {
        for cs in [0u32, 5, 12, 19] { guard(&format!("AES csize={cs}"), move || {
            let mut body = vec![2,0,0x41,0x45,1]; body.extend(0u16.to_le_bytes());
            let mut e = E::new("a", &vec![0u8; cs as usize]); e.method = 99; e.flags = 1; e.extra_central = field(0x9901, &body);
            let mut a = open(build(&[e])).unwrap();
            let r1 = a.get_aes_verification_key_and_salt(0).map(|o| o.map(|i| i.salt.len())).map_err(|e| e.to_string());
            let r = match a.by_index_decrypt(0, b"pw") { Ok(mut f) => { let mut s = vec![]; format!("{:?} {}", f.read_to_end(&mut s).map_err(|e| e.to_string()), s.len()) } Err(e) => e.to_string() };
            format!("salt={r1:?} decrypt={r}")
        }); }
    }
    "stream" => {
        guard("data_start() on stream file", || {
            let z = build(&[E::new("a", b"x")]);
            let mut c = Cursor::new(z);
            let f = zip::read::read_zipfile_from_stream(&mut c).unwrap().unwrap();
            format!("data_start={}", f.data_start())
        });
        guard("stream symlink", || {
            let d = tempdir("st");
            let z = build(&[E::new("real.txt", b"hello").mode(0o100644), E::new("lnk", b"real.txt").mode(0o120777), E::new("x.sh", b"#!").mode(0o100755), E::new("d/", b"").mode(0o40700)]);
            let r = zip::unstable::stream::ZipStreamReader::new(Cursor::new(z.clone())).extract(&d).map_err(|e| e.to_string());
            let md = std::fs::symlink_metadata(d.join("lnk")).unwrap();
            let dirm = std::fs::metadata(d.join("d")).unwrap().mode();
            let d2 = tempdir("st2");
            open(z).unwrap().extract(&d2).unwrap();
            format!("{r:?} stream: lnk is_symlink={} content={:?} mode={:o} dir={:o} | archive extract is_symlink={}", md.file_type().is_symlink(), std::fs::read_to_string(d.join("lnk")).ok(), md.mode(), dirm, std::fs::symlink_metadata(d2.join("lnk")).unwrap().file_type().is_symlink())
        });
        guard("stream with truncated central", || {
            let d = tempdir("st3");
            let z = build(&[E::new("a", b"hello").mode(0o100644)]);
            let cut = z.len() - 22 - 10; // truncated central entry
            format!("{:?}", zip::unstable::stream::ZipStreamReader::new(Cursor::new(z[..cut].to_vec())).extract(&d).map_err(|e| e.to_string()))
        });
        guard("stream stored entry with garbage trailing/EOF", || {
            let z = build(&[E::new("a", b"hello")]);
            let mut c = Cursor::new(z[..z.len()-22-47].to_vec()); // only local part
            let mut n=0; loop { match zip::read::read_zipfile_from_stream(&mut c) { Ok(Some(_)) => n+=1, Ok(None) => break, Err(e) => return format!("after {n} entries: {e}") } }
            format!("{n}")
        });
        guard("stream eocd only (empty archive)", || {
            let z = eocd(0,0,0,b"");
            let mut c = Cursor::new(z);
            format!("{:?}", zip::read::read_zipfile_from_stream(&mut c).map(|o| o.is_some()).map_err(|e| e.to_string()))
        });
        guard("stream archive after files: no central (eocd directly)", || {
            let (mut o, _) = sections(&[E::new("a", b"hello")], 0); o.extend(eocd(0,0,0,b""));
            let mut c = Cursor::new(o); let mut n = 0;
            loop { match zip::read::read_zipfile_from_stream(&mut c) { Ok(Some(_)) => n+=1, Ok(None) => return format!("ok {n}"), Err(e) => return format!("after {n}: {e}") } }
        });
    }
    _ => println!("?"),
    }
}
