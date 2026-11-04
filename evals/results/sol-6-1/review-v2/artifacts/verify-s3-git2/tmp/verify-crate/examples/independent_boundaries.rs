use git2::*;
use std::{fs,path::Path};
fn main() {
 match std::env::args().nth(1).as_deref().unwrap() {
 "buf" => { let b=Buf::new(); println!("{}", b.is_empty()); }
 "buf_mut" => { let mut b=Buf::new(); let s:&mut [u8]=&mut b; println!("{}",s.len()); }
 "remote" => { let td=tempfile::tempdir().unwrap(); let bare=Repository::init_bare(td.path().join("remote")).unwrap(); let local=Repository::init(td.path().join("local")).unwrap(); let mut remote=local.remote_anonymous(bare.path().to_str().unwrap()).unwrap(); remote.connect(Direction::Fetch).unwrap(); println!("{}",remote.list().unwrap().len()); }
 "binary" => { let td=tempfile::tempdir().unwrap(); let r=Repository::init(td.path()).unwrap(); let a=r.find_blob(r.blob(b"a\0b").unwrap()).unwrap(); let b=r.find_blob(r.blob(b"a\0c").unwrap()).unwrap(); r.diff_blobs(Some(&a),Some("file"),Some(&b),Some("file"),None,None,Some(&mut |_,bin| { println!("contains_data={} kind={:?}",bin.contains_data(),bin.old_file().kind()); println!("{}",bin.old_file().data().len()); true }),None,None).unwrap(); }
 "binary_patch" => { let d=Diff::from_buffer(b"diff --git a/file b/file\nindex 1234567..7654321 100644\nBinary files a/file and b/file differ\n").unwrap(); d.foreach(&mut |_,_| true,Some(&mut |_,b| {println!("contains_data={}",b.contains_data()); println!("{}",b.new_file().data().len());true}),None,None).unwrap(); }
 "blame" | "blame_orig" => { let td=tempfile::tempdir().unwrap(); let r=Repository::init(td.path()).unwrap(); fs::write(td.path().join("file"),b"old\n").unwrap(); let mut i=r.index().unwrap(); i.add_path(Path::new("file")).unwrap(); i.write().unwrap(); let t=r.find_tree(i.write_tree().unwrap()).unwrap(); let sig=Signature::new("A","a@b",&Time::new(123456,0)).unwrap(); r.commit(Some("HEAD"),&sig,&sig,"msg",&t,&[]).unwrap(); let blame=r.blame_file(Path::new("file"),None).unwrap(); let edited=blame.blame_buffer(b"new\n").unwrap(); let h=edited.get_line(1).unwrap(); println!("final oid={}",h.final_commit_id()); if std::env::args().nth(1).unwrap()=="blame" { println!("{:?}",h.final_signature().name()); } else {println!("{:?}",h.orig_signature().name());} }
 _ => panic!("bad mode")
 }
}
