#![allow(deprecated, unused)]
use git2::*;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
mod more;

pub fn dir() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let i = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = PathBuf::from("/tmp/pr").join(format!("{}-{}", std::process::id(), i));
    fs::create_dir_all(&d).unwrap();
    d
}
pub fn sig() -> Signature<'static> { Signature::new("a", "a@b", &Time::new(1000, 0)).unwrap() }
pub fn commit_file(r: &Repository, name: &str, data: &str, msg: &str) -> Oid {
    let wd = r.workdir().unwrap().to_path_buf();
    if let Some(p) = Path::new(name).parent() { fs::create_dir_all(wd.join(p)).unwrap(); }
    fs::write(wd.join(name), data).unwrap();
    let mut idx = r.index().unwrap();
    idx.add_path(Path::new(name)).unwrap();
    idx.write().unwrap();
    let t = r.find_tree(idx.write_tree().unwrap()).unwrap();
    let parent = r.head().ok().and_then(|h| h.peel_to_commit().ok());
    let ps: Vec<&Commit> = parent.iter().collect();
    r.commit(Some("HEAD"), &sig(), &sig(), msg, &t, &ps).unwrap()
}
pub fn repo() -> Repository {
    let d = dir();
    let r = Repository::init(&d).unwrap();
    { let mut c = r.config().unwrap(); c.set_str("user.name","a").unwrap(); c.set_str("user.email","a@b").unwrap(); }
    commit_file(&r, "seed", "seed\n", "init");
    r
}
pub fn sh(cwd: &Path, s: &str) -> String {
    let o = Command::new("sh").arg("-c").arg(s).current_dir(cwd).output().unwrap();
    String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr)
}
/// repo with a linear branch with 2 commits to rebase on top of main
pub fn rebase_repo() -> (Repository, Oid, Oid) {
    let r = repo();
    let base = r.head().unwrap().target().unwrap();
    let c1 = commit_file(&r, "x", "x\n", "x");
    let branch = r.head().unwrap().name().unwrap().to_string();
    { let bc = r.find_commit(base).unwrap(); r.branch("topic", &bc, true).unwrap(); }
    r.set_head("refs/heads/topic").unwrap();
    r.checkout_head(Some(build::CheckoutBuilder::new().force())).unwrap();
    let t1 = commit_file(&r, "y", "y\n", "y");
    let _ = commit_file(&r, "z", "z\n", "z");
    let t = r.head().unwrap().target().unwrap();
    (r, t, c1)
}

