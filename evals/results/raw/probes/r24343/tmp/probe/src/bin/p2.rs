use probe::*;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::{SimpleFileOptions, FullFileOptions}};
use zip::unstable::write::FileOptionsExt;
use zip::read::ZipFile;
use zip::unstable::stream::{ZipStreamReader, ZipStreamVisitor, ZipStreamFileMetadata};
fn open(b: Vec<u8>) -> zip::result::ZipResult<ZipArchive<Cursor<Vec<u8>>>> { ZipArchive::new(Cursor::new(b)) }
fn stored() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }
fn rd<R: Read + Seek>(a: &mut ZipArchive<R>, n: &str) -> String {
    match a.by_name(n) { Ok(mut f) => { let (s,c)=(f.size(),f.compressed_size()); let mut v=vec![]; let r=f.read_to_end(&mut v); format!("{n}: size={s} csize={c} read={:?} data={:?}", r.map_err(|e|e.to_string()), String::from_utf8_lossy(&v[..v.len().min(40)])) } Err(e)=>format!("{n}: {e}") }
}
#[derive(Default)] struct Cnt(u32,u32);
impl ZipStreamVisitor for Cnt {
    fn visit_file<R: Read>(&mut self, _f: &mut ZipFile<'_, R>) -> zip::result::ZipResult<()> { self.0+=1; Ok(()) }
    fn visit_additional_metadata(&mut self, _m: &ZipStreamFileMetadata) -> zip::result::ZipResult<()> { self.1+=1; Ok(()) }
}
struct VecW(Vec<u8>);
impl Write for VecW { fn write(&mut self,b:&[u8])->std::io::Result<usize>{self.0.extend_from_slice(b);Ok(b.len())} fn flush(&mut self)->std::io::Result<()>{Ok(())} }
fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    match which.as_str() {
    "stream" => {
        guard("visit_additional_metadata called?", || {
            let z = build(&[E::new("a", b"x").mode(0o100755), E::new("b", b"y").mode(0o100600)]);
            let mut c = Cnt::default(); let r = ZipStreamReader::new(Cursor::new(z)).visit(&mut c);
            format!("{r:?} files={} metadata={}", c.0, c.1)
        });
        guard("stream extract perms", || {
            let d = tempdir("s"); let z = build(&[E::new("x.sh", b"#!").mode(0o100755), E::new("p", b"s").mode(0o100600)]);
            let r = ZipStreamReader::new(Cursor::new(z)).extract(&d);
            format!("{r:?} x.sh={:o} p={:o}", std::fs::metadata(d.join("x.sh")).unwrap().mode(), std::fs::metadata(d.join("p")).unwrap().mode())
        });
    }
    "wstream" => {
        guard("new_stream + large_file small entry", || {
            let mut w = ZipWriter::new_stream(VecW(vec![]));
            w.start_file("big", stored().large_file(true)).unwrap(); w.write_all(b"hello world").unwrap();
            w.start_file("small", stored()).unwrap(); w.write_all(b"second").unwrap();
            let out = w.finish().unwrap().into_inner().0;
            let mut a = open(out).unwrap();
            format!("{} | {}", rd(&mut a, "big"), rd(&mut a, "small"))
        });
        guard("new_stream + zipcrypto", || {
            let mut w = ZipWriter::new_stream(VecW(vec![]));
            w.start_file("s", stored().with_deprecated_encryption(b"pw")).unwrap(); w.write_all(b"secret").unwrap();
            let out = w.finish().unwrap().into_inner().0;
            let mut a = open(out).unwrap();
            let r = match a.by_name_decrypt("s", b"pw") { Ok(mut f) => { let mut v=vec![]; format!("{:?} {:?}", f.read_to_end(&mut v).map_err(|e|e.to_string()), v) } Err(e)=>format!("ERR {e}") }; r
        });
        guard("new_stream + raw_copy_file / add_directory bytes", || {
            let src = build(&[E::new("a.txt", b"payload")]);
            let mut s = open(src).unwrap();
            let mut w = ZipWriter::new_stream(VecW(vec![]));
            w.raw_copy_file(s.by_name("a.txt").unwrap()).unwrap();
            w.add_directory("d", stored()).unwrap();
            w.start_file("z", stored()).unwrap(); w.write_all(b"zz").unwrap();
            let out = w.finish().unwrap().into_inner().0;
            // walk: local header flags at offset 6; data followed by descriptor?
            let flags = u16::from_le_bytes([out[6],out[7]]);
            let hdr = 30 + u16::from_le_bytes([out[26],out[27]]) as usize + u16::from_le_bytes([out[28],out[29]]) as usize;
            let after = &out[hdr..hdr+7+4];
            let mut a = open(out.clone()).unwrap();
            format!("first entry flags={flags:#x} data+next4={:?} -> {} | {}", String::from_utf8_lossy(after), rd(&mut a,"a.txt"), rd(&mut a,"z"))
        });
    }
    "readable" => {
        guard("finish_into_readable + large_file", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("big", stored().large_file(true)).unwrap(); w.write_all(b"hello world").unwrap();
            w.start_file("small", stored()).unwrap(); w.write_all(b"second").unwrap();
            let mut a = w.finish_into_readable().unwrap();
            format!("{} | {}", rd(&mut a, "big"), rd(&mut a, "small"))
        });
        guard("deep_copy after large_file", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("big", stored().large_file(true)).unwrap(); w.write_all(b"hello world").unwrap();
            w.start_file("small", stored()).unwrap(); w.write_all(b"second").unwrap();
            let r = w.deep_copy_file("big", "copy").map_err(|e|e.to_string());
            let out = w.finish().unwrap().into_inner();
            let mut a = open(out).unwrap();
            format!("{r:?} {} | {} | {}", rd(&mut a, "big"), rd(&mut a, "copy"), rd(&mut a, "small"))
        });
    }
    "append" => {
        guard("new_append keeps zip64 size (crafted 5GiB)", || {
            let mut e = E::new("big", b"hello"); e.usize32=Some(u32::MAX); e.extra_central=field(1,&(5u64<<30).to_le_bytes());
            let z = build(&[e]);
            let before = open(z.clone()).unwrap().by_index(0).unwrap().size();
            let mut w = ZipWriter::new_append(Cursor::new(z)).unwrap();
            w.start_file("n", stored()).unwrap(); w.write_all(b"new").unwrap();
            let out = w.finish().unwrap().into_inner();
            let after = open(out).map(|mut a| a.by_name("big").map(|f| f.size()).map_err(|e|e.to_string())).map_err(|e|e.to_string());
            format!("before={before} after={after:?}")
        });
        guard("real: sparse archive offset beyond 4GiB, append", || {
            let p = tempdir("sp").join("a.zip");
            let mut f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&p).unwrap();
            f.seek(SeekFrom::Start(5u64<<30)).unwrap();
            let mut w = ZipWriter::new(f);
            w.start_file("far", stored()).unwrap(); w.write_all(b"far data").unwrap();
            w.finish().unwrap();
            let mut a = ZipArchive::new(std::fs::File::open(&p).unwrap()).unwrap();
            let r0 = rd(&mut a,"far"); let hs = a.by_name("far").unwrap().header_start();
            let f = std::fs::OpenOptions::new().read(true).write(true).open(&p).unwrap();
            let mut w = ZipWriter::new_append(f).unwrap();
            w.start_file("n", stored()).unwrap(); w.write_all(b"new").unwrap();
            w.finish().unwrap();
            let r1 = ZipArchive::new(std::fs::File::open(&p).unwrap()).map(|mut a| format!("{} hs={:?}", rd(&mut a,"far"), a.by_name("far").map(|f| f.header_start()).map_err(|e|e.to_string()))).map_err(|e|e.to_string());
            let _ = std::fs::remove_file(&p);
            format!("before: {r0} hs={hs}\n  after append: {r1:?}")
        });
        guard("new_append + large_file entry (small)", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("big", stored().large_file(true)).unwrap(); w.write_all(b"hello world").unwrap();
            let out = w.finish().unwrap();
            let mut w = ZipWriter::new_append(out).unwrap();
            w.start_file("n", stored()).unwrap(); w.write_all(b"new").unwrap();
            let r = w.deep_copy_file("big","copy").map_err(|e|e.to_string());
            let out = w.finish().unwrap().into_inner();
            let copyoff = out.windows(8).position(|w| w == b"PK\x03\x04\x2d\0\0\0").unwrap_or(0);
            let mut a = open(out.clone()).unwrap();
            let hs = a.by_name("copy").map(|f| f.header_start()).unwrap_or(0) as usize;
            let l = &out[hs..hs+30];
            let (cs,us,xl)=(u32::from_le_bytes(l[18..22].try_into().unwrap()),u32::from_le_bytes(l[22..26].try_into().unwrap()),u16::from_le_bytes(l[28..30].try_into().unwrap()));
            let _ = copyoff;
            // stream reader over the bytes of the copy
            let mut c = Cursor::new(out[hs..].to_vec());
            let sr = zip::read::read_zipfile_from_stream(&mut c).map(|o| o.map(|f| (f.compressed_size(), f.size()))).map_err(|e|e.to_string());
            format!("{r:?} {} {} | {} | deep copy local header csize={cs:#x} usize={us:#x} extra_len={xl} stream-read={sr:?}", rd(&mut a,"big"), rd(&mut a,"n"), rd(&mut a,"copy"))
        });
        guard("new_append with CP437 comment", || {
            let mut e = E::new("a.txt", b"x"); e.comment = b"caf\x82".to_vec();
            let z = build(&[e]);
            let c0 = open(z.clone()).unwrap().by_index(0).unwrap().comment().to_string();
            let w = ZipWriter::new_append(Cursor::new(z)).unwrap();
            let out = w.finish().unwrap().into_inner();
            let c1 = open(out).unwrap().by_index(0).unwrap().comment().to_string();
            format!("before={c0:?} after={c1:?}")
        });
        if std::env::var("ABORTY").is_ok() { guard("new_append long CP437 comment", || {
            let mut e = E::new("a.txt", b"x"); e.comment = vec![0x82; 40000];
            let w = ZipWriter::new_append(Cursor::new(build(&[e]))).unwrap();
            let r = w.finish().map(|c| c.into_inner().len()).map_err(|e|e.to_string());
            format!("{r:?}")
        }); }
        guard("new_append CP437 name that is valid UTF-8", || {
            let mut e = E::new("x", b"x"); e.name = b"caf\xc3\xa9.txt".to_vec();
            let z = build(&[e]);
            let n0: Vec<String> = open(z.clone()).unwrap().file_names().map(String::from).collect();
            let w = ZipWriter::new_append(Cursor::new(z)).unwrap();
            let n1: Vec<String> = open(w.finish().unwrap().into_inner()).unwrap().file_names().map(String::from).collect();
            format!("before={n0:?} after={n1:?}")
        });
        guard("new_append keeps flags bits (implode)", || {
            let mut e = E::new("a", b"data"); e.method=6; e.flags=6; e.crc=Some(0); e.usize32=Some(1);
            let z = build(&[e]);
            let w = ZipWriter::new_append(Cursor::new(z)).unwrap();
            let out = w.finish().unwrap().into_inner();
            let cdh = out.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
            format!("central flags after append={:#x}", u16::from_le_bytes([out[cdh+8],out[cdh+9]]))
        });
        guard("append with prefix", || {
            let mut z = vec![7u8; 1000]; z.extend(build(&[E::new("a.txt", b"prefixed")]));
            let mut w = ZipWriter::new_append(Cursor::new(z)).unwrap();
            w.start_file("n", stored()).unwrap(); w.write_all(b"new").unwrap();
            let mut a = open(w.finish().unwrap().into_inner()).unwrap();
            format!("{} | {}", rd(&mut a,"a.txt"), rd(&mut a,"n"))
        });
        guard("new_append with archive comment + no new files", || {
            let (mut o, cd) = sections(&[E::new("a.txt", b"x")], 0); let off=o.len() as u32; o.extend(&cd); o.extend(eocd(1,cd.len() as u32,off,b"arc comment"));
            let w = ZipWriter::new_append(Cursor::new(o.clone())).unwrap();
            let out = w.finish().unwrap().into_inner();
            let a = open(out.clone()).unwrap();
            format!("len {} -> {} entries={} comment={:?}", o.len(), out.len(), a.len(), String::from_utf8_lossy(a.comment()))
        });
    }
    "raw" => {
        guard("raw_copy_file symlink + comment + extra", || {
            let mut l = E::new("lnk", b"target").mode(0o120777); l.comment=b"note".to_vec(); l.extra_central=field(0x5455,&[1,1,2,3,4]); l.extra_local=field(0x5455,&[1,1,2,3,4]);
            let mut s = open(build(&[l, E::new("d/", b"").mode(0o40755)])).unwrap();
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(s.by_name("lnk").unwrap()).unwrap();
            w.raw_copy_file(s.by_name("d/").unwrap()).unwrap();
            let mut a = open(w.finish().unwrap().into_inner()).unwrap();
            let f = a.by_name("lnk").unwrap();
            let r = format!("is_symlink={} mode={:?} comment={:?} extra_fields={}", f.is_symlink(), f.unix_mode().map(|m| format!("{m:o}")), f.comment(), f.extra_data_fields().count());
            drop(f);
            let g = a.by_name("d/").unwrap();
            format!("{r}; dir mode={:?}", g.unix_mode().map(|m| format!("{m:o}")))
        });
        guard("raw_copy_file zipcrypto", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", stored().with_deprecated_encryption(b"pw")).unwrap(); w.write_all(b"secret").unwrap();
            let mut s = open(w.finish().unwrap().into_inner()).unwrap();
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(s.by_index_raw(0).unwrap()).unwrap();
            let mut a = open(w.finish().unwrap().into_inner()).unwrap();
            let enc = a.by_name("s").map(|f| f.encrypted()).map_err(|e|e.to_string());
            let dec = match a.by_name_decrypt("s", b"pw") { Ok(mut f) => { let mut v=vec![]; format!("{:?} {:?}", f.read_to_end(&mut v).map_err(|e|e.to_string()), String::from_utf8_lossy(&v)) } Err(e)=>format!("ERR {e}") };
            format!("encrypted flag after copy={enc:?}; decrypt -> {dec}")
        });
        guard("raw_copy_file zip64 sizes", || {
            let mut e = E::new("big", b"hello"); e.usize32=Some(u32::MAX); e.extra_central=field(1,&(5u64<<30).to_le_bytes());
            let mut s = open(build(&[e])).unwrap();
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(s.by_index_raw(0).unwrap()).unwrap();
            let mut a = open(w.finish().unwrap().into_inner()).unwrap();
            format!("after raw copy: size={:?}", a.by_index_raw(0).map(|f| (f.size(), f.compressed_size())).map_err(|e|e.to_string()))
        });
    }
    "api" => {
        if std::env::var("ABORTY").is_ok() { guard("start_file long name", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            let n = "a".repeat(70000);
            format!("{:?}", w.start_file(n, stored()).map_err(|e|e.to_string()))
        }); }
        guard("add_extra_data poison", || {
            let mut o = FullFileOptions::default().compression_method(CompressionMethod::Stored);
            let r = o.add_extra_data(0x0001, vec![0u8;8].into_boxed_slice(), false).map_err(|e|e.to_string());
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            let r2 = w.start_file("a", o).map_err(|e|e.to_string());
            format!("add -> {r:?}; start_file afterwards -> {r2:?}")
        });
        guard("merge_archive duplicate", || {
            let a = open(build(&[E::new("a.txt", b"first")])).unwrap();
            let b = open(build(&[E::new("a.txt", b"second")])).unwrap();
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            let r1 = w.merge_archive(a).map_err(|e|e.to_string()); let r2 = w.merge_archive(b).map_err(|e|e.to_string());
            let mut m = open(w.finish().unwrap().into_inner()).unwrap();
            format!("{r1:?} {r2:?} n={} {}", m.len(), rd(&mut m,"a.txt"))
        });
        guard("write after merge_archive", || {
            let a = open(build(&[E::new("a.txt", b"first")])).unwrap();
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.merge_archive(a).unwrap();
            let r = w.write(b"JUNK").map_err(|e|e.to_string());
            let out = w.finish().unwrap().into_inner();
            let junk = out.windows(4).any(|x| x==b"JUNK");
            format!("write -> {r:?}, junk in output={junk}")
        });
        guard("comment too long", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.set_raw_comment(vec![b'c'; 70000].into_boxed_slice());
            format!("{:?}", w.finish().map(|c| c.into_inner().len()).map_err(|e|e.to_string()))
        });
        guard("entries 65535 / 65536", || {
            let mut s = String::new();
            for n in [65535usize, 65536] {
                let mut w = ZipWriter::new(Cursor::new(vec![]));
                for i in 0..n { w.start_file(format!("{i}"), stored()).unwrap(); }
                let out = w.finish().unwrap().into_inner();
                s += &format!("{n}: {:?}; ", open(out).map(|a| a.len()).map_err(|e|e.to_string()));
            }
            s
        });
        guard("zipcrypto data-descriptor with invalid date", || {
            // Encrypt with crate, then rewrite: craft header check against time: compute by hand impossible w/o API; show validator input only
            "skipped".to_string()
        });
        guard("by_path directory", || {
            let mut a = open(build(&[E::new("dir/", b""), E::new("dir/f", b"x")])).unwrap();
            format!("by_path(dir/)={:?} by_name(dir/)={:?} index_for_path(dir/f)={:?}", a.by_path("dir/").map(|f| f.name().to_string()).map_err(|e|e.to_string()), a.by_name("dir/").map(|f| f.name().to_string()).map_err(|e|e.to_string()), a.index_for_path("dir/f"))
        });
        guard("backslash names", || {
            let d = tempdir("bs");
            let r = open(build(&[E::new("dir\\", b""), E::new("dir\\f.txt", b"x")])).unwrap().extract(&d).map_err(|e|e.to_string());
            let ls: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect();
            format!("{r:?} {ls:?}")
        });
        guard("zip64 EOCD with prefix + comment", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.set_zip64_comment(Some("z64")); w.start_file("a", stored()).unwrap(); w.write_all(b"data").unwrap();
            let out = w.finish().unwrap().into_inner();
            let mut p = vec![1u8; 777]; p.extend(out);
            let mut a = open(p).unwrap();
            format!("offset={} z64={:?} {}", a.offset(), a.zip64_comment().map(|c| String::from_utf8_lossy(c).to_string()), rd(&mut a,"a"))
        });
        guard("overlap check adjacent false positive", || {
            let mut a = open(build(&[E::new("a", b"xxx"), E::new("b", b"yyy")])).unwrap();
            format!("{:?}", a.has_overlapping_files().map_err(|e|e.to_string()))
        });
        guard("overlap huge csize overflow", || {
            let mut e = E::new("a", b"xxx"); e.csize32=Some(u32::MAX); e.extra_central=field(1,&u64::MAX.to_le_bytes());
            let mut a = open(build(&[e])).unwrap();
            format!("{:?}", a.has_overlapping_files().map_err(|e|e.to_string()))
        });
        guard("seek end overflow", || {
            use std::io::Seek;
            let mut e = E::new("a", b"xxx"); e.csize32=Some(u32::MAX); e.extra_central=field(1,&u64::MAX.to_le_bytes());
            let mut a = open(build(&[e])).unwrap();
            let mut f = a.by_index_seek(0).map_err(|e|e.to_string()).unwrap();
            format!("{:?}", f.seek(SeekFrom::End(0)).map_err(|e|e.to_string()))
        });
        guard("decompressed_size and sizes", || {
            let z = build(&[E::new("a", b"xxx")]);
            let a = open(z).unwrap(); format!("{:?}", a.decompressed_size())
        });
    }
    "misc" => {
        guard("root_dir with ./ prefix", || {
            let a = open(build(&[E::new("./root/", b""), E::new("./root/a", b"x"), E::new("./root/s/b", b"y")])).unwrap();
            let d = tempdir("r"); let mut a2 = open(build(&[E::new("./root/a", b"x")])).unwrap();
            let r = a2.extract_unwrapped_root_dir(&d, zip::read::root_dir_common_filter);
            format!("root_dir={:?} extract={r:?} ls={:?}", a.root_dir(zip::read::root_dir_common_filter), std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>())
        });
        guard("root_dir with root/../evil", || {
            let d = tempdir("r2");
            let mut a = open(build(&[E::new("root/../evil", b"x"), E::new("root/a", b"y")])).unwrap();
            let rd = a.root_dir(zip::read::root_dir_common_filter).map_err(|e|e.to_string());
            let r = a.extract_unwrapped_root_dir(&d, zip::read::root_dir_common_filter).map_err(|e|e.to_string());
            format!("root_dir={rd:?} extract={r:?} ls={:?}", std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>())
        });
        guard("zipcrypto bit3 with invalid dos date", || {
            fn crc8(c: u32, b: u8) -> u32 { let mut x = (c ^ b as u32) & 0xff; for _ in 0..8 { x = if x&1==1 { 0xedb88320 ^ (x>>1) } else { x>>1 }; } x ^ (c>>8) }
            struct K(u32,u32,u32);
            impl K { fn up(&mut self,b:u8){ self.0=crc8(self.0,b); self.1=self.1.wrapping_add(self.0&0xff).wrapping_mul(0x08088405).wrapping_add(1); self.2=crc8(self.2,(self.1>>24) as u8);} fn s(&self)->u8{ let t=(self.2 as u16)|3; ((t.wrapping_mul(t^1))>>8) as u8 } fn enc(&mut self,b:u8)->u8{ let c=self.s()^b; self.up(b); c } }
            let mut k=K(0x12345678,0x23456789,0x34567890); for b in b"pw" { k.up(*b); }
            let time: u16 = 0x6000; let plain=b"secret";
            let mut hdr=[0u8;12]; hdr[11]=(time>>8) as u8;
            let mut data: Vec<u8> = hdr.iter().map(|b| k.enc(*b)).collect(); data.extend(plain.iter().map(|b| k.enc(*b)));
            let mut res = vec![];
            for date in [0x5021u16, 0u16] {
                let mut e = E::new("s", &data); e.flags=1|8; e.crc=Some(crc(plain)); e.usize32=Some(plain.len() as u32); e.time=time; e.date=date;
                let mut a = open(build(&[e])).unwrap();
                res.push(match a.by_index_decrypt(0,b"pw") { Ok(mut f) => { let mut v=vec![]; let r=f.read_to_end(&mut v).map_err(|e|e.to_string()); format!("date={date:#x}: {r:?}") } Err(e)=>format!("date={date:#x}: ERR {e}") });
            }
            res.join(" ; ")
        });
        guard("compression_level documented ranges", || {
            let mut s=String::new();
            for (n,m,l) in [("deflate 0",CompressionMethod::Deflated,0i64),("bzip2 0",CompressionMethod::Bzip2,0),("zstd 0",CompressionMethod::Zstd,0),("zstd -7",CompressionMethod::Zstd,-7)] {
                let mut w=ZipWriter::new(Cursor::new(vec![]));
                s+=&format!("{n}: {:?}; ", w.start_file("a",SimpleFileOptions::default().compression_method(m).compression_level(Some(l))).map_err(|e|e.to_string()));
            }
            s
        });
        guard("cp437 table dump", || {
            let mut e=E::new("x",b""); e.name=(0x20u8..=0xffu8).filter(|b| *b!=b'/' && *b!=b'\\').collect();
            let a=open(build(&[e])).unwrap();
            let n: Vec<String>=a.file_names().map(String::from).collect();
            std::fs::write("/work/tmp/cp437.txt", &n[0]).unwrap(); "written".into()
        });
        guard("ppmd restore methods", || {
            let mut s=String::new();
            for r in 0u16..16 { let p: u16 = 7 | (0<<4) | (r<<12); let mut d=p.to_le_bytes().to_vec(); d.extend([0u8;32]);
              let mut e=E::new("a",&d); e.method=98; e.crc=Some(0); e.usize32=Some(10);
              let mut a=open(build(&[e])).unwrap(); let mut f=a.by_index(0).unwrap(); let mut v=vec![];
              s+=&format!("{r}:{}; ", f.read_to_end(&mut v).map(|_| "ok".to_string()).unwrap_or_else(|e| e.to_string())); }
            s
        });
        guard("multiple zip64 + other extras removal", || {
            // zip64 in middle of extras: fields after it survive and earlier survive?
            let mut x=field(0x1234,b"AA"); x.extend(field(1,&5u64.to_le_bytes())); x.extend(field(0x5678,b"BB"));
            let mut e=E::new("a",b"hello"); e.usize32=Some(u32::MAX); e.extra_central=x;
            let mut a=open(build(&[e])).unwrap(); let f=a.by_index(0).unwrap();
            format!("size={} extra={:?}", f.size(), f.extra_data())
        });
        guard("two removable zip64 fields", || {
            let mut x=field(0x1,&5u64.to_le_bytes()); x.extend(field(0x5678,b"BB")); x.extend(field(1,&[])); x.extend(field(0x9999,b"CC"));
            let mut e=E::new("a",b"hello"); e.usize32=Some(u32::MAX); e.extra_central=x;
            let mut a=open(build(&[e])).unwrap(); let f=a.by_index(0).unwrap();
            format!("size={} extra={:?}", f.size(), f.extra_data())
        });
        guard("only zip64 field: extra none", || {
            let mut e=E::new("a",b"hello"); e.usize32=Some(u32::MAX); e.extra_central=field(1,&5u64.to_le_bytes());
            let mut a=open(build(&[e])).unwrap(); let f=a.by_index(0).unwrap();
            format!("size={} extra={:?}", f.size(), f.extra_data())
        });
        guard("eocd count wrap 65536+", || {
            // writer-without-zip64 style: 65537 entries, EOCD count wraps to 1, cd size right
            let es: Vec<E> = (0..65537).map(|i| E::new(&format!("{i}"), b"")).collect();
            let (mut o, cd) = sections(&es, 0); let off=o.len() as u32; o.extend(&cd); o.extend(eocd(1, cd.len() as u32, off, b""));
            format!("{:?}", open(o).map(|a| a.len()).map_err(|e|e.to_string()))
        });
        guard("eocd disk totals disagree", || {
            // number_of_files_on_this_disk=0 but total=1
            let mut z=build(&[E::new("a",b"x")]); let n=z.len(); z[n-22+8]=0;
            format!("{:?}", open(z).map(|a| a.len()).map_err(|e|e.to_string()))
        });
    }
    "more" => {
        guard("set_zip64_comment roundtrip", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.set_zip64_comment(Some("z64")); w.start_file("a", stored()).unwrap(); w.write_all(b"data").unwrap();
            let out = w.finish().unwrap().into_inner();
            let has = out.windows(4).any(|x| x==b"PK\x06\x06");
            let a = open(out.clone()).unwrap();
            let r = a.zip64_comment().map(|c| String::from_utf8_lossy(c).to_string());
            let w2 = ZipWriter::new_append(Cursor::new(out)).unwrap();
            let c2 = w2.get_raw_zip64_comment().map(|c| c.to_vec());
            format!("eocd64 written={has} read back={r:?}; new_append sees {c2:?}")
        });
        guard("vacuous stream symlink test, rewound", || {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            writer.add_symlink("symlink/", "../dest-sibling/", SimpleFileOptions::default()).unwrap();
            writer.start_file("symlink/dest-file", SimpleFileOptions::default()).unwrap();
            let c = writer.finish().unwrap();
            let pos = c.position(); let len = c.get_ref().len();
            let parent = tempdir("v"); std::fs::create_dir(parent.join("dest-sibling")).unwrap(); std::fs::create_dir(parent.join("dest")).unwrap();
            let err0 = ZipStreamReader::new(c.clone()).extract(parent.join("dest")).map_err(|e|e.to_string());
            let mut c2 = c; c2.set_position(0);
            let r = ZipStreamReader::new(c2).extract(parent.join("dest")).map_err(|e|e.to_string());
            format!("cursor pos={pos}/{len}; as in test -> {err0:?}; rewound -> {r:?}; dest/symlink/dest-file exists={}", parent.join("dest/symlink/dest-file").exists())
        });
        guard("new_append central flags/method AES + implode", || {
            let last = |out: &[u8]| { let p = out.windows(4).rposition(|w| w==b"PK\x01\x02").unwrap(); (u16::from_le_bytes([out[p+8],out[p+9]]), u16::from_le_bytes([out[p+10],out[p+11]])) };
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes128,"pw")).unwrap(); w.write_all(&[1u8;64]).unwrap();
            let out = w.finish().unwrap().into_inner();
            let before = last(&out);
            let w2 = ZipWriter::new_append(Cursor::new(out)).unwrap();
            let o2 = w2.finish().unwrap().into_inner();
            let mut e = E::new("a", b"data"); e.method=6; e.flags=6; e.crc=Some(0); e.usize32=Some(1);
            let z = build(&[e]); let b3 = last(&z);
            let o3 = ZipWriter::new_append(Cursor::new(z)).unwrap().finish().unwrap().into_inner();
            format!("AES (flags,method) before={before:?} after={:?}; implode before={b3:?} after={:?}", last(&o2), last(&o3))
        });
        guard("implode 256 code-length bytes", || {
            let mut d = vec![255u8]; d.extend(vec![0x07u8;256]); // 256 literals, length 8
            // len tree: 64 symbols length 6 => one byte (run 64 -> high nibble 15 gives 16 max) so 4 bytes 0xf5
            d.push(3); d.extend([0xf5u8;4]); d.push(3); d.extend([0xf5u8;4]);
            d.extend([0u8;16]);
            let mut e = E::new("a", &d); e.method=6; e.flags=4; e.crc=Some(0); e.usize32=Some(2);
            let mut a = open(build(&[e])).unwrap(); let mut f = a.by_index(0).unwrap(); let mut v=vec![];
            format!("{:?} out={:?}", f.read_to_end(&mut v).map_err(|e|e.to_string()), v)
        });
        guard("AES release short csize", || {
            let mut res = String::new();
            for cs in [5u32, 19] { let mut body = vec![2,0,0x41,0x45,1]; body.extend(0u16.to_le_bytes());
              let mut e = E::new("a", &vec![0u8; cs as usize]); e.method = 99; e.flags = 1; e.extra_central = field(0x9901, &body);
              let mut a = open(build(&[e])).unwrap();
              let r = a.by_index_decrypt(0, b"pw").map(|_| "ok").map_err(|e| e.to_string());
              let r1 = a.get_aes_verification_key_and_salt(0).map(|o| o.map(|i| i.salt.len())).map_err(|e| e.to_string());
              res += &format!("cs={cs}: {r:?} {r1:?}; "); }
            res
        });
        guard("huge offset and overlap (release)", || {
            let mut e = E::new("a", b"xxx"); e.csize32=Some(u32::MAX); e.extra_central=field(1,&u64::MAX.to_le_bytes());
            let mut a = open(build(&[e])).unwrap();
            format!("{:?}", a.has_overlapping_files().map_err(|e|e.to_string()))
        });
        guard("symlink u64::MAX release", || {
            let mut e = E::new("s", b"target").mode(0o120777);
            e.usize32 = Some(u32::MAX); e.csize32 = Some(6); e.extra_central = field(1, &u64::MAX.to_le_bytes());
            let mut a = open(build(&[e])).unwrap();
            format!("{:?}", a.extract(tempdir("sym")).map_err(|e|e.to_string()))
        });
        guard("data_start stream release", || {
            let mut c = Cursor::new(build(&[E::new("a", b"x")]));
            let f = zip::read::read_zipfile_from_stream(&mut c).unwrap().unwrap();
            format!("{}", f.data_start())
        });
    }
    "aesforge" => {
        guard("AE-2 tampered ciphertext, oversize csize, data at EOF", || {
            let mut w = ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_aes_encryption(zip::AesMode::Aes256,"pw")).unwrap();
            w.write_all(b"pay alice $100").unwrap();
            let mut a = open(w.finish().unwrap().into_inner()).unwrap();
            let mut raw = vec![]; let (extra, crcv, us);
            { let mut f = a.by_index_raw(0).unwrap(); f.read_to_end(&mut raw).unwrap(); extra = f.extra_data().unwrap().to_vec(); crcv = f.crc32(); us = f.size(); }
            // baseline ok
            let mut good=String::new(); a.by_index_decrypt(0,b"pw").unwrap().read_to_string(&mut good).unwrap();
            // tamper: flip bits turning "alice" into "mallo"? flip one char; drop auth code; keep salt(16)+verify(2)+ct
            let mut body = raw[..raw.len()-10].to_vec();
            body[16+2+11] ^= b'1' ^ b'9'; // "$100" -> "$900"
            // Layout: CD, EOCD, then local header + data with csize larger than what remains.
            let name=b"s";
            let csize_claim = raw.len() as u32 + 1;
            let mut loc = 0x04034b50u32.to_le_bytes().to_vec();
            loc.extend(51u16.to_le_bytes()); loc.extend(1u16.to_le_bytes()); loc.extend(99u16.to_le_bytes()); loc.extend([0u8;4]); loc.extend(crcv.to_le_bytes());
            loc.extend(csize_claim.to_le_bytes()); loc.extend((us as u32).to_le_bytes()); loc.extend((name.len() as u16).to_le_bytes()); loc.extend((extra.len() as u16).to_le_bytes()); loc.extend(name); loc.extend(&extra); loc.extend(&body);
            let cd_len = 46 + name.len() + extra.len();
            let header_off = (cd_len + 22) as u32;
            let mut cd = 0x02014b50u32.to_le_bytes().to_vec();
            cd.extend(0x0333u16.to_le_bytes()); cd.extend(51u16.to_le_bytes()); cd.extend(1u16.to_le_bytes()); cd.extend(99u16.to_le_bytes()); cd.extend(0u16.to_le_bytes()); cd.extend(0x5021u16.to_le_bytes()); cd.extend(crcv.to_le_bytes());
            cd.extend(csize_claim.to_le_bytes()); cd.extend((us as u32).to_le_bytes()); cd.extend((name.len() as u16).to_le_bytes()); cd.extend((extra.len() as u16).to_le_bytes()); cd.extend(0u16.to_le_bytes()); cd.extend([0u8;4]); cd.extend((0o100644u32<<16).to_le_bytes()); cd.extend(header_off.to_le_bytes());
            cd.extend(name); cd.extend(&extra);
            let mut z = cd.clone(); z.extend(eocd(1, cd.len() as u32, 0, b"")); z.extend(&loc);
            let mut t = open(z).unwrap();
            let mut f = t.by_index_decrypt(0,b"pw").unwrap();
            let mut v=vec![]; let r=f.read_to_end(&mut v).map_err(|e|e.to_string());
            format!("original={good:?}; forged: {r:?} {:?}", String::from_utf8_lossy(&v))
        });
        guard("declared size not enforced", || {
            let mut e=E::new("a",b"hello world"); e.usize32=Some(1);
            let mut a=open(build(&[e])).unwrap(); let mut f=a.by_index(0).unwrap(); let mut v=vec![];
            let dec = { let r=f.read_to_end(&mut v).map_err(|e|e.to_string()); (f.size(), r, v.len()) };
            format!("size()={} read={:?} got={}", dec.0, dec.1, dec.2)
        });
        guard("decompressed_size with data descriptor", || {
            let mut e=E::new("a",b"hello"); e.flags=8;
            let a=open(build(&[e])).unwrap(); format!("{:?}", a.decompressed_size())
        });
    }
    "legacy" => {
        let meta = |m: u16, f: u16, data: Vec<u8>, us: Option<u64>| {
            let mut e = E::new("a", &data); e.method=m; e.flags=f; e.crc=Some(0);
            if let Some(u) = us { e.usize32=Some(u32::MAX); e.extra_central=field(1,&u.to_le_bytes()); }
            let mut a = open(build(&[e])).unwrap();
            let mut f = a.by_index(0).unwrap(); let mut v=vec![];
            format!("{:?} out={}", f.read_to_end(&mut v).map_err(|e|e.to_string()), v.len())
        };
        for (nm,m) in [("shrink",1u16),("reduce",2),("implode",6)] {
            guard(&format!("{nm} size=u64::MAX"), move || meta(m,0,vec![0u8;64],Some(u64::MAX)));
        }
        guard("implode num_bytes byte=255", move || { let mut d = vec![0xffu8]; d.extend(vec![0u8;512]); meta(6,4,d,None) });
    }
    "shrinkrec" => {
        guard("shrink repeated control codes", || {
            // 9-bit LSB codes: literal 0x41, then (256,2) repeated
            let n: usize = std::env::args().nth(2).unwrap().parse().unwrap();
            let mut bits: Vec<u16> = vec![0x41];
            for _ in 0..n { bits.push(256); bits.push(2); }
            bits.push(0x42);
            let mut out=vec![]; let mut acc=0u32; let mut nb=0;
            for b in bits { acc |= (b as u32) << nb; nb+=9; while nb>=8 { out.push(acc as u8); acc>>=8; nb-=8; } }
            if nb>0 { out.push(acc as u8); }
            let mut e = E::new("a", &out); e.method=1; e.usize32=Some(2); e.crc=Some(crc(b"AB"));
            let t=std::time::Instant::now();
            let mut a = open(build(&[e])).unwrap();
            let mut f = a.by_index(0).unwrap(); let mut v=vec![];
            format!("compressed={} {:?} out={:?} t={:?}", out.len(), f.read_to_end(&mut v).map_err(|e|e.to_string()), String::from_utf8_lossy(&v), t.elapsed())
        });
    }
    "eocdquad" => {
        let n: usize = std::env::args().nth(2).unwrap().parse().unwrap();
        let m: usize = std::env::args().nth(3).unwrap().parse().unwrap();
        guard("many EOCDs each pointing at a long bad CD", move || {
            let es: Vec<E> = (0..n).map(|i| E::new(&format!("{i}"), b"")).collect();
            let (mut o, cd) = sections(&es, 0);
            let off = o.len() as u32; o.extend(&cd);
            // count one too many so that every attempt parses all n entries then fails at next EOCD
            for _ in 0..m { o.extend(eocd((n+1) as u16, cd.len() as u32, off, b"")); }
            let t=std::time::Instant::now(); let sz=o.len();
            let r = open(o).map(|a| a.len()).map_err(|e|e.to_string());
            format!("file={sz}B {r:?} t={:?}", t.elapsed())
        });
    }
    "overlap" => {
        let n: usize = std::env::args().nth(2).unwrap().parse().unwrap();
        guard("has_overlapping_files scale", move || {
            let es: Vec<E> = (0..n).map(|i| E::new(&format!("{i}"), b"x")).collect();
            let z = if n>65535 { // via writer for zip64
                let mut w = ZipWriter::new(Cursor::new(vec![])); for e in &es { w.start_file(String::from_utf8_lossy(&e.name).to_string(), stored()).unwrap(); w.write_all(b"x").unwrap(); } w.finish().unwrap().into_inner()
            } else { build(&es) };
            let mut a = open(z).unwrap();
            let t=std::time::Instant::now();
            format!("n={n} {:?} t={:?}", a.has_overlapping_files().map_err(|e|e.to_string()), t.elapsed())
        });
    }
    "big" => {
        guard("new_stream large_file >4GiB", || {
            let mut w = ZipWriter::new_stream(std::io::sink());
            w.start_file("big", stored().large_file(true)).unwrap();
            let buf = vec![0u8; 1<<24];
            for _ in 0..257 { w.write_all(&buf).unwrap(); }
            format!("{:?}", w.finish().map(|_| ()).map_err(|e|e.to_string()))
        });
    }
    _ => println!("?"),
    }
}
