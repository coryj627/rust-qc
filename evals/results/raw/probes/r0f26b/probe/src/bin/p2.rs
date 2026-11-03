use probe::*;
use std::io::{Cursor, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use zip::{ZipArchive, ZipWriter, write::{SimpleFileOptions, FullFileOptions}, CompressionMethod, AesMode, HasZipMetadata};
use zip::read::read_zipfile_from_stream;
use zip::unstable::write::FileOptionsExt;

fn t<F: FnOnce() -> String>(name: &str, f: F) {
    if let Ok(x)=std::env::var("SKIP") { if x.split('|').any(|y| name.contains(y)) { return; } }
    let r = catch_unwind(AssertUnwindSafe(f));
    match r { Ok(s) => println!("[{name}] {s}"),
        Err(e) => { let m = e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(); println!("[{name}] PANIC: {m}") } }
}
fn st() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }
fn read_all(a:&mut ZipArchive<Cursor<Vec<u8>>>, n:&str)->String{
    match a.by_name(n) { Err(e)=>format!("open err: {e}"), Ok(mut f)=>{ let mut v=vec![]; match f.read_to_end(&mut v){Ok(_)=>format!("{:?}",String::from_utf8_lossy(&v)),Err(e)=>format!("read err: {e} (got {} bytes)",v.len())} } }
}
fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let on = |n: &str| which.is_empty() || which == n;
    if on("large") {
        t("large_file(true) finish_into_readable", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("a", st().large_file(true)).unwrap(); w.write_all(b"hello").unwrap();
            let mut a=w.finish_into_readable().unwrap();
            let size=a.by_index_raw(0).unwrap().size(); let cs=a.by_index_raw(0).unwrap().compressed_size();
            format!("size={size} compressed={cs} content={}", read_all(&mut a,"a"))
        });
        t("large_file(true) finish + new", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("a", st().large_file(true)).unwrap(); w.write_all(b"hello").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            {let sz=a.by_index_raw(0).unwrap().size(); format!("size={} content={}", sz, read_all(&mut a,"a"))}
        });
        t("large_file deep_copy, source not last", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("a", st().large_file(true)).unwrap(); w.write_all(b"hello").unwrap();
            w.start_file("b", st()).unwrap(); w.write_all(b"world").unwrap();
            w.deep_copy_file("a","c").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            format!("a={} b={} c={}", read_all(&mut a,"a"), read_all(&mut a,"b"), read_all(&mut a,"c"))
        });
        t("raw copy, source >4GiB declared", || {
            let mut b=vec![]; b.extend((5u64<<30).to_le_bytes());
            let mut e=E::new("big",b"compressed"); e.uncomp=Some(u32::MAX); e.cextra=ext(1,&b);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let before=a.by_index_raw(0).unwrap().size();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a2=ZipArchive::new(Cursor::new(v)).unwrap();
            let f=a2.by_index_raw(0).unwrap();
            format!("before size={before}; after size={} compressed={}", f.size(), f.compressed_size())
        });
        t("new_append keeps zip64 fields?", || {
            let mut b=vec![]; b.extend((5u64<<30).to_le_bytes());
            let mut e=E::new("big",b"compressed"); e.uncomp=Some(u32::MAX); e.cextra=ext(1,&b);
            let mut w=ZipWriter::new_append(Cursor::new(build(&[e]))).unwrap();
            w.start_file("x",st()).unwrap(); w.write_all(b"y").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a2=ZipArchive::new(Cursor::new(v)).unwrap();
            let s=a2.by_name("big").map(|f| f.size());
            format!("after append size={s:?} (was {})", 5u64<<30)
        });
    }
    if on("raw") {
        t("raw copy symlink", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.add_symlink("l","target",SimpleFileOptions::default()).unwrap();
            w.add_directory("d",SimpleFileOptions::default()).unwrap();
            let mut src=w.finish_into_readable().unwrap();
            let b1=src.by_name("l").unwrap().is_symlink(); let before=(b1, src.by_name("d/").unwrap().unix_mode());
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(src.by_name("l").unwrap()).unwrap();
            w.raw_copy_file(src.by_name("d/").unwrap()).unwrap();
            let mut a=w.finish_into_readable().unwrap();
            let f=a.by_name("l").unwrap(); let af=(f.is_symlink(), format!("{:o}",f.unix_mode().unwrap()));drop(f);
            format!("before={before:?} after l={af:?} d={:o}", a.by_name("d/").unwrap().unix_mode().unwrap())
        });
        t("raw copy zipcrypto", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", st().with_deprecated_encryption(b"pw")).unwrap(); w.write_all(b"secret!").unwrap();
            let mut src=w.finish_into_readable().unwrap();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(src.by_index_raw(0).unwrap()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let enc=a.by_index_raw(0).unwrap().encrypted();
            let dec=a.by_index_decrypt(0,b"pw").map(|mut f|{let mut v=vec![];let r=f.read_to_end(&mut v);(r.map_err(|e|e.to_string()),v.len())}).map_err(|e|e.to_string());
            format!("encrypted flag after copy={enc} plain read={} decrypt={dec:?}", read_all(&mut a,"s"))
        });
        t("raw copy aes (control)", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", st().with_aes_encryption(AesMode::Aes128,"pw")).unwrap(); w.write_all(b"secret!").unwrap();
            let mut src=w.finish_into_readable().unwrap();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(src.by_index_raw(0).unwrap()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let dec=a.by_index_decrypt(0,b"pw").map(|mut f|{let mut v=vec![];let r=f.read_to_end(&mut v);(r.map_err(|e|e.to_string()),v.len())}).map_err(|e|e.to_string());
            format!("decrypt={dec:?}")
        });
        t("raw copy implode flags", || {
            let mut e=E::new("i",b"\x00\x01\x02"); e.method=6; e.flags=6; e.uncomp=Some(3);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let a2=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut a2=a2; let fl=a2.by_index_raw(0).unwrap().get_metadata().flags;
            format!("flags before=6 after={fl}")
        });
        t("raw copy comment+extra+data-descriptor", || {
            let mut e=E::new("c",b"x"); e.comment=b"hi".to_vec(); e.cextra=ext(0x7777,b"abc");
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a2=ZipArchive::new(Cursor::new(v)).unwrap();
            let f=a2.by_index_raw(0).unwrap();
            format!("comment={:?} extra={:?}", f.comment(), f.extra_data())
        });
        t("raw copy into new_stream", || {
            let mut src={let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("a",st()).unwrap(); w.write_all(b"hello").unwrap(); w.finish_into_readable().unwrap()};
            let mut w=ZipWriter::new_stream(Vec::<u8>::new());
            w.raw_copy_file(src.by_index_raw(0).unwrap()).unwrap();
            w.start_file("b",st()).unwrap(); w.write_all(b"world").unwrap();
            let v=w.finish().unwrap().into_inner();
            // flag bit3, descriptor between a's data and b's header?
            let flags=u16::from_le_bytes([v[6],v[7]]);
            let dsig=v.windows(4).filter(|w|*w==[0x50,0x4b,7,8]).count();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            format!("a local flags={flags:#x} descriptors in file={dsig} (2 entries); a={} b={}", read_all(&mut a,"a"), read_all(&mut a,"b"))
        });
    }
    if on("append") {
        t("append: Info-ZIP unicode path entry", || {
            let raw=b"caf\x82.txt"; let mut b=vec![1u8]; b.extend(crc(raw).to_le_bytes()); b.extend("café.txt".as_bytes());
            let mut e=E::new("x",b"data"); e.name=raw.to_vec(); e.cextra=ext(0x7075,&b); e.lextra=e.cextra.clone();
            let v=build(&[e]);
            let a=ZipArchive::new(Cursor::new(v.clone())).unwrap(); let names:Vec<String>=a.file_names().map(|s|s.to_string()).collect();
            let mut w=ZipWriter::new_append(Cursor::new(v)).unwrap();
            w.start_file("new",st()).unwrap(); w.write_all(b"n").unwrap();
            let out=w.finish().unwrap().into_inner();
            std::fs::write("/work/tmp/append_unicode.zip",&out).unwrap();
            format!("before names={names:?}; reopened={:?}", ZipArchive::new(Cursor::new(out)).map(|a|a.file_names().map(|s|s.to_string()).collect::<Vec<_>>()))
        });
        t("append: long CP437 comment expands past u16", || {
            let mut e=E::new("x",b"data"); e.comment=vec![0xB0;30000];
            let mut w=ZipWriter::new_append(Cursor::new(build(&[e]))).unwrap();
            w.start_file("new",st()).unwrap(); w.write_all(b"n").unwrap();
            let r=w.finish().map(|_|());
            format!("finish={r:?}")
        });
        t("append: CP437 comment round trip", || {
            let mut e=E::new("x",b"data"); e.comment=b"caf\x82".to_vec();
            let mut a0=ZipArchive::new(Cursor::new(build(&[E::new("y",b"")]))).unwrap(); let _=&mut a0;
            let mut w=ZipWriter::new_append(Cursor::new(build(&[e]))).unwrap();
            w.start_file("new",st()).unwrap(); w.write_all(b"n").unwrap();
            let out=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(out)).unwrap();
            let c=a.by_name("x").unwrap().comment().to_string();
            format!("comment after append={c:?} (original café)")
        });
        t("append: streaming reader sees?", || {
            let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("old",st()).unwrap(); w.write_all(b"o").unwrap();
            let v=w.finish().unwrap().into_inner(); let n0=v.len();
            let mut w=ZipWriter::new_append(Cursor::new(v)).unwrap(); w.start_file("new",st()).unwrap(); w.write_all(b"n").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut c=Cursor::new(v.clone()); let mut names=vec![];
            while let Ok(Some(f))=read_zipfile_from_stream(&mut c){names.push(f.name().to_string());}
            std::fs::write("/work/tmp/append_stream.zip",&v).unwrap();
            let cdsigs=v.windows(4).filter(|w|*w==[0x50,0x4b,1,2]).count();
            format!("orig_len={n0} new_len={} central headers in file={cdsigs} stream names={names:?}", v.len())
        });
        t("zip64 comment roundtrip", || {
            let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("a",st()).unwrap(); w.write_all(b"o").unwrap();
            w.set_zip64_comment(Some("zip64 hello"));
            let v=w.finish().unwrap().into_inner();
            let a=ZipArchive::new(Cursor::new(v)).unwrap();
            format!("{:?}", a.zip64_comment().map(|c|String::from_utf8_lossy(c).to_string()))
        });
    }
    if on("merge") {
        t("merge duplicate names", || {
            let mk=|n:&str,c:&[u8]|{let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file(n,st()).unwrap(); w.write_all(c).unwrap(); w.finish_into_readable().unwrap()};
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("a",st()).unwrap(); w.write_all(b"first").unwrap();
            w.merge_archive(mk("a",b"second")).unwrap();
            let isw=w.is_writing_file();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            format!("is_writing_file after merge={isw}; len={} a={}", a.len(), read_all(&mut a,"a"))
        });
        t("write after merge", || {
            let mk=|n:&str,c:&[u8]|{let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file(n,st()).unwrap(); w.write_all(c).unwrap(); w.finish_into_readable().unwrap()};
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.merge_archive(mk("a",b"data")).unwrap();
            let r=w.write(b"oops").map_err(|e|e.to_string());
            w.start_file("b",st()).unwrap(); w.write_all(b"bb").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            format!("write={r:?} a={} b={}", read_all(&mut a,"a"), read_all(&mut a,"b"))
        });
    }
    if on("stream") {
        t("new_stream + AES", || {
            let mut w=ZipWriter::new_stream(Vec::<u8>::new());
            let s=w.start_file("a",st().with_aes_encryption(AesMode::Aes256,"pw")).map_err(|e|e.to_string());
            let wr=w.write_all(b"hello world").map_err(|e|e.to_string());
            let f=w.finish().map(|_|()).map_err(|e|e.to_string());
            format!("start={s:?} write={wr:?} finish={f:?}")
        });
        t("new_stream abort then continue", || {
            let mut w=ZipWriter::new_stream(Vec::<u8>::new());
            w.start_file("a",st()).unwrap(); w.write_all(b"AAAA").unwrap();
            w.start_file("b",st()).unwrap(); w.write_all(b"BBBBBBBBBBBB").unwrap();
            let r=w.abort_file().map_err(|e|e.to_string());
            w.start_file("c",st()).unwrap(); w.write_all(b"CCCC").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let names:Vec<String>=a.file_names().map(|s|s.to_string()).collect();
            let mut s=String::new(); for n in &names { s+=&format!(" {n}={} ", read_all(&mut a,n)); }
            format!("abort={r:?} names={names:?}{s}")
        });
        t("new_stream directory flag", || {
            let mut w=ZipWriter::new_stream(Vec::<u8>::new());
            w.add_directory("d",SimpleFileOptions::default()).unwrap();
            w.start_file("f",st()).unwrap(); w.write_all(b"x").unwrap();
            let v=w.finish().unwrap().into_inner();
            let flags=u16::from_le_bytes([v[6],v[7]]);
            let dsig=v.windows(4).filter(|w|*w==[0x50,0x4b,7,8]).count();
            std::fs::write("/work/tmp/stream_dir.zip",&v).unwrap();
            format!("dir flags={flags:#x} descriptors={dsig} (2 entries, 1 file)")
        });
        t("new_stream large_file >4GiB", || {
            let mut w=ZipWriter::new_stream(std::io::sink());
            w.start_file("a",st().large_file(true)).unwrap();
            let buf=vec![0u8;1<<24]; for _ in 0..257 { w.write_all(&buf).unwrap(); }
            let f=w.finish().map(|_|()).map_err(|e|e.to_string());
            format!("wrote {} bytes; finish={f:?}", 257u64<<24)
        });
        t("new_stream small large_file", || {
            let mut w=ZipWriter::new_stream(Vec::<u8>::new());
            w.start_file("a",st().large_file(true)).unwrap(); w.write_all(b"hello").unwrap();
            w.start_file("b",st()).unwrap(); w.write_all(b"world").unwrap();
            let v=w.finish().unwrap().into_inner();
            std::fs::write("/work/tmp/stream_large.zip",&v).unwrap();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            {let x=read_all(&mut a,"a"); let sz=a.by_index_raw(0).unwrap().size(); format!("a={} size={} b={}", x, sz, read_all(&mut a,"b"))}
        });
    }
    if on("misc") {
        t("long file name", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            format!("{:?}", w.start_file("a".repeat(70000),st()).map_err(|e|e.to_string()))
        });
        t("long comment", || {
            let mut w=ZipWriter::new(Cursor::new(vec![])); w.set_comment("c".repeat(70000));
            w.start_file("a",st()).unwrap();
            format!("{:?}", w.finish().map(|_|()).map_err(|e|e.to_string()))
        });
        t("extra poison", || {
            let mut o=FullFileOptions::default();
            let r1=o.add_extra_data(0x000a,vec![0u8;3].into_boxed_slice(),false).map_err(|e|e.to_string());
            let r2=o.add_extra_data(0x7777,vec![1u8,2].into_boxed_slice(),false).map_err(|e|e.to_string());
            format!("bad={r1:?} then good={r2:?}")
        });
        t("levels", || {
            let mut s=String::new();
            for (m,l) in [(CompressionMethod::Deflated,0i64),(CompressionMethod::Bzip2,0),(CompressionMethod::Zstd,-7),(CompressionMethod::Zstd,0),(CompressionMethod::Zstd,22)] {
                let mut w=ZipWriter::new(Cursor::new(vec![]));
                let r=w.start_file("a",SimpleFileOptions::default().compression_method(m).compression_level(Some(l))).map_err(|e|e.to_string());
                s+=&format!("{m:?}@{l}={r:?} ");
            }
            s
        });
        t("start_file with options from AES entry", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", st().with_aes_encryption(AesMode::Aes128,"pw")).unwrap(); w.write_all(b"secret!").unwrap();
            let bytes=w.finish().unwrap().into_inner();
            let mut src=ZipArchive::new(Cursor::new(bytes)).unwrap();
            let o=src.by_index_raw(0).unwrap().options();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("copy",o).unwrap(); w.write_all(b"PLAINTEXT-PAYLOAD-XYZ-0123456789").unwrap();
            let v=w.finish().unwrap().into_inner();
            let contains=v.windows(21).any(|w|w==b"PLAINTEXT-PAYLOAD-XYZ");
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let enc=a.by_index_raw(0).unwrap().encrypted();
            let dec=a.by_index_decrypt(0,b"pw").map(|_|()).map_err(|e|e.to_string());
            format!("plaintext in bytes={contains} encrypted flag={enc} decrypt={dec:?}")
        });
        t("roundtrip matrix", || {
            let mut out=String::new();
            let methods=[CompressionMethod::Stored,CompressionMethod::Deflated,CompressionMethod::Bzip2,CompressionMethod::Zstd,CompressionMethod::Xz,CompressionMethod::Ppmd];
            let datas:Vec<Vec<u8>>=vec![vec![],vec![7],b"hello hello hello".to_vec(),(0..200000u32).map(|i|(i.wrapping_mul(i)>>3) as u8).collect()];
            for m in methods { for (di,d) in datas.iter().enumerate() {
              for enc in 0..3 { for stream in [false,true] {
                let run=||->Result<Vec<u8>,String>{
                    let o=SimpleFileOptions::default().compression_method(m);
                    let o2=match enc {0=>o,1=>o.with_aes_encryption(AesMode::Aes192,"pw"),_=>o.with_deprecated_encryption(b"pw")};
                    let v=if stream {
                        let mut w=ZipWriter::new_stream(Vec::<u8>::new());
                        w.start_file("n",o2).map_err(|e|format!("start {e}"))?; w.write_all(d).map_err(|e|format!("write {e}"))?;
                        w.finish().map_err(|e|format!("finish {e}"))?.into_inner()
                    } else {
                        let mut w=ZipWriter::new(Cursor::new(vec![]));
                        w.start_file("n",o2).map_err(|e|format!("start {e}"))?; w.write_all(d).map_err(|e|format!("write {e}"))?;
                        w.finish().map_err(|e|format!("finish {e}"))?.into_inner()
                    };
                    let mut a=ZipArchive::new(Cursor::new(v)).map_err(|e|format!("open {e}"))?;
                    let mut f= if enc==0 {a.by_index(0)} else {a.by_index_decrypt(0,b"pw")}.map_err(|e|format!("idx {e}"))?;
                    let mut r=vec![]; f.read_to_end(&mut r).map_err(|e|format!("read {e}"))?; Ok(r)
                };
                let res=catch_unwind(AssertUnwindSafe(run));
                match res { Ok(Ok(r)) if &r==d => {}, Ok(Ok(_))=>out+=&format!("\n  {m:?} d{di} enc{enc} stream={stream}: MISMATCH"), Ok(Err(e))=>out+=&format!("\n  {m:?} d{di} enc{enc} stream={stream}: {e}"), Err(_)=>out+=&format!("\n  {m:?} d{di} enc{enc} stream={stream}: PANIC") }
              }}
            }}
            out
        });
        t("65535 entries", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            for i in 0..65535 { w.start_file(format!("{i}"),st()).unwrap(); }
            let v=w.finish().unwrap().into_inner();
            format!("{:?}", ZipArchive::new(Cursor::new(v)).map(|a|a.len()).map_err(|e|e.to_string()))
        });
        t("65536 entries", || {
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            for i in 0..65536 { w.start_file(format!("{i}"),st()).unwrap(); }
            let v=w.finish().unwrap().into_inner();
            format!("{:?}", ZipArchive::new(Cursor::new(v)).map(|a|a.len()).map_err(|e|e.to_string()))
        });
        t("by_path dir", || {
            let mut w=ZipWriter::new(Cursor::new(vec![])); w.add_directory("d",SimpleFileOptions::default()).unwrap();
            w.start_file("./dot",st()).unwrap();
            let mut a=w.finish_into_readable().unwrap();
            format!("by_path d/={:?} by_name d/={:?} by_path ./dot={:?} by_name ./dot={:?}", a.by_path("d/").map(|_|()).map_err(|e|e.to_string()), a.by_name("d/").map(|_|()).map_err(|e|e.to_string()), a.by_path("./dot").map(|_|()).map_err(|e|e.to_string()), a.by_name("./dot").map(|_|()).map_err(|e|e.to_string()))
        });
        t("duplicates", || {
            let mut a=ZipArchive::new(Cursor::new(build(&[E::new("a",b"1"),E::new("b",b"2"),E::new("a",b"3")]))).unwrap();
            format!("len={} a={} order={:?}", a.len(), read_all_slice(&mut a), a.file_names().map(|s|s.to_string()).collect::<Vec<_>>())
        });
    }
}
fn read_all_slice(a:&mut ZipArchive<Cursor<Vec<u8>>>)->String{ read_all(a,"a") }