fn refspec_uaf() {
    let r = repo();
    r.remote("o", "/nonexistent").unwrap();
    let s;
    { let rm = r.find_remote("o").unwrap(); s = rm.get_refspec(0).unwrap(); }
    println!("str {:?}", s.str());
}
fn rebase_op_uaf() {
    let (r, t, c1) = rebase_repo();
    let tc = r.find_annotated_commit(t).unwrap();
    let up = r.find_annotated_commit(c1).unwrap();
    let mut rb = r.rebase(Some(&tc), Some(&up), None, None).unwrap();
    let op = rb.next().unwrap().unwrap();
    println!("before kind {:?} id {}", op.kind(), op.id());
    drop(rb);
    println!("after kind {:?} id {}", op.kind(), op.id());
}
fn rebase_opts_uaf() {
    let (r, t, c1) = rebase_repo();
    let tc = r.find_annotated_commit(t).unwrap();
    let up = r.find_annotated_commit(c1).unwrap();
    let mut rb;
    {
        let v = vec![1u8; 32];
        let mut co = build::CheckoutBuilder::new();
        co.force();
        co.progress(move |_, _, _| { println!("cb sees {:?}", &v[..2]); });
        let mut o = RebaseOptions::new();
        o.checkout_options(co);
        rb = r.rebase(Some(&tc), Some(&up), None, Some(&mut o)).unwrap();
    }
    let g = vec![0xAAu8; 32]; 
    let op = rb.next();
    println!("next {:?}", op.map(|x| x.map(|o| o.kind())));
    drop(g);
}
fn revwalk_uaf() {
    let r = repo();
    commit_file(&r, "b", "b", "b"); commit_file(&r, "c", "c", "c");
    let h = r.head().unwrap().target().unwrap();
    let w = r.revwalk().unwrap();
    let w2;
    {
        let v = vec![1u64; 4];
        let mut cb = |_o: Oid| { println!("hide cb sees {:?}", v); false };
        let wh = w.with_hide_callback(&mut cb).unwrap();
        w2 = wh.into_inner().unwrap();
    }
    let mut w2 = w2;
    let g = vec![0xAAu64; 4];
    w2.push(h).unwrap();
    println!("n {}", w2.count());
    drop(g);
}
fn mk_stash_opts<'a>() -> Box<StashApplyOptions<'a>> {
    let mut o = StashApplyOptions::new();
    o.progress_cb(|_p| { println!("progress"); true });
    Box::new(o)
}
fn stash_self() {
    let mut r = repo();
    fs::write(r.workdir().unwrap().join("seed"), "dirty\n").unwrap();
    r.stash_save(&sig(), "m", None).unwrap();
    let mut o = mk_stash_opts();
    println!("{:?}", r.stash_apply(0, Some(&mut o)));
}
fn mk_apply_opts<'a>() -> ApplyOptions<'a> {
    let mut o = ApplyOptions::new();
    o.delta_callback(|_p| { println!("delta"); true });
    o
}
fn apply_self() {
    let r = repo();
    let d = Diff::from_buffer(b"diff --git a/seed b/seed\n--- a/seed\n+++ b/seed\n@@ -1 +1 @@\n-seed\n+new\n").unwrap();
    let mut o = mk_apply_opts();
    let _pad = [0u8; 4096];
    let t = r.head().unwrap().peel_to_tree().unwrap();
    println!("{:?}", r.apply_to_tree(&t, &d, Some(&mut o)).map(|_| ()));
}
fn indexer_uaf() {
    let d = dir();
    let mut ix;
    { let odb = Odb::new().unwrap(); ix = Indexer::new(Some(&odb), &d, 0, true).unwrap(); }
    // pack with a delta-less blob
    let r = repo();
    let mut pb = r.packbuilder().unwrap();
    pb.insert_commit(r.head().unwrap().target().unwrap()).unwrap();
    let mut b = Buf::new();
    pb.write_buf(&mut b).unwrap();
    println!("write {:?}", ix.write_all(&b));
    println!("commit {:?}", ix.commit());
}
fn pathspec_uaf() {
    let r = repo();
    let t = r.head().unwrap().peel_to_tree().unwrap();
    let ps = Pathspec::new(["*"].iter()).unwrap();
    let l;
    { let d = r.diff_tree_to_tree(None, Some(&t), None).unwrap(); l = ps.match_diff(&d, PathspecFlags::DEFAULT).unwrap(); }
    println!("{:?}", l.diff_entry(0).map(|e| e.new_file().path().map(|p| p.to_path_buf())));
}
fn reflog_uaf() {
    let mut log;
    { let r = repo(); log = r.reflog("HEAD").unwrap(); }
    println!("append {:?}", log.append(Oid::from_bytes(&[1;20]).unwrap(), &sig(), Some("x")));
    println!("write {:?}", log.write());
}
fn mempack_race() {
    let r = repo();
    let odb = r.odb().unwrap();
    let mp = odb.add_new_mempack_backend(1000).unwrap();
    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|s| {
        let st = &stop; let od = &odb;
        s.spawn(move || { for i in 0..300000u32 { od.write(ObjectType::Blob, &i.to_le_bytes()).ok(); } st.store(true, std::sync::atomic::Ordering::SeqCst); });
        while !st.load(std::sync::atomic::Ordering::SeqCst) { mp.reset().ok(); }
    });
    println!("survived");
}
fn finalize_twice() {
    let r = repo();
    let odb = r.odb().unwrap();
    let mut w = odb.writer(3, ObjectType::Blob).unwrap();
    w.write_all(b"abc").unwrap();
    println!("1 {:?}", w.finalize());
    println!("2 {:?}", w.finalize());
}
fn finalize_write() {
    let r = repo();
    let odb = r.odb().unwrap();
    let mut w = odb.writer(3, ObjectType::Blob).unwrap();
    w.write_all(b"abc").unwrap();
    println!("1 {:?}", w.finalize());
    println!("write after {:?}", w.write(b"z"));
}
fn buf_null() { let b = Buf::new(); println!("len {}", b.len()); println!("{:?}", b.as_str()); }
fn binary_null() {
    let r = repo();
    let d = r.workdir().unwrap();
    let a = r.blob(b"a\0b").unwrap(); let b = r.blob(b"c\0d").unwrap();
    let (a, b) = (r.find_blob(a).unwrap(), r.find_blob(b).unwrap());
    r.diff_blobs(Some(&a), None, Some(&b), None, None, None, Some(&mut |_d, bin| {
        println!("contains {}", bin.contains_data());
        println!("len {}", bin.old_file().data().len());
        true
    }), None, None).unwrap();
}
fn list_empty() {
    let d = dir(); Repository::init_bare(&d).unwrap();
    let r = repo();
    let mut rm = r.remote_anonymous(d.to_str().unwrap()).unwrap();
    rm.connect(Direction::Fetch).unwrap();
    println!("n {}", rm.list().unwrap().len());
}
fn blame_null() {
    let r = repo();
    let b = r.blame_file(Path::new("seed"), None).unwrap();
    let b2 = b.blame_buffer(b"seed\nnew line\n").unwrap();
    for h in b2.iter() { println!("id {} lines {}", h.final_commit_id(), h.lines_in_hunk()); println!("name {:?}", h.final_signature().name()); }
}
fn blame_null_orig() {
    let r = repo();
    let b = r.blame_file(Path::new("seed"), None).unwrap();
    let b2 = b.blame_buffer(b"seed\nnew line\n").unwrap();
    for h in b2.iter() { println!("id {}", h.orig_commit_id()); println!("name {:?}", h.orig_signature().name()); }
}
fn index_cb(which: &str) {
    let r = repo();
    fs::write(r.workdir().unwrap().join("seed"), "mod\n").unwrap();
    fs::write(r.workdir().unwrap().join("newf"), "n\n").unwrap();
    let mut i = r.index().unwrap();
    let mut cb = |p: &Path, s: &[u8]| { println!("cb {:?} {:?}", p, s); 0 };
    let e: Vec<&str> = vec![];
    let res = match which {
        "add" => i.add_all(e.iter(), IndexAddOption::DEFAULT, Some(&mut cb)),
        "update" => i.update_all(e.iter(), Some(&mut cb)),
        _ => i.remove_all(e.iter(), Some(&mut cb)),
    };
    println!("{:?}", res);
}
fn workdir_uaf() {
    let r = repo();
    let w = r.workdir().unwrap();
    let before = w.to_path_buf();
    let d = dir();
    r.set_workdir(&d, false).unwrap();
    let g = vec![0xAAu8; before.as_os_str().len()+1];
    println!("{:?} vs {:?}", w, before);
    drop(g);
}
fn namespace_uaf() {
    let r = repo();
    r.set_namespace("first_namespace_abc").unwrap();
    let n = r.namespace().unwrap();
    r.set_namespace("second_namespace_x").unwrap();
    println!("{:?}", n);
}
fn remove_ns_uaf() {
    let r = repo();
    r.set_namespace("first_namespace_abc").unwrap();
    let n = r.namespace_bytes().unwrap();
    r.remove_namespace().unwrap();
    println!("{:?}", n.to_vec());
}
fn attr_uaf() {
    let r = repo();
    let ga = r.workdir().unwrap().join(".gitattributes");
    fs::write(&ga, "f foo=this_is_the_first_value_abcdefghijklmnopqrstuvwxyz\n").unwrap();
    let v = r.get_attr(Path::new("f"), "foo", AttrCheckFlags::FILE_THEN_INDEX).unwrap().unwrap();
    println!("v1 {}", v);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::write(&ga, "f foo=other_value_entirely_0123456789 bar=1\nzz baz=2\n").unwrap();
    let v2 = r.get_attr(Path::new("f"), "foo", AttrCheckFlags::FILE_THEN_INDEX).unwrap().unwrap();
    println!("v2 {}", v2);
    println!("v1 again {}", v);
}
fn stash_null_msg() {
    let mut r = repo();
    fs::write(r.workdir().unwrap().join("seed"), "dirty\n").unwrap();
    r.stash_save(&sig(), "m", None).unwrap();
    let p = r.path().join("logs/refs/stash");
    let s = fs::read_to_string(&p).unwrap();
    println!("log {:?}", s);
    let i = s.find('\t').unwrap();
    fs::write(&p, format!("{}\n", &s[..i])).unwrap();
    println!("log {:?}", fs::read_to_string(&p));
    let r2 = Repository::open(r.path()).unwrap();
    let mut r2 = r2;
    println!("{:?}", r2.stash_foreach(|i, m, id| { println!("{} {:?}", i, m); true }));
}
fn clone_null() {
    let src = repo();
    let d = dir().join("c");
    let mut co = build::CheckoutBuilder::new();
    co.notify_on(CheckoutNotificationType::all());
    co.notify(|_, _, _, _, _| false);
    let res = build::RepoBuilder::new().with_checkout(co).clone(src.path().to_str().unwrap(), &d);
    println!("is_ok {}", res.is_ok());
    if let Ok(r) = res { println!("path {:?}", r.path()); } else { println!("{:?}", res.err()); }
}
pub fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n = a.get(1).map(|s| s.as_str()).unwrap_or("");
    match n {
        "refspec_uaf" => refspec_uaf(), "rebase_op_uaf" => rebase_op_uaf(), "rebase_opts_uaf" => rebase_opts_uaf(),
        "revwalk_uaf" => revwalk_uaf(), "stash_self" => stash_self(), "apply_self" => apply_self(),
        "indexer_uaf" => indexer_uaf(), "pathspec_uaf" => pathspec_uaf(), "reflog_uaf" => reflog_uaf(),
        "mempack_race" => mempack_race(), "finalize_twice" => finalize_twice(), "finalize_write" => finalize_write(), "buf_null" => buf_null(),
        "binary_null" => binary_null(), "list_empty" => list_empty(), "blame_null" => blame_null(), "blame_null_orig" => blame_null_orig(),
        "index_add" => index_cb("add"), "index_update" => index_cb("update"), "index_remove" => index_cb("remove"),
        "workdir_uaf" => workdir_uaf(), "namespace_uaf" => namespace_uaf(), "remove_ns_uaf" => remove_ns_uaf(), "attr_uaf" => attr_uaf(),
        "stash_null_msg" => stash_null_msg(), "clone_null" => clone_null(),
        _ => more::dispatch(n),
    }
}
