use std::io::{Cursor, Read, Write};
use zip::{ZipWriter,ZipArchive,CompressionMethod,AesMode};
use zip::write::SimpleFileOptions;
use zip::unstable::stream::{ZipStreamReader,ZipStreamVisitor,ZipStreamFileMetadata};
use std::path::Path;
fn options() -> SimpleFileOptions { SimpleFileOptions::default().compression_method(CompressionMethod::Stored) }
fn archive(data: &[u8], opts: SimpleFileOptions) -> Vec<u8> {
 let mut w=ZipWriter::new(Cursor::new(Vec::new())); w.start_file("file",opts).unwrap(); w.write_all(data).unwrap(); w.finish().unwrap().into_inner()
}
fn central(b: &[u8]) -> usize { b.windows(4).rposition(|w|w==b"PK\x01\x02").unwrap() }
fn encrypted(method: CompressionMethod, data: &[u8]) -> Vec<u8> {
 let mut w=ZipWriter::new(Cursor::new(Vec::new())); w.start_file("file", options().compression_method(method).with_aes_encryption(AesMode::Aes128,"password")).unwrap(); w.write_all(data).unwrap(); w.finish().unwrap().into_inner()
}
fn read_encrypted(b: Vec<u8>) -> std::io::Result<Vec<u8>> { let mut a=ZipArchive::new(Cursor::new(b)).unwrap(); let mut v=Vec::new(); a.by_index_decrypt(0,b"password").unwrap().read_to_end(&mut v)?; Ok(v) }
struct Counts(usize,usize);
impl ZipStreamVisitor for Counts {
 fn visit_file<R:Read>(&mut self,_f:&mut zip::read::ZipFile<'_,R>) -> zip::result::ZipResult<()> { self.0+=1; Ok(()) }
 fn visit_additional_metadata(&mut self,_f:&ZipStreamFileMetadata) -> zip::result::ZipResult<()> { self.1+=1; Ok(()) }
}
fn main() {
 let mut w=ZipWriter::new(Cursor::new(Vec::new())); w.start_file("file",options().large_file(true)).unwrap(); w.write_all(b"hello").unwrap(); let mut a=w.finish_into_readable().unwrap(); let mut f=a.by_index(0).unwrap(); let mut out=Vec::new(); println!("large readable size={} compressed={} read={:?}",f.size(),f.compressed_size(),f.read_to_end(&mut out)); println!("large output len={}",out.len());
 let b=archive(b"hello",options()); let mut c=Counts(0,0); let r=ZipStreamReader::new(Cursor::new(b)).visit(&mut c); println!("stream visit={:?} files={} metadata={}",r,c.0,c.1);
 let mut w=ZipWriter::new(Cursor::new(Vec::new())); w.set_zip64_comment(Some("zip64 comment")); w.start_file("file",options()).unwrap(); w.write_all(b"hello").unwrap(); let a=ZipArchive::new(w.finish().unwrap()).unwrap(); println!("zip64 comment={:?}",a.zip64_comment());
 let dest=Path::new("/work/tmp/extract_probe"); std::fs::create_dir_all(dest).unwrap();
 let mut w=ZipWriter::new(Cursor::new(Vec::new())); w.add_directory("target",options()).unwrap(); w.add_symlink("link","target",options()).unwrap(); w.start_file("link/file",options()).unwrap(); w.write_all(b"hello").unwrap(); let mut a=ZipArchive::new(w.finish().unwrap()).unwrap(); println!("safe link extract={:?}",a.extract(dest));
 let dest=Path::new("/work/tmp/permissions_probe"); let mut w=ZipWriter::new(Cursor::new(Vec::new())); w.add_directory("private",options().unix_permissions(0o500)).unwrap(); let mut a=ZipArchive::new(w.finish().unwrap()).unwrap(); a.extract(dest).unwrap(); use std::os::unix::fs::PermissionsExt; println!("directory mode={:o}",std::fs::metadata(dest.join("private")).unwrap().permissions().mode()&0o777);
 let mut b=encrypted(CompressionMethod::Deflated,b"hello"); let cd=central(&b); let size=u32::from_le_bytes(b[cd+20..cd+24].try_into().unwrap()) as usize; let start=30+4+11; println!("AES compressed size={size} data_start={start}"); b[start+size-1]^=0x80; println!("bad auth normal={:?}",read_encrypted(b.clone())); b[cd+20..cd+24].copy_from_slice(&100000u32.to_le_bytes()); println!("bad auth inflated size={:?}",read_encrypted(b.clone()));
 for i in start+10..start+size-10 { for bit in 0..8 { let mut x=b.clone(); x[i]^=1<<bit; if let Ok(out)=read_encrypted(x) { if out!=b"hello" { println!("AES altered plaintext accepted byte={} bit={} out={:?}",i,bit,out); break; } } } }
 let b=encrypted(CompressionMethod::Stored,b""); let cd=central(&b); let mut bad=b.clone(); bad[cd-1]^=1; println!("empty AES tampered auth={:?}",read_encrypted(bad));
 let mut b=encrypted(CompressionMethod::Stored,b"hello"); let cd=central(&b); b[cd+20..cd+24].copy_from_slice(&0u32.to_le_bytes()); let panic=std::panic::catch_unwind(|| { let mut a=ZipArchive::new(Cursor::new(b)).unwrap(); a.get_aes_verification_key_and_salt(0).map(|_|()) }); println!("short AES panic={:?}",panic);
 let mut w=ZipWriter::new_stream(Vec::new()); w.start_file("file",options().with_aes_encryption(AesMode::Aes128,"password")).unwrap(); w.write_all(b"hello").unwrap(); println!("AES streaming finish={:?}",w.finish().map(|_|()));
}
