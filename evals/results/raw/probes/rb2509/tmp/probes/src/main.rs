#![allow(unused, deprecated)]
use git2::*;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

fn sig() -> Signature<'static> { Signature::new("A U Thor", "a@example.com", &Time::new(1700000000, 0)).unwrap() }

fn tmp(name: &str) -> PathBuf {
    let base = std::env::temp_dir();
    let p = base.join(format!("p-{}-{}", name, std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn commit_file(repo: &Repository, name: &str, content: &[u8], msg: &str) -> Oid {
    let wd = repo.workdir().unwrap().to_path_buf();
    fs::write(wd.join(name), content).unwrap();
    let mut ix = repo.index().unwrap();
    ix.add_path(Path::new(name)).unwrap();
    ix.write().unwrap();
    let tid = ix.write_tree().unwrap();
    let tree = repo.find_tree(tid).unwrap();
    let s = sig();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &s, &s, msg, &tree, &parents).unwrap()
}

fn repo(name: &str) -> (PathBuf, Repository) {
    let p = tmp(name);
    let r = Repository::init(&p).unwrap();
    { let mut c = r.config().unwrap(); c.set_str("user.name","A").unwrap(); c.set_str("user.email","a@e").unwrap(); }
    commit_file(&r, "file.txt", b"line1\nline2\nline3\n", "initial");
    (p, r)
}

fn add_bad_ref(p: &Path, r: &Repository, dir: &str) -> Oid {
    // The audit filesystem refuses non-UTF-8 file names, so use packed-refs,
    // which is how a fetched or cloned ref would normally be stored anyway.
    let id = r.head().unwrap().target().unwrap();
    let mut data = b"# pack-refs with: peeled fully-peeled sorted \n".to_vec();
    data.extend_from_slice(format!("{} {}/bad", id, dir).as_bytes());
    data.extend_from_slice(b"\xff\n");
    fs::write(p.join(".git/packed-refs"), data).unwrap();
    id
}
trait V { fn into_vec_(self) -> Vec<u8>; }
impl V for std::ffi::OsString { fn into_vec_(self) -> Vec<u8> { use std::os::unix::ffi::OsStringExt; self.into_vec() } }

// Two-commit branches for rebase: main advanced, topic diverged.
fn rebase_repo(name: &str) -> (PathBuf, Repository, Oid, Oid) {
    let (p, r) = repo(name);
    let base = r.head().unwrap().target().unwrap();
    let main = commit_file(&r, "m.txt", b"m\n", "main work");
    // topic from base
    let bc = r.find_commit(base).unwrap();
    r.branch("topic", &bc, false).unwrap();
    r.set_head("refs/heads/topic").unwrap();
    let mut co = build::CheckoutBuilder::new(); co.force();
    r.checkout_head(Some(&mut co)).unwrap();
    let t = commit_file(&r, "t.txt", b"t\n", "topic work");
    drop(bc);
    (p, r, main, t)
}

fn p(s: &str) { println!("{}", s); }

fn run(name: &str) {
    match name {
    // ---------- soundness ----------
    "refspec_uaf" => {
        let (_p, r) = repo("rs");
        r.remote("origin", "/nowhere").unwrap();
        let spec = { let rem = r.find_remote("origin").unwrap(); rem.get_refspec(0).unwrap() };
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![b'X'; 64]).collect();
        p(&format!("after remote dropped, refspec.str() = {:?}", spec.str()));
        drop(junk);
    }
    "rebase_op_uaf" => {
        let (_p, r, main, t) = rebase_repo("ro");
        let up = r.find_annotated_commit(main).unwrap();
        let br = r.find_annotated_commit(t).unwrap();
        let mut o = RebaseOptions::new(); o.inmemory(true);
        let op = {
            let mut rb = r.rebase(Some(&br), Some(&up), None, Some(&mut o)).unwrap();
            rb.next().unwrap().unwrap()
        };
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0x41u8; 64]).collect();
        p(&format!("op after rebase dropped: kind={:?} id={}", op.kind(), op.id()));
        drop(junk);
    }
    "rebase_opts_uaf" => {
        let (_p, r, main, t) = rebase_repo("rou");
        let up = r.find_annotated_commit(main).unwrap();
        let br = r.find_annotated_commit(t).unwrap();
        let mut rb = {
            let mut co = build::CheckoutBuilder::new();
            let v = vec![1u8; 32];
            co.progress(move |_, _, _| { println!("progress cb sees {:?}", &v[..2]); });
            let mut o = RebaseOptions::new();
            o.checkout_options(co);
            r.rebase(Some(&br), Some(&up), None, Some(&mut o)).unwrap()
        };
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0x41u8; 128]).collect();
        p("options dropped, calling next()");
        let op = rb.next();
        p(&format!("next returned {:?}", op.map(|o| o.is_ok())));
        drop(junk);
    }
    "revwalk_hide_uaf" => {
        let (_p, r) = repo("rh");
        commit_file(&r, "a", b"a", "2"); commit_file(&r, "b", b"b", "3");
        let mut walk = {
            let heap = vec![7u64; 4];
            let mut cb = move |_o: Oid| { println!("hide cb runs, heap[0]={}", heap[0]); false };
            let w = r.revwalk().unwrap().with_hide_callback(&mut cb).unwrap();
            w.into_inner().unwrap()
        };
        fn clobber() { let mut a = [0xAAu8; 8192]; std::hint::black_box(&mut a); }
        clobber();
        walk.push_head().unwrap();
        let n: Vec<_> = walk.by_ref().collect();
        p(&format!("walk yielded {}", n.len()));
    }
    "stash_opts_moved" => {
        let (p0, mut r) = repo("so");
        fs::write(p0.join("file.txt"), b"changed\n").unwrap();
        let s = sig();
        r.stash_save(&s, "x", None).unwrap();
        fn mk<'a>() -> StashApplyOptions<'a> { let mut b = Box::new(StashApplyOptions::new()); b.progress_cb(|_| { println!("stash progress"); true }); *b }
        let mut moved = mk();
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0x41u8; 128]).collect();
        let res = r.stash_apply(0, Some(&mut moved));
        p(&format!("stash_apply = {:?}", res.is_ok()));
        drop(junk);
    }
    "apply_opts_moved" => {
        let (p0, r) = repo("ao");
        fs::write(p0.join("file.txt"), b"line1\nchanged\nline3\n").unwrap();
        let diff = r.diff_index_to_workdir(None, None).unwrap();
        fn mk<'a>() -> ApplyOptions<'a> { let mut b = Box::new(ApplyOptions::new()); b.delta_callback(|_| { println!("delta cb"); true }); *b }
        let mut moved = mk();
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0x41u8; 512]).collect();
        let res = r.apply(&diff, ApplyLocation::Index, Some(&mut moved));
        p(&format!("apply = {:?}", res.is_ok()));
        drop(junk);
    }
    "indexer_odb_uaf" => {
        let (p0, r) = repo("io");
        let head = r.head().unwrap().target().unwrap();
        let mut pb = r.packbuilder().unwrap();
        pb.insert_commit(head).unwrap();
        let mut buf = Buf::new();
        pb.write_buf(&mut buf).unwrap();
        let dir = tmp("io2");
        let odb = Odb::new().unwrap();
        let mut ix = Indexer::new(Some(&odb), &dir, 0, true).unwrap();
        drop(odb);
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0x41u8; 128]).collect();
        let w = ix.write_all(&buf);
        p(&format!("indexer write = {:?}", w.is_ok()));
        p(&format!("commit = {:?}", ix.commit().is_ok()));
        drop(junk);
    }
    "pathspec_diff_uaf" => {
        let (p0, r) = repo("pd");
        fs::write(p0.join("file.txt"), b"line1\nchanged\nline3\n").unwrap();
        let ps = Pathspec::new(["*"]).unwrap();
        let list = {
            let diff = r.diff_index_to_workdir(None, None).unwrap();
            ps.match_diff(&diff, PathspecFlags::DEFAULT).unwrap()
        };
        let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0x41u8; 128]).collect();
        for d in list.diff_entries() { p(&format!("delta path after diff dropped: {:?}", d.new_file().path())); }
        drop(junk);
    }
    "buf_null" => {
        let b = Buf::new();
        p(&format!("len={}", b.len()));
        p(&format!("as_str={:?}", b.as_str()));
    }
    "binary_null" => {
        let (p0, r) = repo("bn");
        commit_file(&r, "bin", b"\0\x01\x02", "bin");
        fs::write(p0.join("bin"), b"\0\x01\x03").unwrap();
        let diff = r.diff_index_to_workdir(None, None).unwrap();
        let mut f = |_: DiffDelta<'_>, _: f32| true;
        let mut bcb = |_: DiffDelta<'_>, b: DiffBinary<'_>| {
            println!("contains_data={}", b.contains_data());
            println!("old data len={}", b.old_file().data().len());
            true
        };
        let res = diff.foreach(&mut f, Some(&mut bcb), None, None);
        p(&format!("foreach={:?}", res.is_ok()));
    }
    "blame_buffer_sig" => {
        let (_p, r) = repo("bb");
        let blame = r.blame_file(Path::new("file.txt"), None).unwrap();
        let b2 = blame.blame_buffer(b"line1\nNEW\nline2\nline3\n").unwrap();
        p(&format!("hunks={}", b2.len()));
        for h in b2.iter() {
            p(&format!("hunk start={} commit={}", h.final_start_line(), h.final_commit_id()));
            let s = h.final_signature();
            p(&format!("  name={:?}", s.name()));
        }
    }
    "odb_double_finalize" => {
        let (_p, r) = repo("df");
        let odb = r.odb().unwrap();
        let mut w = odb.writer(3, ObjectType::Blob).unwrap();
        w.write_all(b"abc").unwrap();
        p(&format!("1st finalize {:?}", w.finalize()));
        p(&format!("2nd finalize {:?}", w.finalize()));
    }
    "odb_double_commit" => {
        let (_p, r) = repo("dc");
        let head = r.head().unwrap().target().unwrap();
        let mut pb = r.packbuilder().unwrap();
        pb.insert_commit(head).unwrap();
        let mut buf = Buf::new();
        pb.write_buf(&mut buf).unwrap();
        let (_p2, r2) = repo("dc2");
        let odb = r2.odb().unwrap();
        let mut pw = odb.packwriter().unwrap();
        pw.write_all(&buf).unwrap();
        p(&format!("1st commit {:?}", pw.commit()));
        p(&format!("2nd commit {:?}", pw.commit()));
        p(&format!("write after commit {:?}", pw.write_all(&buf).is_ok()));
    }
    "submodules_abort" => {
        let (p0, r) = repo("sa");
        let mut f = fs::OpenOptions::new().append(true).open(p0.join(".git/config")).unwrap();
        f.write_all(b"[submodule \"orphan\"]\n\turl = https://example.com/x.git\n\tactive = true\n").unwrap();
        drop(f);
        let r = Repository::open(&p0).unwrap();
        p(&format!("find_submodule(orphan) = {:?}", r.find_submodule("orphan").map(|_| ()).map_err(|e| e.to_string())));
        let s = r.submodules();
        p(&format!("submodules = {:?}", s.map(|v| v.len()).map_err(|e| e.to_string())));
    }
    "submodules_abort2" => {
        // gitlink in index, plus .gitmodules naming a different path layout
        let (p0, r) = repo("sb");
        fs::write(p0.join(".gitmodules"), b"[submodule \"a\"]\n\tpath = sub/a\n\turl = x\n[submodule \"b\"]\n\tpath = sub/a\n\turl = y\n[submodule \"c/../d\"]\n\tpath = c\n\turl = z\n").unwrap();
        let r = Repository::open(&p0).unwrap();
        let s = r.submodules();
        p(&format!("submodules = {:?}", s.map(|v| v.len()).map_err(|e| e.to_string())));
    }
    // ---------- panics ----------
    "ref_names_nonutf8" => {
        let (p0, r) = repo("rn");
        add_bad_ref(&p0, &r, "refs/heads");
        let mut refs = r.references().unwrap();
        let v: Vec<_> = refs.names().map(|x| x.map(|s| s.to_string())).collect();
        p(&format!("{:?}", v));
    }
    "remote_head_nonutf8" => {
        let (p0, r) = repo("rh2");
        add_bad_ref(&p0, &r, "refs/heads");
        let (_p1, c) = repo("rh3");
        let mut rem = c.remote_anonymous(p0.to_str().unwrap()).unwrap();
        rem.connect(Direction::Fetch).unwrap();
        for h in rem.list().unwrap() { p(&format!("head: {}", h.name())); }
    }
    "update_tips_nonutf8" => {
        let (p0, r) = repo("ut");
        add_bad_ref(&p0, &r, "refs/heads");
        let (_p1, c) = repo("ut2");
        let mut rem = c.remote("origin", p0.to_str().unwrap()).unwrap();
        let mut cb = RemoteCallbacks::new();
        cb.update_tips(|n, _, _| { println!("tip {}", n); true });
        let mut fo = FetchOptions::new(); fo.remote_callbacks(cb);
        let res = rem.fetch(&[] as &[&str], Some(&mut fo), None);
        p(&format!("fetch={:?}", res.map_err(|e| e.to_string())));
    }
    "nul_valid_name_ref" => { p(&format!("{}", Reference::is_valid_name("a\0b"))); }
    "nul_valid_name_tag" => { p(&format!("{}", Tag::is_valid_name("a\0b"))); }
    "nul_valid_name_remote" => { p(&format!("{}", Remote::is_valid_name("a\0b"))); }
    "nul_tree_get_name" => { let (_p, r) = repo("nt"); let t = r.head().unwrap().peel_to_tree().unwrap(); p(&format!("{}", t.get_name("a\0b").is_some())); }
    "nul_objtype" => { p(&format!("{:?}", ObjectType::from_str("blob\0"))); }
    "nul_refspec" => { let (_p, r) = repo("nr"); r.remote("o","/x").unwrap(); let rem = r.find_remote("o").unwrap(); let s = rem.get_refspec(0).unwrap(); p(&format!("{:?}", s.transform("a\0b").is_err())); }
    "nul_txn" => { let (_p, r) = repo("ntx"); let mut t = r.transaction().unwrap(); p(&format!("{:?}", t.lock_ref("a\0b").is_err())); }
    "nul_builder" => { let mut o = StatusOptions::new(); o.pathspec("a\0b"); p("no panic"); }
    "prettify_nonutf8" => { p(&format!("{:?}", message_prettify(&b"caf\xe9\n"[..], None))); }
    "index_get_path_abs" => { let (_p, r) = repo("ig"); let ix = r.index().unwrap(); p(&format!("{}", ix.get_path(Path::new("/etc/passwd"), 0).is_some())); }
    "index_get_path_dot" => { let (_p, r) = repo("ig2"); let ix = r.index().unwrap(); p(&format!("{}", ix.get_path(Path::new("./file.txt"), 0).is_some())); }
    "pathspec_matches_abs" => { let ps = Pathspec::new(["*"]).unwrap(); p(&format!("{}", ps.matches_path(Path::new("/abs"), PathspecFlags::DEFAULT))); }
    "config_valueless" => {
        let d = tmp("cv"); let f = d.join("cfg");
        fs::write(&f, "[core]\n\tbare\n").unwrap();
        let c = Config::open(&f).unwrap();
        let mut it = c.entries(None).unwrap();
        while let Some(e) = it.next() { let e = e.unwrap(); p(&format!("{:?} = {:?}", e.name(), e.value().unwrap_or(""))); }
    }
    "annotated_refname" => { let (_p, r) = repo("an"); let id = r.head().unwrap().target().unwrap(); let a = r.find_annotated_commit(id).unwrap(); p(&format!("{:?}", a.refname())); }
    "rebase_exec" => {
        let (_p, r, main, t) = rebase_repo("re");
        let up = r.find_annotated_commit(main).unwrap();
        let br = r.find_annotated_commit(t).unwrap();
        let mut o = RebaseOptions::new(); o.inmemory(true);
        let mut rb = r.rebase(Some(&br), Some(&up), None, Some(&mut o)).unwrap();
        let op = rb.next().unwrap().unwrap();
        p(&format!("kind={:?}", op.kind()));
        p(&format!("exec={:?}", op.exec()));
    }
    "worktree_lock_nonutf8" => {
        let (p0, r) = repo("wl");
        let wtp = tmp("wl2").join("wt");
        let wt = r.worktree("wt", &wtp, None).unwrap();
        fs::write(p0.join(".git/worktrees/wt/locked"), b"caf\xe9").unwrap();
        p(&format!("{:?}", wt.is_locked()));
    }
    "describe_nonutf8" => {
        let (p0, r) = repo("de");
        add_bad_ref(&p0, &r, "refs/tags");
        let mut o = DescribeOptions::new(); o.describe_tags();
        let d = r.describe(&o).unwrap();
        p(&format!("{:?}", d.format(None)));
    }
    "merge_msg_nonutf8" => { let (p0, r) = repo("mm"); fs::write(p0.join(".git/MERGE_MSG"), b"caf\xe9\n").unwrap(); p(&format!("{:?}", r.message())); }
    "notes_ref_nonutf8" => {
        let (p0, r) = repo("nn");
        let mut f = fs::OpenOptions::new().append(true).open(p0.join(".git/config")).unwrap();
        f.write_all(b"[core]\n\tnotesRef = refs/notes/caf\xe9\n").unwrap(); drop(f);
        let r = Repository::open(&p0).unwrap();
        p(&format!("{:?}", r.note_default_ref()));
    }
    "diff_mode_panic" => {
        let patch = b"diff --git a/x b/x\nnew file mode 100600\nindex 0000000..e69de29\n--- /dev/null\n+++ b/x\n@@ -0,0 +1 @@\n+a\n";
        let d = Diff::from_buffer(patch).unwrap();
        for dd in d.deltas() { p(&format!("status={:?}", dd.status())); p(&format!("mode={:?}", dd.new_file().mode())); }
    }
    "format_email_assert" => {
        let (_p, r) = repo("fe");
        let id = commit_file(&r, "x", b"x\n", "first line\nsecond line of subject\n\nbody\n");
        let c = r.find_commit(id).unwrap();
        let t = c.tree().unwrap(); let pt = c.parent(0).unwrap().tree().unwrap();
        let mut d = r.diff_tree_to_tree(Some(&pt), Some(&t), None).unwrap();
        p(&format!("{:?}", d.format_email(1, 1, &c, None).is_ok()));
    }
    "abort_set_err_io" => {
        struct T; struct S;
        impl std::io::Read for S { fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { Err(std::io::Error::new(std::io::ErrorKind::Other, "bad\0byte")) } }
        impl std::io::Write for S { fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { Ok(b.len()) } fn flush(&mut self) -> std::io::Result<()> { Ok(()) } }
        impl transport::SmartSubtransport for T {
            fn action(&self, _: &str, _: transport::Service) -> Result<Box<dyn transport::SmartSubtransportStream>, Error> { Ok(Box::new(S)) }
            fn close(&self) -> Result<(), Error> { Ok(()) }
        }
        unsafe { transport::register("dummy", |r| transport::Transport::smart(r, false, T)).unwrap(); }
        let (_p, r) = repo("ae");
        let mut rem = r.remote("o", "dummy://x").unwrap();
        p(&format!("{:?}", rem.connect(Direction::Fetch).map_err(|e| e.to_string())));
    }
    "factory_err_msg" => {
        unsafe { transport::register("dummy", |_r| Err(Error::from_str("my transport factory failed"))).unwrap(); }
        let (_p, r) = repo("fm");
        let mut rem = r.remote("o", "dummy://x").unwrap();
        p(&format!("{:?}", rem.connect(Direction::Fetch).map_err(|e| e.to_string())));
    }
    // ---------- swallowed panics ----------
    "swallow_odb_foreach" => {
        let (_p, r) = repo("s1");
        let odb = r.odb().unwrap();
        let res = odb.foreach(|_| panic!("boom in odb foreach"));
        p(&format!("returned normally: {:?}", res));
        poison_check(&r);
    }
    "swallow_tag_foreach" => {
        let (_p, r) = repo("s2");
        let o = r.head().unwrap().peel(ObjectType::Any).unwrap();
        r.tag_lightweight("t", &o, false).unwrap();
        let res = r.tag_foreach(|_, _| panic!("boom in tag foreach"));
        p(&format!("returned normally: {:?}", res));
        poison_check(&r);
    }
    "swallow_checkout_progress" => {
        let (p0, r) = repo("s3");
        fs::write(p0.join("file.txt"), b"x\n").unwrap();
        let mut co = build::CheckoutBuilder::new(); co.force();
        co.progress(|_, _, _| panic!("boom in progress"));
        let res = r.checkout_head(Some(&mut co));
        p(&format!("returned normally: {:?}", res));
        poison_check(&r);
    }
    "swallow_checkout_notify" => {
        let (p0, r) = repo("s4");
        fs::write(p0.join("file.txt"), b"x\n").unwrap();
        let mut co = build::CheckoutBuilder::new(); co.force(); co.notify_on(CheckoutNotificationType::all());
        co.notify(|_, _, _, _, _| panic!("boom in notify"));
        let res = r.checkout_head(Some(&mut co));
        p(&format!("returned normally: {:?}; file={:?}", res, fs::read(p0.join("file.txt")).unwrap()));
        poison_check(&r);
    }
    "checkout_notify_false" => {
        let (p0, r) = repo("s5");
        fs::write(p0.join("file.txt"), b"x\n").unwrap();
        let mut co = build::CheckoutBuilder::new(); co.force(); co.notify_on(CheckoutNotificationType::all());
        co.notify(|_, _, _, _, _| false);
        let res = r.checkout_head(Some(&mut co));
        p(&format!("cancelled checkout returned: {:?}; file={:?}", res.map_err(|e| e.to_string()), String::from_utf8_lossy(&fs::read(p0.join("file.txt")).unwrap())));
    }
    "swallow_stash_foreach" => {
        let (p0, mut r) = repo("s6");
        fs::write(p0.join("file.txt"), b"x\n").unwrap();
        let s = sig(); r.stash_save(&s, "m", None).unwrap();
        let res = r.stash_foreach(|_, _, _| panic!("boom in stash foreach"));
        p(&format!("returned normally: {:?}", res));
        poison_check(&r);
    }
    "stash_nonutf8" => {
        let (p0, mut r) = repo("s7");
        fs::write(p0.join("file.txt"), b"x\n").unwrap();
        let s = sig(); r.stash_save(&s, "first", None).unwrap();
        fs::write(p0.join("file.txt"), b"y\n").unwrap();
        r.stash_save(&s, "second", None).unwrap();
        // rewrite newest reflog message with latin-1
        let lf = p0.join(".git/logs/refs/stash");
        let mut c = fs::read(&lf).unwrap();
        let pos = c.windows(6).rposition(|w| w == b"second").unwrap();
        c[pos] = 0xe9; fs::write(&lf, c).unwrap();
        let mut r = Repository::open(&p0).unwrap();
        let mut n = 0;
        let res = r.stash_foreach(|_, m, _| { println!("msg {}", m); n += 1; true });
        p(&format!("returned normally: {:?}; entries seen {}", res, n));
        poison_check(&r);
    }
    "fetchhead_nonutf8" => {
        let (p0, r) = repo("s8");
        let id = r.head().unwrap().target().unwrap();
        fs::write(p0.join(".git/FETCH_HEAD"), [format!("{}\t\tbranch 'bad", id).into_bytes(), b"\xff' of /x\n".to_vec()].concat()).unwrap();
        let mut n = 0;
        let res = r.fetchhead_foreach(|_, _, _, _| { n += 1; true });
        p(&format!("returned normally: {:?}; entries seen {}", res.map_err(|e| e.to_string()), n));
        poison_check(&r);
    }
    "treewalk_nonutf8" => {
        let (_p, r) = repo("tw");
        let blob = r.blob(b"x").unwrap();
        let mut sub = r.treebuilder(None).unwrap();
        sub.insert("f", blob, 0o100644).unwrap();
        let subid = sub.write().unwrap();
        let mut top = r.treebuilder(None).unwrap();
        top.insert("a", blob, 0o100644).unwrap();
        top.insert(&b"caf\xe9"[..], subid, 0o040000).unwrap();
        top.insert("z", blob, 0o100644).unwrap();
        let t = r.find_tree(top.write().unwrap()).unwrap();
        let mut seen = vec![];
        let res = t.walk(TreeWalkMode::PreOrder, |root, e| { seen.push(format!("{}{:?}", root, e.name())); TreeWalkResult::Ok });
        p(&format!("walk={:?} seen={:?}", res.map_err(|e| e.to_string()), seen));
    }
    // ---------- behaviour ----------
    "skip_reuc" => {
        let (p0, r) = repo("sr");
        let base = r.head().unwrap().peel_to_commit().unwrap();
        let ours = commit_file(&r, "file.txt", b"ours\n", "ours");
        r.branch("o", &r.find_commit(ours).unwrap(), false).unwrap();
        r.set_head_detached(base.id()).unwrap();
        let mut co = build::CheckoutBuilder::new(); co.force(); r.checkout_head(Some(&mut co)).unwrap();
        let theirs = { let mut ix = r.index().unwrap(); fs::write(p0.join("file.txt"), b"theirs\n").unwrap(); ix.add_path(Path::new("file.txt")).unwrap(); let t = ix.write_tree().unwrap(); let tr = r.find_tree(t).unwrap(); let s = sig(); r.commit(None, &s, &s, "theirs", &tr, &[&base]).unwrap() };
        let oc = r.find_commit(ours).unwrap(); let tc = r.find_commit(theirs).unwrap();
        let plain = r.merge_commits(&oc, &tc, None);
        p(&format!("no options: ok={} conflicts={:?}", plain.is_ok(), plain.as_ref().map(|i| i.has_conflicts()).ok()));
        let mut mo = MergeOptions::new(); mo.skip_reuc(true);
        let res = r.merge_commits(&oc, &tc, Some(&mo));
        p(&format!("skip_reuc(true): {:?}", res.map(|i| i.has_conflicts()).map_err(|e| e.to_string())));
    }
    "is_none_flags" => {
        p(&format!("Sort::NONE.is_none()={}", Sort::NONE.is_none()));
        p(&format!("MergeAnalysis::ANALYSIS_NONE.is_none()={}", MergeAnalysis::ANALYSIS_NONE.is_none()));
        p(&format!("MergePreference::NONE.is_none()={}", MergePreference::NONE.is_none()));
        p(&format!("IndexAddOption::DEFAULT.is_default()={}", IndexAddOption::DEFAULT.is_default()));
        p(&format!("PathspecFlags::DEFAULT.is_default()={}", PathspecFlags::DEFAULT.is_default()));
        p(&format!("StashFlags::DEFAULT.is_default()={}", StashFlags::DEFAULT.is_default()));
        p(&format!("StashApplyFlags::DEFAULT.is_default()={}", StashApplyFlags::DEFAULT.is_default()));
        p(&format!("DiffStatsFormat::NONE.is_none()={}", DiffStatsFormat::NONE.is_none()));
        p(&format!("RepositoryInitMode::SHARED_UMASK.is_shared_umask()={}", RepositoryInitMode::SHARED_UMASK.is_shared_umask()));
        p(&format!("RevparseMode::SINGLE.is_no_single()={}", RevparseMode::SINGLE.is_no_single()));
    }
    "eq_impls" => {
        let (p0, r) = repo("eq");
        let t1 = r.head().unwrap().peel_to_tree().unwrap();
        let id2 = commit_file(&r, "file.txt", b"different\n", "2");
        let t2 = r.find_commit(id2).unwrap().tree().unwrap();
        let e1 = t1.get_name("file.txt").unwrap(); let e2 = t2.get_name("file.txt").unwrap();
        p(&format!("TreeEntry ids differ={} ; e1==e2 -> {}", e1.id()!=e2.id(), e1==e2));
        let c = r.find_commit(id2).unwrap();
        let a = r.branch("aaa", &c, false).unwrap().into_reference();
        let b = r.branch("bbb", &c, false).unwrap().into_reference();
        p(&format!("Reference names {:?} {:?} ; a==b -> {}", a.name(), b.name(), a==b));
        let t0 = Time::new(5, 0);
        let parsed = { let s = Signature::new("a","b",&Time::new(5,0)).unwrap(); s.when() };
        p(&format!("Time eq {}", t0==parsed));
    }
    "time_sign" => {
        let (_p, r) = repo("ts");
        let tree = r.head().unwrap().peel_to_tree().unwrap();
        let parent = r.head().unwrap().peel_to_commit().unwrap();
        let raw = format!("tree {}\nparent {}\nauthor A <a@e> 1700000000 -0000\ncommitter A <a@e> 1700000000 -0000\n\nmsg\n", tree.id(), parent.id());
        let id = r.odb().unwrap().write(ObjectType::Commit, raw.as_bytes()).unwrap();
        let c = r.find_commit(id).unwrap();
        let (a, b) = (c.time(), c.committer().when());
        p(&format!("commit.time().sign()={:?} committer().when().sign()={:?}", a.sign(), b.sign()));
        p(&format!("a==b {} a.cmp(b)={:?}", a==b, a.cmp(&b)));
    }
    "checkout_flags" => {
        let (p0, r) = repo("cf");
        // dry_run then force
        fs::write(p0.join("file.txt"), b"dirty\n").unwrap();
        let mut co = build::CheckoutBuilder::new(); co.dry_run(); co.force();
        r.checkout_head(Some(&mut co)).unwrap();
        p(&format!("dry_run().force(): file={:?}", String::from_utf8_lossy(&fs::read(p0.join("file.txt")).unwrap())));
        let mut co = build::CheckoutBuilder::new(); co.force();
        r.checkout_head(Some(&mut co)).unwrap();
        p(&format!("force(): file={:?}", String::from_utf8_lossy(&fs::read(p0.join("file.txt")).unwrap())));
        // recreate_missing then safe
        fs::remove_file(p0.join("file.txt")).unwrap();
        let mut co = build::CheckoutBuilder::new(); co.recreate_missing(true); co.safe();
        r.checkout_head(Some(&mut co)).unwrap();
        p(&format!("recreate_missing(true).safe(): exists={}", p0.join("file.txt").exists()));
        let mut co = build::CheckoutBuilder::new(); co.safe(); co.recreate_missing(true);
        r.checkout_head(Some(&mut co)).unwrap();
        p(&format!("safe().recreate_missing(true): exists={}", p0.join("file.txt").exists()));
    }
    "cred_inject" => {
        let d = tmp("ci");
        let script = d.join("helper.sh");
        let log = d.join("stdin.log");
        fs::write(&script, format!("#!/bin/sh\ncat > {}\necho username=u\necho password=p\n", log.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let f = d.join("cfg"); fs::write(&f, "").unwrap();
        let mut c = Config::open(&f).unwrap();
        c.set_str("credential.helper", &format!("!{}", script.display())).unwrap();
        let mut h = CredentialHelper::new("https://example.com/repo.git");
        h.config(&c);
        h.username(Some("x\nhost=evil.example\nprotocol=http"));
        p(&format!("{:?}", h.execute()));
        p(&format!("helper stdin:\n{}", fs::read_to_string(&log).unwrap()));
    }
    "cred_url_e2e" => {
        use std::io::{Read as _};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || { for c in l.incoming() { let mut c = c.unwrap(); let mut b=[0u8;4096]; let _ = c.read(&mut b); let _ = c.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); } });
        let d = tmp("ce");
        let script = d.join("helper.sh"); let log = d.join("stdin.log");
        fs::write(&script, format!("#!/bin/sh\ncat > {}\necho username=u\necho password=p\n", log.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let f = d.join("cfg"); fs::write(&f, "").unwrap();
        let mut cfg = Config::open(&f).unwrap();
        cfg.set_str("credential.helper", &format!("!{}", script.display())).unwrap();
        let (_p, r) = repo("ce2");
        let url = format!("http://x%0Ahost=evil.example@localhost:{}/repo.git", port);
        let mut rem = r.remote_anonymous(&url).unwrap();
        let mut cb = RemoteCallbacks::new();
        let mut tries = 0;
        cb.credentials(|u, n, _| { tries += 1; println!("cred cb url={:?} username_from_url={:?}", u, n); if tries > 1 { return Err(Error::from_str("stop")); } Cred::credential_helper(&cfg, u, n) });
        let res = rem.connect_auth(Direction::Fetch, Some(cb), None).map(|_| ()).map_err(|e| e.to_string());
        p(&format!("{:?}", res));
        p(&format!("helper stdin:\n{}", fs::read_to_string(&log).unwrap_or_default()));
    }
    "cred_url_decode" => {
        // shows that libgit2 hands the callback a percent-decoded username.
        let (_p, r) = repo("cu");
        let mut rem = r.remote_anonymous("http://x%0Ahost=evil.example@127.0.0.1:9/repo.git").unwrap();
        let mut cb = RemoteCallbacks::new();
        cb.credentials(|u, n, _| { println!("cred cb url={:?} username_from_url={:?}", u, n); Err(Error::from_str("stop")) });
        p(&format!("{:?}", rem.connect_auth(Direction::Fetch, Some(cb), None).map(|_| ()).map_err(|e| e.to_string())));
    }
    "cred_ip_host" => {
        let d = tmp("cih");
        let script = d.join("helper.sh");
        let log = d.join("stdin.log");
        fs::write(&script, format!("#!/bin/sh\ncat > {}\n", log.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let f = d.join("cfg"); fs::write(&f, "").unwrap();
        let mut c = Config::open(&f).unwrap();
        c.set_str("credential.helper", &format!("!{}", script.display())).unwrap();
        for u in ["https://192.168.1.10:8443/r.git", "https://[::1]/r.git", "https://example.com:8443/r.git"] {
            let mut h = CredentialHelper::new(u); h.config(&c); let _ = h.execute();
            p(&format!("{} ->\n{}", u, fs::read_to_string(&log).unwrap()));
        }
    }
    "cred_multi_helper" => {
        let d = tmp("cm");
        use std::os::unix::fs::PermissionsExt;
        let good = d.join("good.sh"); fs::write(&good, "#!/bin/sh\ncat >/dev/null\necho username=u\necho password=p\n").unwrap();
        let bad = d.join("bad.sh"); fs::write(&bad, "#!/bin/sh\ncat >/dev/null\nexit 0\n").unwrap();
        for s in [&good, &bad] { fs::set_permissions(s, fs::Permissions::from_mode(0o755)).unwrap(); }
        let f = d.join("cfg");
        fs::write(&f, format!("[credential]\n\thelper = !{}\n\thelper = !{}\n", good.display(), bad.display())).unwrap();
        let c = Config::open(&f).unwrap();
        let mut h = CredentialHelper::new("https://example.com/r.git"); h.config(&c);
        p(&format!("execute with [good, bad] helpers = {:?}", h.execute()));
    }
    "opts_timeout_neg" => {
        unsafe { let r = opts::set_server_timeout_in_milliseconds(-1); p(&format!("{:?}", r)); }
    }
    "narrow_parent" => {
        let (_p, r) = repo("np");
        let id = commit_file(&r, "x", b"x\n", "two");
        let c = r.find_commit(id).unwrap();
        p(&format!("parent_count={} parent_id(1)={:?} parent_id(1<<32)={:?}", c.parent_count(), c.parent_id(1).is_ok(), c.parent_id(1usize<<32).map(|o| o.to_string())));
        let b = r.blame_file(Path::new("file.txt"), None).unwrap();
        p(&format!("blame len={} get_index(1<<32).is_some()={}", b.len(), b.get_index(1usize<<32).is_some()));
    }
    "parents_fused" => {
        let (p0, r) = repo("pf");
        let c0 = r.head().unwrap().peel_to_commit().unwrap();
        let tree = c0.tree().unwrap();
        let missing = "1111111111111111111111111111111111111111";
        let raw = format!("tree {}\nparent {}\nparent {}\nauthor A <a@e> 1 +0000\ncommitter A <a@e> 1 +0000\n\nm\n", tree.id(), missing, c0.id());
        let id = r.odb().unwrap().write(ObjectType::Commit, raw.as_bytes()).unwrap();
        let c = r.find_commit(id).unwrap();
        let mut it = c.parents();
        p(&format!("len={} first={:?} second={:?} third={:?}", it.len(), it.next().map(|c| c.id()), it.next().map(|c| c.id()), it.next().map(|c| c.id())));
    }
    "packbuilder_hash" => {
        let (_p, r) = repo("ph");
        let head = r.head().unwrap().target().unwrap();
        let mut pb = r.packbuilder().unwrap();
        pb.insert_commit(head).unwrap();
        let d = tmp("ph2");
        pb.write(&d, 0).unwrap();
        p(&format!("objects={} written={} name={:?} hash={:?}", pb.object_count(), pb.written(), pb.name(), pb.hash()));
    }
    "error_msg" => {
        let (_p, r) = repo("em");
        let od = r.odb().unwrap(); let mut w = od.writer(1, ObjectType::Blob).unwrap();
        let e = w.finalize();
        p(&format!("{:?}", e));
        let mut tb = r.treebuilder(None).unwrap();
        p(&format!("{:?}", tb.insert("a/b", Oid::zero(), 0o100644).map(|_| ()).map_err(|e| e.to_string())));
    }
    "packwriter_panic" => {
        let (_p, r) = repo("pp");
        let head = r.head().unwrap().target().unwrap();
        let mut pb = r.packbuilder().unwrap(); pb.insert_commit(head).unwrap();
        let mut buf = Buf::new(); pb.write_buf(&mut buf).unwrap();
        let (_p2, r2) = repo("pp2");
        let odb = r2.odb().unwrap();
        let mut pw = odb.packwriter().unwrap();
        pw.progress(|_| panic!("boom in packwriter progress"));
        let w = pw.write_all(&buf);
        p(&format!("write returned: {:?}", w.map_err(|e| e.to_string())));
        poison_check(&r);
    }
    "indexer_panic" => {
        let (_p, r) = repo("ip");
        let head = r.head().unwrap().target().unwrap();
        let mut pb = r.packbuilder().unwrap(); pb.insert_commit(head).unwrap();
        let mut buf = Buf::new(); pb.write_buf(&mut buf).unwrap();
        let d = tmp("ip2");
        let mut ix = Indexer::new(None, &d, 0, false).unwrap();
        ix.progress(|_| panic!("boom in indexer progress"));
        let w = ix.write_all(&buf);
        p(&format!("write returned: {:?}", w.map_err(|e| e.to_string())));
        poison_check(&r);
    }
    "remote_create_err" => {
        let d = tmp("rc");
        let (p0, _r) = repo("rc0");
        let mut b = build::RepoBuilder::new();
        b.remote_create(|_, _, _| Err(Error::from_str("policy forbids this remote")));
        let res = b.clone(p0.to_str().unwrap(), &d.join("c"));
        p(&format!("{:?}", res.map(|_| ()).map_err(|e| e.to_string())));
    }
    "hide_cb_twice" => {
        let (_p, r) = repo("h2");
        commit_file(&r, "a", b"a", "2");
        let mut c1 = |_o: Oid| { println!("first cb"); false };
        let mut c2 = |_o: Oid| { println!("second cb"); false };
        let w = r.revwalk().unwrap().with_hide_callback(&mut c1).unwrap().into_inner().unwrap();
        let mut w = w.with_hide_callback(&mut c2);
        p(&format!("second with_hide_callback ok={}", w.is_ok()));
        let mut w = w.unwrap().into_inner().unwrap();
        w.push_head().unwrap(); let _ = w.by_ref().count();
    }
    "mempack_race" => {
        let (_p, r) = repo("mr");
        let odb = r.odb().unwrap();
        let _m = odb.add_new_mempack_backend(1000).unwrap();
        std::thread::scope(|s| {
            for t in 0..4u32 {
                let o = &odb;
                s.spawn(move || { for i in 0..200_000u32 { let d = format!("{} {}", t, i); o.write(ObjectType::Blob, d.as_bytes()).unwrap(); } });
            }
        });
        p("no crash");
    }
    "diff_send_scope" => {
        let (p0, r) = repo("ds");
        fs::write(p0.join("file.txt"), b"line1\nchanged\nline3\n").unwrap();
        let diff = r.diff_index_to_workdir(None, None).unwrap();
        std::thread::scope(|s| {
            s.spawn(move || { for _ in 0..2000 { let _ = diff.stats(); } });
            for _ in 0..200 { let _ = r.statuses(None); }
        });
        p("compiled and ran: Diff used on one thread while its Repository is used on another");
    }
    "stash_message" => {
        let (p0, mut r) = repo("sm");
        fs::write(p0.join("file.txt"), b"x\n").unwrap();
        let mut o = StashSaveOptions::new(sig());
        r.stash_save_ext(Some(&mut o)).unwrap();
        r.stash_foreach(|_, m, _| { println!("message {:?}", m); true }).unwrap();
    }
    "crafted_index" => {
        let ix = Index::open(Path::new("/work/tmp/crafted_index")).unwrap();
        p(&format!("entries={}", ix.len()));
        let e = ix.get(0).unwrap();
        p(&format!("flags={:#x} path.len()={} first bytes={:?}", e.flags, e.path.len(), &e.path[..16.min(e.path.len())]));
        p(&format!("non-NUL tail bytes leaked: {}", e.path.iter().skip(2).filter(|b| **b != 0).count()));
    }
    "empty_contents" => {
        let (_p, r) = repo("ec");
        let id = r.blob(b"").unwrap();
        p(&format!("empty blob len={}", r.find_blob(id).unwrap().content().len()));
        let odb = r.odb().unwrap();
        p(&format!("odb read len={}", odb.read(id).unwrap().data().len()));
        let m = Odb::new().unwrap();
        let _mp = m.add_new_mempack_backend(1).unwrap();
        let mid = m.write(ObjectType::Blob, b"").unwrap();
        p(&format!("mempack read len={}", m.read(mid).unwrap().data().len()));
        let e = |id: Oid, st: u16| IndexEntry{ctime:IndexTime::new(0,0),mtime:IndexTime::new(0,0),dev:0,ino:0,mode:0o100644,uid:0,gid:0,file_size:0,id,flags:st<<12,flags_extended:0,path:b"f".to_vec()};
        let res = r.merge_file_from_index(&e(id,1), &e(id,2), &e(id,3), None).unwrap();
        p(&format!("merge content len={}", res.content().len()));
        let p1 = Patch::from_buffers(b"", None, b"", None, None).unwrap();
        p(&format!("empty patch hunks={}", p1.num_hunks()));
        let d = Diff::from_buffer(b"").unwrap();
        p(&format!("empty diff deltas={}", d.deltas().len()));
        let mut pm = Patch::from_buffers(b"a", None, b"a", None, None).unwrap();
        p(&format!("unchanged patch to_buf len={}", pm.to_buf().unwrap().len()));
        let (_p3, r3) = repo("ec2");
        let mut pb = r3.packbuilder().unwrap(); let mut b = Buf::new(); pb.write_buf(&mut b).unwrap();
        p(&format!("empty pack buf len={}", b.len()));
    }
    "cred_port_key" => {
        let d = tmp("cpk"); use std::os::unix::fs::PermissionsExt;
        let a = d.join("a.sh"); fs::write(&a, "#!/bin/sh\ncat >/dev/null\necho username=PORTLESS\necho password=p\n").unwrap();
        let b = d.join("b.sh"); fs::write(&b, "#!/bin/sh\ncat >/dev/null\necho username=PORTED\necho password=p\n").unwrap();
        for s in [&a,&b] { fs::set_permissions(s, fs::Permissions::from_mode(0o755)).unwrap(); }
        let f = d.join("cfg");
        fs::write(&f, format!("[credential \"https://example.com\"]\n\thelper = !{}\n", a.display())).unwrap();
        let c = Config::open(&f).unwrap();
        let mut h = CredentialHelper::new("https://example.com:8443/r.git"); h.config(&c);
        p(&format!("portless section applied to :8443 URL -> {:?}", h.execute()));
        fs::write(&f, format!("[credential \"https://example.com:8443\"]\n\thelper = !{}\n", b.display())).unwrap();
        let c = Config::open(&f).unwrap();
        let mut h = CredentialHelper::new("https://example.com:8443/r.git"); h.config(&c);
        p(&format!("matching host:port section -> {:?}", h.execute()));
    }
    "dry_run_conflict" => {
        let (p0, r) = repo("drc");
        let c1 = r.head().unwrap().peel_to_commit().unwrap();
        let c2 = commit_file(&r, "file.txt", b"second\n", "2");
        fs::write(p0.join("file.txt"), b"dirty\n").unwrap();
        let obj = c1.as_object();
        let mut co = build::CheckoutBuilder::new();
        p(&format!("safe: {:?}", r.checkout_tree(obj, Some(&mut co)).map_err(|e| e.to_string())));
        let mut co = build::CheckoutBuilder::new(); co.dry_run();
        p(&format!("dry_run: {:?}", r.checkout_tree(obj, Some(&mut co)).map_err(|e| e.to_string())));
    }
    "mempack_reset_race" => {
        let odb = Odb::new().unwrap();
        let m = odb.add_new_mempack_backend(1).unwrap();
        std::thread::scope(|s| {
            let o = &odb;
            s.spawn(move || { for i in 0..400_000u32 { let d = format!("{}", i); let _ = o.write(ObjectType::Blob, d.as_bytes()); } });
            for _ in 0..200_000 { let _ = m.reset(); }
        });
        p("no crash");
    }
    "reflog_after_repo" => {
        let (p0, r0) = repo("rl");
        let id = r0.head().unwrap().target().unwrap();
        drop(r0);
        let mut log = { let r = Repository::open(&p0).unwrap(); r.reflog("refs/heads/master").or_else(|_| r.reflog("HEAD")).unwrap() };
        let junk: Vec<Vec<u8>> = (0..300).map(|_| vec![0x41u8; 256]).collect();
        p(&format!("len={}", log.len()));
        let s = sig();
        p(&format!("append={:?}", log.append(id, &s, Some("m")).is_ok()));
        p(&format!("write={:?}", log.write().map_err(|e| e.to_string())));
        drop(junk);
    }
    "stats_after_repo" => {
        let (p0, r0) = repo("sar");
        fs::write(p0.join("file.txt"), b"line1\nchanged\nline3\n").unwrap();
        drop(r0);
        let stats = { let r = Repository::open(&p0).unwrap(); let d = r.diff_index_to_workdir(None, None).unwrap(); d.stats().unwrap() };
        let junk: Vec<Vec<u8>> = (0..300).map(|_| vec![0x41u8; 256]).collect();
        p(&format!("{:?}", stats.to_buf(DiffStatsFormat::FULL, 80).map(|b| String::from_utf8_lossy(&b).to_string()).map_err(|e| e.to_string())));
        drop(stats);
        drop(junk);
        p("stats dropped");
    }
    "pathspec_workdir_after_repo" => {
        let (p0, r0) = repo("pwa");
        drop(r0);
        let ps = Pathspec::new(["*"]).unwrap();
        let l = { let r = Repository::open(&p0).unwrap(); ps.match_workdir(&r, PathspecFlags::DEFAULT).unwrap() };
        p(&format!("{:?}", l.entries().map(|x| x.to_vec()).collect::<Vec<_>>().len()));
    }
    "index_after_repo" => {
        let (p0, r0) = repo("iar");
        drop(r0);
        let mut ix = { let r = Repository::open(&p0).unwrap(); r.index().unwrap() };
        let junk: Vec<Vec<u8>> = (0..300).map(|_| vec![0x41u8; 256]).collect();
        p(&format!("add_path {:?}", ix.add_path(Path::new("file.txt")).map_err(|e| e.to_string())));
        p(&format!("write {:?}", ix.write().map_err(|e| e.to_string())));
        p(&format!("write_tree {:?}", ix.write_tree().map_err(|e| e.to_string())));
        drop(junk);
    }
    "config_after_repo" => {
        let (p0, r0) = repo("car");
        drop(r0);
        let mut c = { let r = Repository::open(&p0).unwrap(); r.config().unwrap() };
        p(&format!("{:?}", c.set_str("a.b", "c").map_err(|e| e.to_string())));
    }
    "worktree_after_repo" => {
        let (p0, r0) = repo("war");
        let wtp = tmp("war2").join("wt");
        r0.worktree("wt", &wtp, None).unwrap();
        drop(r0);
        let w = { let r = Repository::open(&p0).unwrap(); r.find_worktree("wt").unwrap() };
        let junk: Vec<Vec<u8>> = (0..300).map(|_| vec![0x41u8; 256]).collect();
        p(&format!("{:?} {:?} {:?}", w.validate().is_ok(), w.is_locked(), w.is_prunable(None).is_ok()));
        drop(junk);
    }
    "packbuilder_write_leak" => {
        let (_p, r) = repo("pwl");
        let head = r.head().unwrap().target().unwrap();
        let mut pb = r.packbuilder().unwrap(); pb.insert_commit(head).unwrap();
        let d = tmp("pwl2");
        pb.write(&d, 0).unwrap();
    }
    "tree_mode_panic" => {
        let (_p, r) = repo("tm");
        let odb = r.odb().unwrap();
        let (a, b) = (r.blob(b"a\n").unwrap(), r.blob(b"b\n").unwrap());
        let mk = |id: Oid, mode: &str| { let mut v = format!("{} f\0", mode).into_bytes(); v.extend_from_slice(id.as_bytes()); odb.write(ObjectType::Tree, &v).unwrap() };
        for mode in ["100600", "100664", "100640"] {
            let (t1, t2) = (r.find_tree(mk(a, mode)).unwrap(), r.find_tree(mk(b, mode)).unwrap());
            let d = r.diff_tree_to_tree(Some(&t1), Some(&t2), None).unwrap();
            let dd = d.get_delta(0).unwrap();
            p(&format!("tree mode {}: filemode={:o} filemode_raw={:o}", mode, t1.get(0).unwrap().filemode(), t1.get(0).unwrap().filemode_raw()));
            if mode != "100664" { p(&format!("  DiffFile::mode = {:?}", dd.new_file().mode())); }
            else { p(&format!("  DiffFile::mode = {:?}", dd.new_file().mode())); }
        }
    }
    "status_conflict" => {
        let (p0, r) = repo("sc");
        let base = r.head().unwrap().peel_to_commit().unwrap();
        let hd = r.head().unwrap().name().unwrap().to_string();
        let ours = commit_file(&r, "file.txt", b"ours\n", "o");
        r.set_head_detached(base.id()).unwrap();
        let mut co = build::CheckoutBuilder::new(); co.force(); r.checkout_head(Some(&mut co)).unwrap();
        let theirs = commit_file(&r, "file.txt", b"theirs\n", "t");
        r.set_head(&hd).unwrap();
        let mut co = build::CheckoutBuilder::new(); co.force(); r.checkout_head(Some(&mut co)).unwrap();
        let ac = r.find_annotated_commit(theirs).unwrap();
        r.merge(&[&ac], None, None).unwrap();
        fs::remove_file(p0.join("file.txt")).unwrap();
        let st = r.statuses(None).unwrap();
        for e in st.iter() { p(&format!("{:?} {:?}", e.path(), e.status())); }
        for c in r.index().unwrap().conflicts().unwrap() { let c = c.unwrap(); p(&format!("conflict our={:?}", c.our.map(|e| String::from_utf8_lossy(&e.path).to_string()))); }
    }
    "pack_foreach_empty" => {
        let (_p, r) = repo("pfe");
        let id = r.blob(b"").unwrap();
        let mut pb = r.packbuilder().unwrap(); pb.insert_object(id, None).unwrap();
        let mut n = 0; pb.foreach(|b| { n += b.len(); true }).unwrap();
        p(&format!("bytes {}", n));
    }
    "normalize_doc" => {
        p(&format!("foo/bar* -> {:?}", Reference::normalize_name("foo/bar*", ReferenceFormat::REFSPEC_PATTERN).map_err(|e| e.to_string())));
    }
    "fuzz_diff" => fuzz_diff(),
    "fuzz_small" => fuzz_small(),
    other => panic!("unknown probe {}", other),
    }
}

fn poison_check(r: &Repository) {
    // A later, unrelated callback on this thread:
    let mut ran = 0;
    let t = r.head().unwrap().peel_to_tree().unwrap();
    let res = t.walk(TreeWalkMode::PreOrder, |_, _| { ran += 1; TreeWalkResult::Ok });
    println!("later unrelated Tree::walk: callback ran {} times, result {:?}", ran, res.map_err(|e| e.to_string()));
}

fn walk_diff(d: &Diff<'_>) {
    for (i, dd) in d.deltas().enumerate() {
        let _ = dd.status(); let _ = dd.flags(); let _ = dd.nfiles();
        for f in [dd.old_file(), dd.new_file()] { let _ = f.path(); let _ = f.id(); let _ = f.size(); let _ = f.mode(); }
        if let Ok(Some(mut pa)) = Patch::from_diff(d, i) {
            let _ = pa.to_buf();
            for h in 0..pa.num_hunks() { let _ = pa.hunk(h); if let Ok(n) = pa.num_lines_in_hunk(h) { for l in 0..n { if let Ok(li) = pa.line_in_hunk(h, l) { let _ = li.origin(); let _ = li.content(); let _ = li.origin_value(); } } } }
        }
    }
    let _ = d.print(DiffFormat::Patch, |_, _, l| { let _ = l.origin_value(); let _ = l.content(); true });
    let _ = d.stats().map(|s| s.to_buf(DiffStatsFormat::FULL, 80));
}

fn fuzz_diff() {
    std::panic::set_hook(Box::new(|_| {}));
    let seeds: Vec<&[u8]> = vec![
        b"diff --git a/x b/x\nindex 257cc56..5716ca5 100644\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n",
        b"diff --git a/x b/y\nsimilarity index 90%\nrename from x\nrename to y\nold mode 100644\nnew mode 100755\n",
        b"diff --git a/x b/x\nnew file mode 100644\nindex 0000000..e69de29\n--- /dev/null\n+++ b/x\n@@ -0,0 +1 @@\n+a\n\\ No newline at end of file\n",
        b"diff --git a/b b/b\nindex 1234567..89abcde 100644\nGIT binary patch\nliteral 3\nKcmZQzU|?VY00aO5\n\nliteral 3\nKcmZQzU|?VY00aO5\n\n",
    ];
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut rng = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    let toks: Vec<&[u8]> = vec![b"100600",b"000000",b"120000",b"160000",b"040000",b"777777",b"\n",b" ",b"\xff",b"\0",b"@@ -1 +1 @@\n",b"+",b"-",b"rename from ",b"copy to ",b"deleted file mode ",b"GIT binary patch\n",b"delta 3\n",b"literal 99999\n"];
    let mut panics = std::collections::BTreeMap::<String,(usize,Vec<u8>)>::new();
    let n = 300_000usize;
    for it in 0..n {
        let mut b = seeds[(rng() as usize)%seeds.len()].to_vec();
        for _ in 0..(1 + rng()%4) {
            let at = (rng() as usize) % (b.len()+1);
            match rng()%4 {
                0 => { if at < b.len() { b[at] = rng() as u8; } }
                1 => { let t = toks[(rng() as usize)%toks.len()]; let e = (at+t.len()).min(b.len()); b.splice(at..e, t.iter().cloned()); }
                2 => { let t = toks[(rng() as usize)%toks.len()]; b.splice(at..at, t.iter().cloned()); }
                _ => { if at < b.len() { b.remove(at); } }
            }
        }
        let bb = b.clone();
        let r = std::panic::catch_unwind(move || { if let Ok(d) = Diff::from_buffer(&bb) { walk_diff(&d); } });
        if let Err(e) = r {
            let m = e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            let key = m.split(':').next().unwrap_or("").to_string();
            let en = panics.entry(key).or_insert((0,b.clone())); en.0 += 1; if b.len()<en.1.len() { en.1=b; }
        }
    }
    for (k,(c,b)) in &panics { println!("PANIC {:?} x{} smallest={:?}", k, c, String::from_utf8_lossy(b)); }
    println!("done {} iterations, {} distinct panics", n, panics.len());
}

fn fuzz_small() {
    std::panic::set_hook(Box::new(|_| {}));
    let alpha: &[u8] = b"a0<>@ \n\t#=:/.*\\\"'[]-+\xff\xc3\xa9";
    let mut count=0usize; let mut panics = std::collections::BTreeMap::<String,Vec<u8>>::new();
    let mut check = |name: &str, input: &[u8], f: &mut dyn FnMut()| {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f()));
        if r.is_err() { panics.entry(name.to_string()).or_insert(input.to_vec()); }
    };
    let mut inputs: Vec<Vec<u8>> = vec![vec![]];
    for len in 1..=3 { let mut cur = vec![vec![]]; for _ in 0..len { let mut nx = vec![]; for c in &cur { for &a in alpha { let mut d: Vec<u8> = c.clone(); d.push(a); nx.push(d); } } cur = nx; } inputs.extend(cur); }
    for i in &inputs {
        count += 1;
        let s = std::str::from_utf8(i).ok();
        check("message_prettify", i, &mut || { let _ = message_prettify(&i[..], Some(b'#')); });
        check("message_trailers_bytes", i, &mut || { let m = [b"x\n\nK: ".as_slice(), i.as_slice()].concat(); if let Ok(t) = message_trailers_bytes(&m[..]) { for _ in t.iter() {} } });
        check("Config::parse_bool", i, &mut || { let _ = Config::parse_bool(&i[..]); let _ = Config::parse_i32(&i[..]); let _ = Config::parse_i64(&i[..]); });
        if let Some(s) = s {
            let _ = s;
            check("Oid::from_str", i, &mut || { let _ = Oid::from_str(s); });
            check("Mailmap::from_buffer", i, &mut || { let _ = Mailmap::from_buffer(s); });
            check("message_trailers_strs", i, &mut || { let m = format!("x\n\nK: {}", s); if let Ok(t) = message_trailers_strs(&m) { for _ in t.iter() {} } });
            check("normalize_name", i, &mut || { let _ = Reference::normalize_name(s, ReferenceFormat::ALLOW_ONELEVEL|ReferenceFormat::REFSPEC_PATTERN); });
            check("Signature::new", i, &mut || { let _ = Signature::new(s, "e", &Time::new(0,0)); });
            check("CredentialHelper::new", i, &mut || { let mut h = CredentialHelper::new(s); });
            check("Pathspec::new", i, &mut || { let _ = Pathspec::new([s]); });
        }
    }
    for (k,v) in &panics { println!("PANIC {} smallest {:?}", k, String::from_utf8_lossy(v)); }
    println!("done {} inputs, {} panicking functions", count, panics.len());
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    run(&a[1]);
    println!("[probe finished normally]");
}
