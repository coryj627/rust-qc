use probe::*;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::{SimpleFileOptions, FullFileOptions}};
use zip::unstable::write::FileOptionsExt;
use zip::read::HasZipMetadata;
fn open(b: Vec<u8>) -> zip::result::ZipResult<ZipArchive<Cursor<Vec<u8>>>> { ZipArchive::new(Cursor::new(b)) }
fn stored() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }
fn rd<R: Read + Seek>(a: &mut ZipArchive<R>, n: &str) -> String {
    match a.by_name(n) { Ok(mut f) => { let (s,c)=(f.size(),f.compressed_size()); let mut v=vec![]; let r=f.read_to_end(&mut v); format!("{n}: size={s} csize={c} read={:?} data={:?}", r.map_err(|e|e.to_string()), String::from_utf8_lossy(&v[..v.len().min(40)])) } Err(e)=>format!("{n}: {e}") }
}
struct VecW(Vec<u8>);
impl Write for VecW { fn write(&mut self,b:&[u8])->std::io::Result<usize>{self.0.extend_from_slice(b);Ok(b.len())} fn flush(&mut self)->std::io::Result<()>{Ok(())} }
fn hwm()->String{ std::fs::read_to_string("/proc/self/status").unwrap().lines().filter(|l| l.starts_with("VmHWM")||l.starts_with("VmPeak")).collect::<Vec<_>>().join(" ") }
fn main() {
    if std::env::args().nth(1).as_deref()==Some("dump") {
        let d="/work/tmp/samples"; std::fs::create_dir_all(d).unwrap();
        let w=|n:&str,b:Vec<u8>| { let r=zip::ZipArchive::new(Cursor::new(b.clone())).map(|a| format!("{:?}",a.file_names().map(String::from).collect::<Vec<_>>())).map_err(|e|e.to_string()); println!("zip-crate {n}: {r:?}"); std::fs::write(format!("{d}/{n}.zip"),b).unwrap(); };
        let mut z=build(&[E::new("a.txt",b"x")]); let n=z.len(); z[n-2]=5; w("truncated_comment",z);
        let mut body=vec![1u8]; body.extend(0xdeadbeefu32.to_le_bytes()); body.extend("new.txt".as_bytes());
        let mut e=E::new("renamed.txt",b"x"); e.extra_central=field(0x7075,&body); w("stale_unicode_path",build(&[e]));
        let mut e=E::new("a.txt",b"x"); e.extra_central=field(0x000a,&[0u8;8]); w("ntfs_short",build(&[e]));
        let mut e=E::new("a.txt",b"x"); e.extra_central=field(0x5455,&[1,1,2,3,4,5,6,7,8,9,10,11,12]); w("ut_len_mismatch",build(&[e]));
        let mut e=E::new("a.txt",b"x"); e.extra_central=field(0x5455,&[]); w("ut_empty",build(&[e]));
        let es:Vec<E>=(0..65537).map(|i| E::new(&format!("{i}"),b"")).collect(); let (mut o,cd)=sections(&es,0); let off=o.len() as u32; o.extend(&cd); o.extend(eocd(1,cd.len() as u32,off,b"")); w("count_wrapped",o);
        let mut z=build(&[E::new("a",b"x")]); let n=z.len(); z[n-22+8]=0; w("on_disk_zero",z);
        let inner={ let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("inner.txt",stored()).unwrap(); w.write_all(b"INNER").unwrap(); w.finish().unwrap().into_inner() };
        let mut sfx=vec![0x90u8;4096]; sfx.extend(build(&[E::new("a.txt",b"outer a"),E::new("nested.zip",&inner)])); w("sfx_nested",sfx);
        let b=std::fs::read("/work/crate/tests/data/chinese.zip").unwrap(); let o=ZipWriter::new_append(Cursor::new(b)).unwrap().finish().unwrap().into_inner(); w("chinese_after_append",o);
        return;
    }
    if std::env::args().nth(1).as_deref()==Some("mem") {
        let which=std::env::args().nth(2).unwrap();
        guard(&which.clone(), move || {
            let (m,d):(u16,Vec<u8>)=match which.as_str() {
                "lzma" => { let mut d=vec![0u8,0,5,0, 0x5d]; d.extend(0xffffffffu32.to_le_bytes()); d.extend([0u8;32]); (14,d) }
                "ppmd" => { let p:u16=7|(0xff<<4)|(0<<12); let mut d=p.to_le_bytes().to_vec(); d.extend([0u8;32]); (98,d) }
                _ => panic!() };
            let t=std::time::Instant::now();
            let mut e=E::new("a",&d); e.method=m; e.crc=Some(0); e.usize32=Some(1000);
            let mut a=open(build(&[e])).unwrap(); let z=a.by_index(0); let mut v=vec![];
            let r=match z { Ok(mut f)=>f.read_to_end(&mut v).map_err(|e|e.to_string()), Err(e)=>Err(e.to_string()) };
            format!("{r:?} out={} {} t={:?}",v.len(),hwm(),t.elapsed())
        });
        return;
    }
    guard("abort_file then more, with compression & AES", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("keep", SimpleFileOptions::default()).unwrap(); w.write_all(b"keep me").unwrap();
        w.start_file("gone", SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes128,"x")).unwrap(); w.write_all(&[9u8;100]).unwrap();
        w.abort_file().unwrap();
        w.start_file("after", stored()).unwrap(); w.write_all(b"after").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{} {} n={}", rd(&mut a,"keep"), rd(&mut a,"after"), a.len())
    });
    guard("abort_file w/o more data then finish smaller", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("keep", stored()).unwrap(); w.write_all(b"keep me").unwrap();
        w.start_file("gone", stored()).unwrap(); w.write_all(&vec![7u8;1000]).unwrap();
        w.abort_file().unwrap();
        let out = w.finish().unwrap().into_inner();
        let l = out.len(); let mut a = open(out).unwrap();
        format!("len={l} {} n={}", rd(&mut a,"keep"), a.len())
    });
    guard("abort last finished file and new_append", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", stored()).unwrap(); w.write_all(b"aaa").unwrap();
        w.start_file("b", stored()).unwrap(); w.write_all(b"bbb").unwrap();
        let out = w.finish().unwrap();
        let mut w = ZipWriter::new_append(out).unwrap();
        w.abort_file().unwrap(); // removes b
        w.start_file("c", stored()).unwrap(); w.write_all(b"ccc").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{} {} {} n={}", rd(&mut a,"a"), rd(&mut a,"b"), rd(&mut a,"c"), a.len())
    });
    guard("abort empties writer on append", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", stored()).unwrap(); w.write_all(b"aaa").unwrap();
        let out = w.finish().unwrap();
        let mut w = ZipWriter::new_append(out).unwrap();
        w.abort_file().unwrap();
        w.start_file("c", stored()).unwrap(); w.write_all(b"ccc").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{} n={}", rd(&mut a,"c"), a.len())
    });
    guard("shallow copy roundtrip + abort of original", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", stored()).unwrap(); w.write_all(b"aaa").unwrap();
        w.shallow_copy_file("a","b").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{} {} overlap={:?}", rd(&mut a,"a"), rd(&mut a,"b"), a.has_overlapping_files().map_err(|e|e.to_string()))
    });
    guard("shallow_copy after start_file: open file finished?", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", SimpleFileOptions::default()).unwrap(); w.write_all(b"aaa").unwrap();
        w.shallow_copy_file("a","b").unwrap();
        w.deep_copy_file("a","c").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{} {} {}", rd(&mut a,"a"), rd(&mut a,"b"), rd(&mut a,"c"))
    });
    guard("deep_copy AES entry", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes256,"pw")).unwrap(); w.write_all(&[3u8;64]).unwrap();
        w.deep_copy_file("a","c").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let r = match a.by_name_decrypt("c",b"pw") { Ok(mut f) => { let mut v=vec![]; format!("{:?}", f.read_to_end(&mut v).map_err(|e|e.to_string())) } Err(e)=>e.to_string() }; r
    });
    guard("deep_copy with non-ascii dest then flags", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("a", stored()).unwrap(); w.write_all(b"aaa").unwrap();
        w.deep_copy_file("a","caf\u{e9}").unwrap();
        w.shallow_copy_file("a","\u{4e2d}").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{:?}", a.file_names().map(String::from).collect::<Vec<_>>())
    });
    guard("alignment", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        let mut s=String::new();
        for (i,al) in [2u16,3,4,4096,65535,32768].iter().enumerate() {
            let r = w.start_file(format!("f{i}"), stored().with_alignment(*al)).map_err(|e|e.to_string()); w.write_all(b"xx").ok();
            s+=&format!("{al}:{r:?} ");
        }
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        for i in 0..a.len() { let f=a.by_index(i).unwrap(); s+=&format!("{}@{} ", f.name(), f.data_start()); }
        s
    });
    guard("alignment with large_file + AES", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("pad", stored()).unwrap(); w.write_all(b"xxx").unwrap();
        w.start_file("f", stored().large_file(true).with_alignment(4096).with_aes_encryption(zip::AesMode::Aes128,"pw")).unwrap(); w.write_all(&[1u8;64]).unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let mut f = a.by_name_decrypt("f",b"pw").unwrap(); let mut v=vec![]; let r=f.read_to_end(&mut v).map_err(|e|e.to_string());
        format!("{r:?} start={}", f.data_start())
    });
    guard("aes small file on Deflated roundtrip + AE version", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("f", SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes192,"pw")).unwrap(); w.write_all(b"small").unwrap();
        w.start_file("empty", SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes192,"pw")).unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let mut s=String::new();
        for n in ["f","empty"] { match a.by_name_decrypt(n,b"pw") { Ok(mut f)=>{let mut v=vec![]; s+=&format!("{n}: {:?} {:?}; ", f.read_to_end(&mut v).map_err(|e|e.to_string()), f.get_metadata().aes_mode.map(|m| format!("{:?}",m)));}, Err(e)=>s+=&format!("{n}: {e}; ") } }
        s
    });
    guard("aes auth code tamper empty file", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("e", stored().with_aes_encryption(zip::AesMode::Aes128,"pw")).unwrap();
        let mut o = w.finish().unwrap().into_inner();
        // auth code = last 10 bytes before central
        let c = o.windows(4).position(|x| x==b"PK\x01\x02").unwrap(); o[c-1]^=0xff;
        let mut a = open(o).unwrap();
        let mut f = a.by_name_decrypt("e",b"pw").unwrap(); let mut v=vec![];
        format!("{:?}", f.read_to_end(&mut v).map_err(|e|e.to_string()))
    });
    guard("aes tamper auth code in normal file", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("e", stored().with_aes_encryption(zip::AesMode::Aes128,"pw")).unwrap(); w.write_all(b"abc").unwrap();
        let mut o = w.finish().unwrap().into_inner();
        let c = o.windows(4).position(|x| x==b"PK\x01\x02").unwrap(); o[c-1]^=0xff;
        let mut a = open(o).unwrap();
        let mut f = a.by_name_decrypt("e",b"pw").unwrap(); let mut v=vec![];
        format!("{:?}", f.read_to_end(&mut v).map_err(|e|e.to_string()))
    });
    guard("zipcrypto zero password read w/o password; encrypted dir", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("e", stored().with_deprecated_encryption(b"")).unwrap(); w.write_all(b"abc").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let r0 = a.by_name("e").map(|_| ()).map_err(|e|e.to_string());
        let r = a.by_name_decrypt("e",b"").map(|_| ()).map_err(|e|e.to_string());
        format!("{r0:?} {r:?}")
    });
    guard("extract encrypted archive error", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("e", stored().with_deprecated_encryption(b"x")).unwrap(); w.write_all(b"abc").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{:?}", a.extract(tempdir("ee")).map_err(|e|e.to_string()))
    });
    guard("by_index_raw on aes options roundtrip", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("e", SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes128,"pw")).unwrap(); w.write_all(&[5u8;100]).unwrap();
        let mut s = open(w.finish().unwrap().into_inner()).unwrap();
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.raw_copy_file(s.by_index_raw(0).unwrap()).unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let mut f = a.by_name_decrypt("e",b"pw").unwrap(); let mut v=vec![]; 
        format!("{:?} {}", f.read_to_end(&mut v).map_err(|e|e.to_string()), v.len())
    });
    guard("raw_copy_file_touch preserves type", || {
        let mut s = open(build(&[E::new("l", b"target").mode(0o120777)])).unwrap();
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.raw_copy_file_touch(s.by_index_raw(0).unwrap(), zip::DateTime::default(), Some(0o120777)).unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let f = a.by_index(0).unwrap(); format!("is_symlink={}", f.is_symlink())
    });
    guard("options of existing symlink -> add", || {
        let opts = SimpleFileOptions::default().unix_permissions(0o120777);
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.start_file("x", opts).unwrap(); w.write_all(b"y").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let f = a.by_index(0).unwrap(); format!("{:o}", f.unix_mode().unwrap())
    });
    guard("add_directory perm dir 0o700 + is_dir + unix", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        w.add_directory("d", stored().unix_permissions(0o700)).unwrap();
        w.add_directory("e\\", stored()).unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let s=format!("{:?}", a.file_names().map(String::from).collect::<Vec<_>>());
        let f = a.by_name("d/").unwrap(); format!("{s} {:o} dir={}", f.unix_mode().unwrap(), f.is_dir())
    });
    guard("start_file_from_path odd paths", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        let mut s=String::new();
        for p in ["a/b/../c","/abs/x","../up","./d/./e","f/","."] { let r=w.start_file_from_path(p, stored()).map_err(|e|e.to_string()); s+=&format!("{p}:{r:?} "); }
        let a = open(w.finish().unwrap().into_inner()).unwrap();
        format!("{s}-> {:?}", a.file_names().map(String::from).collect::<Vec<_>>())
    });
    guard("from_path: non-UTF-8 collisions", || {
        use std::os::unix::ffi::OsStrExt; use std::ffi::OsStr;
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        let r1=w.start_file_from_path(std::path::Path::new(OsStr::from_bytes(b"\xff")), stored()).map_err(|e|e.to_string());
        let r2=w.start_file_from_path(std::path::Path::new(OsStr::from_bytes(b"\xfe")), stored()).map_err(|e|e.to_string());
        format!("{r1:?} {r2:?}")
    });
    guard("flush_on_finish_file with stream", || {
        let mut w = ZipWriter::new_stream(VecW(vec![]));
        w.start_file("a", SimpleFileOptions::default()).unwrap(); w.write_all(b"aaa").unwrap();
        w.add_symlink("l","a",stored()).unwrap();
        let r = w.abort_file().map_err(|e|e.to_string());
        let r2 = w.start_file("b",SimpleFileOptions::default()).map_err(|e|e.to_string());
        w.write_all(b"bbb").ok();
        let o = w.finish().map(|c| c.into_inner().0).map_err(|e|e.to_string());
        let d = o.map(|o| { let mut a=open(o).unwrap(); format!("{} {} n={}", rd(&mut a,"a"), rd(&mut a,"b"), a.len()) });
        format!("abort={r:?} start={r2:?} {d:?}")
    });
    guard("new_stream ok readable by stream reader? (data descriptor)", || {
        let mut w = ZipWriter::new_stream(VecW(vec![]));
        w.start_file("a", SimpleFileOptions::default()).unwrap(); w.write_all(b"aaa").unwrap();
        let o = w.finish().unwrap().into_inner().0;
        let mut c = Cursor::new(o);
        format!("{:?}", zip::read::read_zipfile_from_stream(&mut c).map(|o| o.is_some()).map_err(|e|e.to_string()))
    });
    guard("stored file seek semantic", || {
        let mut a = open(build(&[E::new("a", b"0123456789")])).unwrap();
        let mut f = a.by_index_seek(0).unwrap();
        let mut s = String::new();
        s+=&format!("{:?} ", f.seek(SeekFrom::End(5)).map_err(|e|e.to_string()));
        s+=&format!("{:?} ", f.seek(SeekFrom::Start(100)).map_err(|e|e.to_string()));
        let mut b=[0u8;4]; s+=&format!("{:?} ", f.read(&mut b).map_err(|e|e.to_string()));
        s+=&format!("{:?} ", f.seek(SeekFrom::End(-2)).map_err(|e|e.to_string()));
        let mut v=vec![]; f.read_to_end(&mut v).unwrap(); s+=&format!("{:?} ", v);
        s+=&format!("{:?}", f.seek(SeekFrom::Current(-100)).map_err(|e|e.to_string()));
        s
    });
    guard("extended timestamp in file with central short", || {
        let mut w = ZipWriter::new(Cursor::new(vec![]));
        let mut o = FullFileOptions::default().compression_method(CompressionMethod::Stored);
        o.add_extra_data(0x5455, vec![1,1,2,3,4].into_boxed_slice(), false).unwrap();
        w.start_file("a", o).unwrap(); w.write_all(b"x").unwrap();
        let mut a = open(w.finish().unwrap().into_inner()).unwrap();
        let f = a.by_index(0).unwrap(); format!("{:?}", f.extra_data_fields().count())
    });
    guard("big comment file header reading 64k names", || {
        let mut e = E::new("x", b""); e.name=vec![b'n';65535]; e.comment=vec![b'c';65535]; e.extra_central=vec![0x99,0x99,0xfb,0xff]; e.extra_central.extend(vec![0;65531]);
        let a = open(build(&[e])).unwrap(); format!("{} {}", a.len(), a.file_names().next().unwrap().len())
    });
    guard("eocd comment 65535 and magic at window edge", || {
        let mut s=String::new();
        for k in [0usize,1,2,3,2044,2045,2046,2047,2048,2049,4090,65535] {
            let (mut o, cd)=sections(&[E::new("a",b"x")],0); let off=o.len() as u32; o.extend(&cd); o.extend(eocd(1,cd.len() as u32,off,&vec![b'c';k]));
            let r=open(o).map(|a| a.len()).map_err(|e|e.to_string()); if r!=Ok(1){ s+=&format!("{k}:{r:?} "); }
        }
        // trailing garbage
        for k in [1usize,1000,2040,2050,5000,70000,200000] {
            let (mut o, cd)=sections(&[E::new("a",b"x")],0); let off=o.len() as u32; o.extend(&cd); o.extend(eocd(1,cd.len() as u32,off,b"")); o.extend(vec![0u8;k]);
            let r=open(o).map(|a| a.len()).map_err(|e|e.to_string()); if r!=Ok(1){ s+=&format!("trail {k}:{r:?} "); }
        }
        // prefix crossing windows
        for k in [1usize,3,4,2044,2048,5000] {
            let mut z=vec![0u8;k]; z.extend(build(&[E::new("a",b"x"),E::new("b",b"yy")]));
            let mut a=open(z).unwrap(); let r=rd(&mut a,"b"); if !r.contains("yy") { s+=&format!("prefix {k}:{r} "); }
        }
        if s.is_empty() {"all ok".into()} else {s}
    });
}
