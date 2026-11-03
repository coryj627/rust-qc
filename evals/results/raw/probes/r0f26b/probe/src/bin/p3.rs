use probe::*;
use std::io::{Cursor, Read, Write};
use std::time::Instant;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions, CompressionMethod};
struct BW{v:Vec<u8>,cur:u64,n:u32}
impl BW{fn new()->Self{BW{v:vec![],cur:0,n:0}} fn put(&mut self,c:u32,bits:u32){self.cur|=(c as u64)<<self.n;self.n+=bits;while self.n>=8{self.v.push(self.cur as u8);self.cur>>=8;self.n-=8;}} fn done(mut self)->Vec<u8>{if self.n>0{self.v.push(self.cur as u8);}self.v}}
fn hwm()->String{std::fs::read_to_string("/proc/self/status").unwrap().lines().filter(|l|l.starts_with("VmHWM")||l.starts_with("VmPeak")).collect::<Vec<_>>().join(" ")}
fn eocd_bomb(m:usize,n:usize)->Vec<u8>{
    // m valid central headers, then n EOCD records that each claim m+1 entries.
    let mut out=vec![];
    for i in 0..m { 
        out.extend(0x02014b50u32.to_le_bytes()); out.extend(0x031eu16.to_le_bytes()); out.extend(20u16.to_le_bytes()); out.extend([0u8;4]);
        out.extend([0u8;4]); out.extend([0u8;12]); out.extend(1u16.to_le_bytes()); out.extend([0u8;4]); out.extend([0u8;4]); out.extend([0u8;4]); out.extend(0u32.to_le_bytes());
        out.push(b'a'+(i%26) as u8);
    }
    let sz=out.len() as u32;
    for _ in 0..n {
        out.extend(0x06054b50u32.to_le_bytes()); out.extend([0u8;4]);
        out.extend(((m+1) as u16).to_le_bytes()); out.extend(((m+1) as u16).to_le_bytes());
        out.extend(sz.to_le_bytes()); out.extend(0u32.to_le_bytes()); out.extend(0u16.to_le_bytes());
    }
    out
}
fn main(){
    let w=std::env::args().nth(1).unwrap();
    match w.as_str(){
      "eocd"=>{ for (m,n) in [(1000,1000),(2000,2000),(4000,4000),(8000,8000)] {
            let v=eocd_bomb(m,n); let l=v.len(); let t=Instant::now();
            let r=ZipArchive::new(Cursor::new(v)).map(|a|a.len());
            println!("eocd bomb m={m} n={n} size={l} -> {:?} in {:?}",r.map_err(|e|e.to_string()),t.elapsed());
      }}
      "overlap"=>{ for n in [20000usize,40000,80000,160000]{
            let mut w=ZipWriter::new(Cursor::new(Vec::with_capacity(n*150))); let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for i in 0..n { w.start_file(format!("{i}"),o).unwrap(); w.write_all(b"x").unwrap(); }
            let mut a=ZipArchive::new(Cursor::new(w.finish().unwrap().into_inner())).unwrap();
            let t=Instant::now(); let r=a.has_overlapping_files().map_err(|e|e.to_string());
            println!("overlap n={n} -> {r:?} in {:?}",t.elapsed());
      }}
      "shrink"=>{ // repeated partial-clear control codes
            for pairs in [20000usize,200000,2000000]{
              let mut b=BW::new(); b.put(b'a' as u32,9); for _ in 0..pairs { b.put(256,9); b.put(2,9);} b.put(b'b' as u32,9);
              let d=b.done(); let l=d.len();
              let mut e=E::new("x",&d); e.method=1; e.uncomp=Some(2);
              let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
              let t=Instant::now(); let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
              println!("shrink pairs={pairs} payload={l} -> {r:?} in {:?}",t.elapsed());
            }
      }
      "symabort"=>{ let mut e=E::new("l",b"target").unix(0o120777); e.uncomp=Some(u32::MAX); e.cextra=ext(1,&(1u64<<62).to_le_bytes());
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap(); println!("{:?}",a.extract("/work/tmp/symabort")); }
      "dict"=>{ for dict in [1u32<<30, (1u32<<31)-16]{
            let mut d=vec![0x10,0x02,5,0, 0x5d]; d.extend(dict.to_le_bytes()); d.extend([0u8;64]);
            let mut e=E::new("x",&d); e.method=14; e.uncomp=Some(100);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let t=Instant::now(); let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            println!("lzma dict={dict} file~150B -> {r:?} in {:?} {}",t.elapsed(),hwm());
      }}
      "ppmd"=>{ let mut d=vec![0xff,0x0f]; d.extend([0u8;16]); let mut e=E::new("x",&d); e.method=98; e.uncomp=Some(1);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let t=Instant::now(); let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s)).map_err(|e|e.to_string());
            println!("ppmd max params -> {r:?} in {:?} {}",t.elapsed(),hwm());
      }
      "backslash"=>{ let dd="/work/tmp/bs"; std::fs::create_dir_all(dd).unwrap();
            let es=[E::new("d\\",b""),E::new("d\\f.txt",b"x")];
            let mut a=ZipArchive::new(Cursor::new(build(&es))).unwrap();
            let r=a.extract(dd).map_err(|e|e.to_string());
            println!("{r:?} {:?}", std::fs::read_dir(dd).unwrap().map(|e|e.unwrap().file_name()).collect::<Vec<_>>());
      }
      "misc"=>{
            // decompressed_size with descriptor
            let mut e=E::new("x",b"hello"); e.flags=8;
            let a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            println!("decompressed_size w/ data descriptor flag = {:?}", a.decompressed_size());
            // cp437 table
            let mut names=vec![]; for b in 0x80u16..=0xff { let mut e=E::new("",b""); e.name=vec![b as u8]; names.push(e); }
            let a=ZipArchive::new(Cursor::new(build(&names))).unwrap();
            let s:Vec<String>=a.file_names().map(|s|s.to_string()).collect();
            println!("CP437:{}",s.join(""));
            // bad date with data-descriptor zipcrypto: decrypt with valid password
      }
      "implode_append"=>{
            let b=std::fs::read("/work/crate/tests/data/legacy/implode.zip").unwrap();
            let mut a=ZipArchive::new(Cursor::new(b.clone())).unwrap();
            let n=a.name_for_index(0).unwrap().to_string();
            let mut s=vec![]; let r0={let mut f=a.by_index(0).unwrap(); println!("flags={}",{use zip::HasZipMetadata; f.get_metadata().flags}); f.read_to_end(&mut s).map_err(|e|e.to_string())};
            let mut w=ZipWriter::new_append(Cursor::new(b)).unwrap();
            w.start_file("zzz",SimpleFileOptions::default()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a2=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut s2=vec![]; let r1={let mut f=a2.by_name(&n).unwrap(); println!("flags after append={}",{use zip::HasZipMetadata; f.get_metadata().flags}); f.read_to_end(&mut s2).map_err(|e|e.to_string())};
            println!("{n}: before={r0:?} after={r1:?} equal={}",s==s2);
      }
      "append_misc"=>{
            // name bytes valid UTF-8 but no flag; invalid date
            let mut e=E::new("",b"x"); e.name=vec![0xC3,0xA9]; e.date=0; e.time=0x6000;
            let a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let before=a.name_for_index(0).unwrap().to_string();
            let mut e=E::new("",b"x"); e.name=vec![0xC3,0xA9]; e.date=0; e.time=0x6000;
            let mut w=ZipWriter::new_append(Cursor::new(build(&[e]))).unwrap();
            w.start_file("zzz",SimpleFileOptions::default()).unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut a2=ZipArchive::new(Cursor::new(v)).unwrap();
            let after=a2.name_for_index(0).unwrap().to_string();
            println!("name before={before:?} after={after:?} date after={:?}", a2.by_index_raw(0).unwrap().last_modified());
            // read-only re-extract
            let dd="/work/tmp/roex"; std::fs::create_dir_all(dd).unwrap();
            let v=build(&[E::new("ro",b"x").unix(0o100444)]);
            let r1=ZipArchive::new(Cursor::new(v.clone())).unwrap().extract(dd).map_err(|e|e.to_string());
            let r2=ZipArchive::new(Cursor::new(v)).unwrap().extract(dd).map_err(|e|format!("{e:?}"));
            println!("first={r1:?} second={r2:?}");
      }
      "zc_baddate"=>{
            use zip::unstable::write::FileOptionsExt;
            // ZipCrypto entry with data-descriptor bit whose check byte is the time high byte, date invalid.
            // Build plaintext header manually by re-encrypting: use crate writer to encrypt then patch not possible; emulate keys here.
            let pw=b"pw"; let time:u16=0x6000; let data=b"hello";
            let tbl:Vec<u32>=(0..256u32).map(|i|{let mut c=i;for _ in 0..8{c=if c&1!=0{0xedb88320^(c>>1)}else{c>>1};}c}).collect();
            let mut k=[0x12345678u32,0x23456789,0x34567890];
            let upd=|k:&mut [u32;3],b:u8,tbl:&Vec<u32>|{k[0]=(k[0]>>8)^tbl[((k[0]^b as u32)&0xff) as usize];k[1]=(k[1].wrapping_add(k[0]&0xff)).wrapping_mul(0x08088405).wrapping_add(1);k[2]=(k[2]>>8)^tbl[((k[2]^(k[1]>>24))&0xff) as usize];};
            for b in pw { upd(&mut k,*b,&tbl); }
            let mut ct=vec![];
            let mut hdr=[0u8;12]; hdr[11]=(time>>8) as u8; hdr[10]=time as u8;
            for b in hdr.iter().chain(data.iter()) { let t=(k[2]|2) as u16 as u32; let s=((t*(t^1))>>8) as u8; ct.push(b^s); upd(&mut k,*b,&tbl); }
            for (date,label) in [(0x5021u16,"valid date"),(0u16,"date field 0")] {
                let mut e=E::new("z",&ct); e.flags=1|8; e.crc=Some(crc(data)); e.uncomp=Some(data.len() as u32); e.time=time; e.date=date;
                let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
                let mut s=vec![]; let r=a.by_index_decrypt(0,pw).map(|mut f|f.read_to_end(&mut s).map_err(|e|e.to_string())).map_err(|e|e.to_string());
                println!("{label}: {r:?} {:?}",String::from_utf8_lossy(&s));
            }
            let _=SimpleFileOptions::default().with_deprecated_encryption(b"x");
      }
      "xz_empty"=>{
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("e",SimpleFileOptions::default().compression_method(CompressionMethod::Xz)).unwrap();
            let v=w.finish().unwrap().into_inner();
            std::fs::write("/work/tmp/xz_empty.zip",&v).unwrap();
            let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
            let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s).map_err(|e|e.to_string())).map_err(|e|e.to_string());
            println!("{r:?}");
      }
      "xzidx"=>{
            let mut d=vec![0xfd,0x37,0x7a,0x58,0x5a,0x00,0x00,0x04,0xe6,0xd6,0xb4,0x46, 0x00];
            // number-of-records VLI = 2^56
            d.extend([0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x01]); d.extend([0u8;40]);
            let mut e=E::new("x",&d); e.method=95; e.uncomp=Some(1);
            let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
            let mut s=vec![]; let r=a.by_index(0).map(|mut f|f.read_to_end(&mut s).map_err(|e|e.to_string())).map_err(|e|e.to_string());
            println!("xz huge index: {r:?}");
      }
      "ppmdp"=>{
            for rm in 0u16..16 { for ord in [0u16,1,2,15] {
              let p=ord|(0<<4)|(rm<<12); let mut d=p.to_le_bytes().to_vec(); d.extend([0u8;16]);
              let mut e=E::new("x",&d); e.method=98; e.uncomp=Some(1);
              let mut a=ZipArchive::new(Cursor::new(build(&[e]))).unwrap();
              let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{let mut s=vec![]; a.by_index(0).map(|mut f|f.read_to_end(&mut s).map_err(|e|e.to_string())).map_err(|e|e.to_string())}));
              if r.is_err(){ println!("ppmd PANIC rm={rm} ord={ord}"); }
            }} println!("ppmd params done");
      }
      "align"=>{
            let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for al in [2u16,3,4,7,8,512,4096,65535]{ for lf in [false,true]{ for aes in [false,true]{
              let mut w=ZipWriter::new(Cursor::new(vec![0u8;0]));
              let mut oo=o.with_alignment(al).large_file(lf); if aes { oo=oo.with_aes_encryption(zip::AesMode::Aes128,"p"); }
              let r=w.start_file("abc",oo).and_then(|_|{w.write_all(b"hello").map_err(Into::into)}); if r.is_err(){println!("al={al} lf={lf} aes={aes} {r:?}");continue;}
              let v=w.finish().unwrap().into_inner(); let mut a=ZipArchive::new(Cursor::new(v)).unwrap();
              let ds={let f=a.by_index_raw(0).unwrap(); f.data_start()}; let mut s=vec![];
              let rr= if aes {a.by_index_decrypt(0,b"p").and_then(|mut f|f.read_to_end(&mut s).map_err(Into::into))} else {a.by_index(0).and_then(|mut f|f.read_to_end(&mut s).map_err(Into::into))};
              if ds%al as u64!=0 || rr.is_err() { println!("al={al} lf={lf} aes={aes} ds={ds} rr={rr:?}"); }
            }}} println!("align done");
      }
      "streamlarge"=>{
            let mut w=ZipWriter::new(Cursor::new(vec![])); let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            w.start_file("a",o.large_file(true)).unwrap(); w.write_all(b"hello").unwrap(); w.start_file("b",o).unwrap(); w.write_all(b"world").unwrap();
            let v=w.finish().unwrap().into_inner(); let mut c=Cursor::new(v); let mut out=vec![];
            while let Some(mut f)=zip::read::read_zipfile_from_stream(&mut c).unwrap(){ let mut s=String::new(); let r=f.read_to_string(&mut s).map_err(|e|e.to_string()); out.push((f.name().to_string(),r,s)); }
            println!("{out:?}");
      }
      "write_set"=>{
            let mut sets:Vec<(&str,Vec<u8>)>=vec![];
            let mut b=vec![1u8]; b.extend(0x12345678u32.to_le_bytes()); b.extend("é.txt".as_bytes()); let mut e=E::new("e.txt",b"x"); e.cextra=ext(0x7075,&b); sets.push(("stale7075",build(&[e])));
            let raw=b"caf\x82"; let mut b=vec![1u8]; b.extend(crc(raw).to_le_bytes()); b.extend("café".as_bytes()); let mut e=E::new("e.txt",b"x"); e.comment=raw.to_vec(); e.cextra=ext(0x6375,&b); sets.push(("c6375",build(&[e])));
            let mut b=vec![0u8;4]; b.extend(1u16.to_le_bytes()); b.extend(24u16.to_le_bytes()); b.extend([1u8;24]); b.extend(2u16.to_le_bytes()); b.extend(0u16.to_le_bytes()); let mut e=E::new("e.txt",b"x"); e.cextra=ext(0x000a,&b); sets.push(("ntfs36",build(&[e])));
            let mut b=vec![1u8]; b.extend([0u8;8]); let mut e=E::new("e.txt",b"x"); e.cextra=ext(0x5455,&b); sets.push(("ut9",build(&[e])));
            let mut v=build_c(&[E::new("a",b"x")],b"0123456789"); v.truncate(v.len()-5); sets.push(("trunccomment",v));
            for (n,v) in sets { std::fs::write(format!("/work/tmp/set_{n}.zip"),v).unwrap(); }
            use zip::unstable::write::FileOptionsExt;
            let mut w=ZipWriter::new_stream(Vec::<u8>::new());
            w.start_file("s", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_deprecated_encryption(b"pw")).unwrap(); w.write_all(b"hello").unwrap();
            std::fs::write("/work/tmp/stream_zc.zip",w.finish().unwrap().into_inner()).unwrap();
            let mut w=ZipWriter::new(Cursor::new(vec![]));
            w.start_file("s", SimpleFileOptions::default().compression_method(CompressionMethod::Stored).with_deprecated_encryption(b"pw")).unwrap(); w.write_all(b"hello").unwrap();
            std::fs::write("/work/tmp/seek_zc.zip",w.finish().unwrap().into_inner()).unwrap();
      }
      "sparse"=>{
            use std::io::{Seek,SeekFrom}; use std::fs::{File,OpenOptions};
            struct SW(File);
            impl Write for SW{ fn write(&mut self,b:&[u8])->std::io::Result<usize>{ if b.len()>=1<<20 && b.iter().all(|x|*x==0) { let p=self.0.stream_position()?; let e=p+b.len() as u64; if self.0.metadata()?.len()<e {self.0.set_len(e)?;} self.0.seek(SeekFrom::Start(e))?; Ok(b.len()) } else { self.0.write(b) } } fn flush(&mut self)->std::io::Result<()>{self.0.flush()} }
            impl Seek for SW{ fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)} }
            impl Read for SW{ fn read(&mut self,b:&mut [u8])->std::io::Result<usize>{self.0.read(b)} }
            let path="/target/sparse.zip"; let _=std::fs::remove_file(path);
            let f=OpenOptions::new().create(true).read(true).write(true).open(path).unwrap();
            let mut w=ZipWriter::new(SW(f)); let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            w.start_file("big",o.large_file(true)).unwrap(); let z=vec![0u8;1<<24]; for _ in 0..260 { w.write_all(&z).unwrap(); }
            w.start_file("after",o).unwrap(); w.write_all(b"after-data").unwrap();
            let sw=w.finish().unwrap(); drop(sw);
            fn dump(tag:&str,p:&str){ let mut a=ZipArchive::new(File::open(p).unwrap()); match &mut a { Err(e)=>println!("{tag}: open err {e}"), Ok(a)=>{ for i in 0..a.len(){ let n=a.name_for_index(i).unwrap().to_string(); let (hs,sz,cs)={match a.by_index_raw(i){Ok(f)=>(f.header_start(),f.size(),f.compressed_size()),Err(e)=>{println!("{tag}: {n} raw err {e}");continue;}}}; let rd=if sz<1<<20 {let mut s=vec![]; a.by_index(i).and_then(|mut f|f.read_to_end(&mut s).map_err(Into::into)).map_err(|e|e.to_string())}else{Ok(0)}; println!("{tag}: {n} header_start={hs} size={sz} comp={cs} read={rd:?}"); } } } }
            dump("original",path);
            let f=OpenOptions::new().read(true).write(true).open(path).unwrap();
            let mut w=ZipWriter::new_append(SW(f)).unwrap();
            w.start_file("new",o).unwrap(); w.write_all(b"new-data").unwrap(); drop(w.finish().unwrap());
            dump("after new_append",path);
            // merge a small archive after >4GiB of data
            let path2="/target/sparse2.zip"; let _=std::fs::remove_file(path2);
            let f=OpenOptions::new().create(true).read(true).write(true).open(path2).unwrap();
            let mut w=ZipWriter::new(SW(f));
            w.start_file("big",o.large_file(true)).unwrap(); for _ in 0..260 { w.write_all(&z).unwrap(); }
            let small={let mut s=ZipWriter::new(Cursor::new(vec![])); s.start_file("m",o).unwrap(); s.write_all(b"merged").unwrap(); s.finish_into_readable().unwrap()};
            w.merge_archive(small).unwrap(); drop(w.finish().unwrap());
            dump("merged past 4GiB",path2);
      }
      "batch"=>{
            let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            let d="/work/tmp/ux"; std::fs::create_dir_all(d).unwrap();
            let save=|n:&str,v:Vec<u8>|{std::fs::write(format!("{d}/{n}.zip"),v).unwrap();};
            { let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("a",o).unwrap(); w.write_all(b"hello").unwrap(); w.add_directory("d",o).unwrap(); w.add_symlink("l","a",o).unwrap(); save("plain",w.finish().unwrap().into_inner()); }
            { let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("a",o.large_file(true)).unwrap(); w.write_all(b"hello").unwrap(); w.start_file("b",o).unwrap(); w.write_all(b"w").unwrap(); save("large",w.finish().unwrap().into_inner()); }
            { let mut w=ZipWriter::new_stream(Vec::<u8>::new()); w.start_file("a",o).unwrap(); w.write_all(b"hello").unwrap(); w.start_file("b",o).unwrap(); w.write_all(b"w").unwrap(); save("streamfiles",w.finish().unwrap().into_inner()); }
            { let mut w=ZipWriter::new_stream(Vec::<u8>::new()); w.start_file("a",o.large_file(true)).unwrap(); w.write_all(b"hello").unwrap(); w.start_file("b",o).unwrap(); w.write_all(b"w").unwrap(); save("streamlarge",w.finish().unwrap().into_inner()); }
            { let mut src={let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("a",o).unwrap(); w.write_all(b"hello").unwrap(); w.finish_into_readable().unwrap()}; let mut w=ZipWriter::new_stream(Vec::<u8>::new()); w.raw_copy_file(src.by_index_raw(0).unwrap()).unwrap(); w.start_file("b",o).unwrap(); w.write_all(b"w").unwrap(); save("streamraw",w.finish().unwrap().into_inner()); }
            { let mut w=ZipWriter::new(Cursor::new(vec![])); w.start_file("a",o).unwrap(); w.write_all(b"h").unwrap(); w.set_zip64_comment(Some("zc")); save("z64comment",w.finish().unwrap().into_inner()); }
            { let mut w=ZipWriter::new(Cursor::new(vec![])); w.add_directory("",o).unwrap(); let v=w.finish().unwrap().into_inner(); let names:Vec<String>=ZipArchive::new(Cursor::new(v.clone())).unwrap().file_names().map(|s|s.to_string()).collect(); println!("add_directory(\"\") names={names:?}"); let dd=format!("{d}/emptydir"); println!(" extract={:?}",ZipArchive::new(Cursor::new(v)).unwrap().extract(dd).map_err(|e|e.to_string())); }
            // EOCD count handling
            let base=probe::build(&[E::new("a",b"x"),E::new("b",b"y")]); let n=base.len();
            let mut v=base.clone(); v[n-22+8]=0; v[n-22+9]=0; println!("on-disk count 0, total 2: {:?}",ZipArchive::new(Cursor::new(v.clone())).map(|a|a.len()).map_err(|e|e.to_string())); save("count_ondisk0",v);
            let mut v=base.clone(); v[n-22+8]=3; v[n-22+10]=3; println!("count 3 for 2 entries: {:?}",ZipArchive::new(Cursor::new(v.clone())).map(|a|a.len()).map_err(|e|e.to_string())); save("count3",v);
            let mut v=base.clone(); v[n-22+8]=1; v[n-22+10]=1; println!("count 1 for 2 entries: {:?}",ZipArchive::new(Cursor::new(v.clone())).map(|a|a.len()).map_err(|e|e.to_string())); save("count1",v);
            // append metadata
            let mut e=E::new("x",b"d"); e.vmade=0x0a14; e.ext=0x20;
            let mut w=ZipWriter::new_append(Cursor::new(probe::build(&[e]))).unwrap(); w.start_file("n",o).unwrap();
            let v=w.finish().unwrap().into_inner(); let cd=v.windows(4).position(|w|w==[0x50,0x4b,1,2]).unwrap();
            println!("system byte after append (was 10)={}",v[cd+5]);
            // SUPPORTED list
            println!("SUPPORTED={:?}",zip::SUPPORTED_COMPRESSION_METHODS);
      }
      "counts"=>{
            use std::io::{Seek,SeekFrom,BufReader}; use std::rc::Rc; use std::cell::Cell;
            struct C{c:Cursor<Vec<u8>>,r:Rc<Cell<u64>>,s:Rc<Cell<u64>>}
            impl Read for C{fn read(&mut self,b:&mut [u8])->std::io::Result<usize>{self.r.set(self.r.get()+1);self.c.read(b)}}
            impl Seek for C{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.s.set(self.s.get()+1);self.c.seek(p)}}
            let o=SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            let mut w=ZipWriter::new(Cursor::new(vec![])); for i in 0..20000 { w.start_file(format!("{i}"),o).unwrap(); }
            let v=w.finish().unwrap().into_inner(); let (r,s)=(Rc::new(Cell::new(0)),Rc::new(Cell::new(0)));
            let a=ZipArchive::new(BufReader::new(C{c:Cursor::new(v),r:r.clone(),s:s.clone()})).unwrap();
            println!("20000 entries through BufReader: inner reads={} seeks={} len={}",r.get(),s.get(),a.len());
            let mut w=ZipWriter::new_stream(Vec::<u8>::new()); w.start_file("a",o).unwrap(); w.write_all(b"hello").unwrap();
            let v=w.finish().unwrap().into_inner();
            let mut c=Cursor::new(v); println!("own new_stream output through read_zipfile_from_stream: {:?}", zip::read::read_zipfile_from_stream(&mut c).map(|f|f.is_some()).map_err(|e|e.to_string()));
      }
      _=>{}
    }
}
