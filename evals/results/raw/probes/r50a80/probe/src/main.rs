use git2::*;
use git2::build::CheckoutBuilder;
struct Counting;
static LIVE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 { LIVE.fetch_add(1, std::sync::atomic::Ordering::SeqCst); unsafe { std::alloc::System.alloc(l) } }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) { LIVE.fetch_sub(1, std::sync::atomic::Ordering::SeqCst); unsafe { std::alloc::System.dealloc(p, l) } }
}
#[global_allocator]
static A: Counting = Counting;
use std::fs;
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use tempfile::TempDir;

fn init() -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let repo = Repository::init(td.path()).unwrap();
    {
        let mut c = repo.config().unwrap();
        c.set_str("user.name", "n").unwrap();
        c.set_str("user.email", "e@x").unwrap();
    }
    (td, repo)
}
fn commit(repo: &Repository, files: &[(&[u8], &[u8])], msg: &str) -> Oid {
    let wd = repo.workdir().unwrap().to_path_buf();
    let mut idx = repo.index().unwrap();
    for (n, d) in files {
        let rel = Path::new(std::ffi::OsStr::from_bytes(n));
        let p = wd.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, d).unwrap();
        idx.add_path(rel).unwrap();
    }
    idx.write().unwrap();
    let t = idx.write_tree().unwrap();
    let tree = repo.find_tree(t).unwrap();
    let sig = repo.signature().unwrap();
    let parents: Vec<Commit> = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .map(|o| repo.find_commit(o).unwrap())
        .into_iter()
        .collect();
    let pr: Vec<&Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &pr).unwrap()
}

fn main() {
    let name = std::env::args().nth(1).unwrap();
    match name.as_str() {
        "buf_empty" => {
            let b = Buf::new();
            println!("len {}", b.len());
        }
        "config_empty" => {
            let td = TempDir::new().unwrap();
            let p = td.path().join("c");
            fs::write(&p, "[a]\n\tb =\n").unwrap();
            let c = Config::open(&p).unwrap();
            println!("{:?}", c.get_string("a.b"));
        }
        "config_empty_path" => {
            let td = TempDir::new().unwrap();
            let p = td.path().join("c");
            fs::write(&p, "[a]\n\tb =\n").unwrap();
            let c = Config::open(&p).unwrap();
            println!("{:?}", c.get_path("a.b"));
        }
        "prettify_empty" => println!("{:?}", message_prettify("", None)),
        "prettify_latin1" => println!("{:?}", message_prettify(&b"caf\xe9\n"[..], None)),
        "merge_msg_empty" => {
            let (_td, repo) = init();
            fs::write(repo.path().join("MERGE_MSG"), "").unwrap();
            println!("{:?}", repo.message());
        }
        "merge_msg_latin1" => {
            let (_td, repo) = init();
            fs::write(repo.path().join("MERGE_MSG"), b"caf\xe9\n").unwrap();
            println!("{:?}", repo.message());
        }
        "blame_buffer" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            let b = repo.blame_file(Path::new("f"), None).unwrap();
            let b2 = b.blame_buffer(b"a\nnew\n").unwrap();
            for h in b2.iter() {
                println!("hunk {} {}", h.final_commit_id(), h.lines_in_hunk());
                println!("sig {:?}", h.final_signature().name());
            }
        }
        "refspec_dangling" => {
            let (_td, repo) = init();
            repo.remote("origin", "file:///nonexistent").unwrap();
            let spec = {
                let r = repo.find_remote("origin").unwrap();
                r.get_refspec(0).unwrap()
            };
            println!("{:?}", spec.str());
        }
        "namespace_uaf" => {
            let (_td, repo) = init();
            repo.set_namespace("first-namespace-value").unwrap();
            let ns = repo.namespace().unwrap();
            repo.set_namespace("other-namespace-value").unwrap();
            println!("{:?}", ns);
        }
        "workdir_uaf" => {
            let (_td, repo) = init();
            let other = TempDir::new().unwrap();
            let wd = repo.workdir().unwrap();
            repo.set_workdir(other.path(), false).unwrap();
            println!("{:?}", wd);
        }
        "attr_uaf" => {
            let (td, repo) = init();
            fs::write(td.path().join(".gitattributes"), "f foo=aaaaaaaaaaaaaaaa\n").unwrap();
            let f = AttrCheckFlags::default();
            let v = repo.get_attr(Path::new("f"), "foo", f).unwrap().unwrap();
            println!("first {:?}", v);
            std::thread::sleep(std::time::Duration::from_millis(1100));
            fs::write(td.path().join(".gitattributes"), "f foo=bbbbbbbbbbbbbbbb\n").unwrap();
            let w = repo.get_attr(Path::new("f"), "foo", f).unwrap().unwrap();
            println!("second {:?} first-now {:?}", w, v);
        }
        "odb_foreach_panic" => {
            let (_td, repo) = init();
            repo.blob(b"x").unwrap();
            let odb = repo.odb().unwrap();
            let r = odb.foreach(|_| panic!("boom"));
            println!("foreach returned {:?}", r);
            let mut n = 0;
            let r = odb.foreach(|_| { n += 1; true });
            println!("second foreach {:?} visited {}", r, n);
            let r = Repository::open("/nonexistent/path");
            println!("unrelated open {:?}", r.is_err());
        }
        "stash_foreach_panic" => {
            let (td, mut repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let sig = repo.signature().unwrap();
            repo.stash_save(&sig, "m", None).unwrap();
            let r = repo.stash_foreach(|_, _, _| panic!("boom"));
            println!("stash_foreach returned {:?}", r);
        }
        "stash_latin1" => {
            let (td, mut repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let sig = repo.signature().unwrap();
            repo.stash_save(&sig, "msg", None).unwrap();
            let rl = repo.path().join("logs/refs/stash");
            let mut d = fs::read(&rl).unwrap();
            let pos = d.windows(3).position(|w| w == b"msg").unwrap();
            d[pos + 1] = 0xe9;
            fs::write(&rl, d).unwrap();
            let mut n = 0;
            let r = repo.stash_foreach(|_, _, _| { n += 1; true });
            println!("stash_foreach returned {:?} visited {}", r, n);
        }
        "fetchhead_latin1" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(repo.path().join("FETCH_HEAD"),
              [format!("{}\t\tbranch 'caf", id).as_bytes(), b"\xe9", b"' of file:///x\n"].concat()).unwrap();
            let mut n = 0;
            let r = repo.fetchhead_foreach(|_, _, _, _| { n += 1; true });
            println!("fetchhead_foreach returned {:?} visited {}", r, n);
        }
        "packwriter_panic" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            let (_td2, repo2) = init();
            let odb = repo2.odb().unwrap();
            let mut pw = odb.packwriter().unwrap();
            pw.progress(|_| panic!("boom"));
            let r = pw.write(&buf);
            println!("write {:?}", r);
            let r = pw.commit();
            println!("commit {:?}", r);
        }
        "revwalk_panic" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            let id = commit(&repo, &[(b"f", b"b\n")], "d");
            let mut w = repo.revwalk().unwrap();
            w.push(id).unwrap();
            let mut cb = |_: Oid| -> bool { panic!("boom") };
            let w = w.with_hide_callback(&mut cb).unwrap();
            let v: Vec<_> = w.collect();
            println!("collected {:?}", v);
            println!("unrelated {:?}", Oid::from_str("zz").is_err());
        }
        "remote_list_empty" => {
            let (_td, repo) = init();
            let td2 = TempDir::new().unwrap();
            Repository::init_bare(td2.path()).unwrap();
            let mut r = repo.remote("o", td2.path().to_str().unwrap()).unwrap();
            r.connect(Direction::Fetch).unwrap();
            println!("{:?}", r.list().map(|l| l.len()));
        }
        "remote_list_latin1" => {
            let (_td, repo) = init();
            let (td2, repo2) = init();
            let id = commit(&repo2, &[(b"f", b"a\n")], "c");
            let rd = td2.path().join(".git/refs/heads");
            let p = rd.join(std::ffi::OsStr::from_bytes(b"caf\xe9"));
            fs::write(&p, format!("{}\n", id)).unwrap();
            let mut r = repo.remote("o", td2.path().to_str().unwrap()).unwrap();
            r.connect(Direction::Fetch).unwrap();
            for h in r.list().unwrap() {
                println!("{:?}", h.name());
            }
        }
        "tree_nul" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let t = repo.find_commit(id).unwrap().tree().unwrap();
            println!("{:?}", t.get_name("a\0b").is_some());
        }
        "index_get_path_empty" => {
            let (_td, repo) = init();
            println!("{:?}", repo.index().unwrap().get_path(Path::new(""), 0).is_some());
        }
        "index_get_path_abs" => {
            let (_td, repo) = init();
            println!("{:?}", repo.index().unwrap().get_path(Path::new("/etc/passwd"), 0).is_some());
        }
        "ref_valid_nul" => println!("{:?}", Reference::is_valid_name("refs/heads/a\0b")),
        "remote_valid_nul" => println!("{:?}", Remote::is_valid_name("a\0b")),
        "tag_valid_nul" => println!("{:?}", Tag::is_valid_name("a\0b")),
        "txn_nul" => {
            let (_td, repo) = init();
            let mut t = repo.transaction().unwrap();
            println!("{:?}", t.lock_ref("refs/heads/a\0b"));
        }
        "tree_walk_latin1" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"caf\xe9/f", b"a\n"), (b"g", b"b\n")], "c");
            let t = repo.find_commit(id).unwrap().tree().unwrap();
            let mut names = vec![];
            let r = t.walk(TreeWalkMode::PreOrder, |root, e| {
                names.push(format!("{}{:?}", root, e.name()));
                TreeWalkResult::Ok
            });
            println!("{:?} {:?}", r, names);
        }
        "worktree_lock_latin1" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            let wtd = TempDir::new().unwrap();
            let wt = repo.worktree("w", &wtd.path().join("w"), None).unwrap();
            fs::write(td.path().join(".git/worktrees/w/locked"), b"caf\xe9").unwrap();
            println!("{:?}", wt.is_locked());
        }
        "describe_latin1" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let rp = repo.path().join("refs/tags");
            fs::write(rp.join(std::ffi::OsStr::from_bytes(b"caf\xe9")), format!("{}\n", id)).unwrap();
            let mut o = DescribeOptions::new();
            o.describe_tags();
            let d = repo.describe(&o).unwrap();
            println!("{:?}", d.format(None));
        }
        "pack_hash" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            #[allow(deprecated)]
            { println!("hash={:?} name={:?} count={}", b.hash(), b.name(), b.object_count()); }
        }
        "format_email" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "first");
            let id = commit(&repo, &[(b"f", b"b\n")], "line one\nline two\n\nbody\n");
            let c = repo.find_commit(id).unwrap();
            let t = c.tree().unwrap();
            let pt = c.parent(0).unwrap().tree().unwrap();
            let mut d = repo.diff_tree_to_tree(Some(&pt), Some(&t), None).unwrap();
            #[allow(deprecated)]
            { println!("{:?}", d.format_email(1, 1, &c, None).map(|b| b.len())); }
        }
        "diff_binary_cb" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\0b\n")], "first");
            let id = commit(&repo, &[(b"f", b"a\0c\n")], "second");
            let c = repo.find_commit(id).unwrap();
            let t = c.tree().unwrap();
            let pt = c.parent(0).unwrap().tree().unwrap();
            let d = repo.diff_tree_to_tree(Some(&pt), Some(&t), None).unwrap();
            let mut n = 0;
            let r = d.foreach(&mut |_, _| true, Some(&mut |_, b| {
                n += b.old_file().data().len();
                true
            }), None, None);
            println!("{:?} {}", r, n);
        }
        "parent_trunc" => {
            let (_td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "first");
            let id = commit(&repo, &[(b"f", b"b\n")], "second");
            let c = repo.find_commit(id).unwrap();
            println!("parent {:?} id {:?} expect {}", c.parent(1usize << 32).map(|p| p.id()), c.parent_id(1usize << 32), a);
        }
        "odb_double_finalize" => {
            let (_td, repo) = init();
            let odb = repo.odb().unwrap();
            let mut w = odb.writer(3, ObjectType::Blob).unwrap();
            w.write_all(b"abc").unwrap();
            let a = w.finalize();
            println!("first {:?}", a);
            let b = w.finalize();
            println!("second {:?}", b);
            let n = w.write(b"");
            println!("write {:?}", n);
        }
        "revwalk_into_inner" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            let id = commit(&repo, &[(b"f", b"b\n")], "d");
            let walk = {
                let captured = vec![1u8; 64];
                let mut cb = move |_: Oid| -> bool { println!("hide cb sees {:?}", &captured[..4]); false };
                let w = repo.revwalk().unwrap().with_hide_callback(&mut cb).unwrap();
                w.into_inner().unwrap()
            };
            let mut walk = walk;
            walk.push(id).unwrap();
            let v: Vec<_> = walk.collect();
            println!("collected {:?}", v.len());
        }
        "stash_moved_opts" => {
            fn make<'a>() -> StashApplyOptions<'a> {
                let mut o = StashApplyOptions::new();
                o.progress_cb(|_| { println!("progress"); true });
                o
            }
            let (td, mut repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let sig = repo.signature().unwrap();
            repo.stash_save(&sig, "m", None).unwrap();
            let mut o = make();
            fn clobber() -> u64 { let a = std::hint::black_box([0x41u8; 4096]); a.iter().map(|x| *x as u64).sum() }
            println!("{}", clobber());
            println!("apply {:?}", repo.stash_apply(0, Some(&mut o)));
        }
        "apply_moved_opts" => {
            fn make<'a>() -> ApplyOptions<'a> {
                let mut o = ApplyOptions::new();
                o.delta_callback(|_| { println!("delta"); true });
                o
            }
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let d = repo.diff_index_to_workdir(None, None).unwrap();
            let mut o = make();
            fn clobber() -> u64 { let a = std::hint::black_box([0x41u8; 4096]); a.iter().map(|x| *x as u64).sum() }
            println!("{}", clobber());
            println!("apply {:?}", repo.apply(&d, ApplyLocation::Index, Some(&mut o)));
        }
        "rebase_op_dangling" | "rebase_exec" => {
            let (_td, repo) = init();
            let base = commit(&repo, &[(b"f", b"a\n")], "base");
            let head_branch = repo.head().unwrap().name().unwrap().to_string();
            let _tip = commit(&repo, &[(b"g", b"b\n")], "tip");
            repo.branch("up", &repo.find_commit(base).unwrap(), true).unwrap();
            repo.set_head("refs/heads/up").unwrap();
            repo.checkout_head(Some(CheckoutBuilder::new().force())).unwrap();
            commit(&repo, &[(b"h", b"c\n")], "up");
            let one = repo.reference_to_annotated_commit(&repo.find_reference(&head_branch).unwrap()).unwrap();
            let up = repo.reference_to_annotated_commit(&repo.head().unwrap()).unwrap();
            let mut opts = RebaseOptions::new();
            opts.inmemory(true);
            if name == "rebase_exec" {
                let mut r = repo.rebase(Some(&one), Some(&up), None, Some(&mut opts)).unwrap();
                let op = r.next().unwrap().unwrap();
                println!("{:?}", op.kind());
                println!("{:?}", op.exec());
            } else {
                let op = {
                    let mut r = repo.rebase(Some(&one), Some(&up), None, Some(&mut opts)).unwrap();
                    r.next().unwrap().unwrap()
                };
                println!("{:?} {:?}", op.kind(), op.id());
            }
        }
        "annotated_refname" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let a = repo.find_annotated_commit(id).unwrap();
            println!("{:?}", a.refname());
        }
        "tag_foreach" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            repo.tag_lightweight("t1", &repo.find_object(id, None).unwrap(), false).unwrap();
            let r = repo.tag_foreach(|_, _| panic!("boom"));
            println!("tag_foreach returned {:?}", r);
            let mut n = 0;
            let r = repo.tag_foreach(|_, _| { n += 1; true });
            println!("again {:?} visited {}", r, n);
        }
        "notify_cancel" => {
            let (td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            let _b = commit(&repo, &[(b"f", b"b\n")], "b");
            let mut cb = CheckoutBuilder::new();
            let mut called = 0;
            cb.notify_on(CheckoutNotificationType::all()).notify(|_, _, _, _, _| { called += 1; false });
            let r = repo.checkout_tree(&repo.find_object(a, None).unwrap(), Some(&mut cb));
            drop(cb);
            println!("called {} result {:?} file now {:?}", called, r, fs::read_to_string(td.path().join("f")));
        }
        "notify_panic" => {
            let (_td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            commit(&repo, &[(b"f", b"b\n")], "b");
            let mut cb = CheckoutBuilder::new();
            cb.notify_on(CheckoutNotificationType::all()).notify(|_, _, _, _, _| panic!("boom"));
            let r = repo.checkout_tree(&repo.find_object(a, None).unwrap(), Some(&mut cb));
            println!("result {:?}", r);
        }
        "recreate_order" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "a");
            fs::remove_file(td.path().join("f")).unwrap();
            let mut cb = CheckoutBuilder::new();
            cb.recreate_missing(true).safe();
            repo.checkout_head(Some(&mut cb)).unwrap();
            println!("recreate_missing(true).safe(): exists={}", td.path().join("f").exists());
            let mut cb = CheckoutBuilder::new();
            cb.safe().recreate_missing(true);
            repo.checkout_head(Some(&mut cb)).unwrap();
            println!("safe().recreate_missing(true): exists={}", td.path().join("f").exists());
        }
        "dry_run" => {
            let (td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            commit(&repo, &[(b"f", b"b\n")], "b");
            fs::write(td.path().join("f"), "dirty\n").unwrap();
            let o = repo.find_object(a, None).unwrap();
            let r = repo.checkout_tree(&o, Some(&mut CheckoutBuilder::new()));
            println!("safe: {:?}", r.map_err(|e| e.code()));
            let r = repo.checkout_tree(&o, Some(CheckoutBuilder::new().dry_run()));
            println!("dry_run: {:?}", r.map_err(|e| e.code()));
            let r = repo.checkout_tree(&o, Some(CheckoutBuilder::new().dry_run().force()));
            println!("dry_run.force: {:?} file {:?}", r.map_err(|e| e.code()), fs::read_to_string(td.path().join("f")));
        }
        "indexer_odb" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            let out = TempDir::new().unwrap();
            let mut idx = {
                let odb = Odb::new().unwrap();
                Indexer::new(Some(&odb), out.path(), 0, true).unwrap()
            };
            for _ in 0..50 { let _ = std::hint::black_box(vec![0u8; 256]); }
            println!("{:?}", idx.write(&buf).map(|_| ()));
            println!("{:?}", idx.commit());
        }
        "from_buffer_static" => {
            let v = b"diff --git a/f b/f\nindex 1111111..2222222 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n".to_vec();
            let d = Diff::from_buffer(&v).unwrap();
            drop(v);
            let mut out = vec![];
            d.print(DiffFormat::Patch, |_, _, l| { out.extend_from_slice(l.content()); true }).unwrap();
            println!("{}", String::from_utf8_lossy(&out));
        }
        "pack_write_leak" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let out = TempDir::new().unwrap();
            let before = LIVE.load(std::sync::atomic::Ordering::SeqCst);
            for _ in 0..2000 {
                let mut b = repo.packbuilder().unwrap();
                b.insert_commit(id).unwrap();
                b.write(out.path(), 0).unwrap();
            }
            let after = LIVE.load(std::sync::atomic::Ordering::SeqCst);
            println!("live rust allocs grew by {} after 2000 PackBuilder::write calls", after - before);
        }
        "refs_names_latin1" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            fs::write(repo.path().join("refs/heads").join(std::ffi::OsStr::from_bytes(b"caf\xe9")), format!("{}\n", id)).unwrap();
            let mut refs = repo.references().unwrap();
            let v: Vec<_> = refs.names().collect();
            println!("{:?}", v);
        }
        "update_tips_latin1" => {
            let (_td, repo) = init();
            let (td2, repo2) = init();
            let id = commit(&repo2, &[(b"f", b"a\n")], "c");
            fs::write(td2.path().join(".git/refs/heads").join(std::ffi::OsStr::from_bytes(b"caf\xe9")), format!("{}\n", id)).unwrap();
            let mut r = repo.remote("o", td2.path().to_str().unwrap()).unwrap();
            let mut cb = RemoteCallbacks::new();
            let mut n = 0;
            cb.update_tips(|_, _, _| { n += 1; true });
            let mut fo = FetchOptions::new();
            fo.remote_callbacks(cb);
            println!("fetch {:?}", r.fetch(&["refs/heads/*:refs/remotes/o/*"], Some(&mut fo), None));
        }
        "pathspec_empty" => {
            let ps = Pathspec::new(["*"].iter()).unwrap();
            println!("{:?}", ps.matches_path(Path::new(""), PathspecFlags::DEFAULT));
        }
        "submodule_fuzz" => {
            for (i, gm) in [
                "[submodule \"a\"]\n\tpath = x\n\turl = ../y\n",
                "[submodule \"a\"]\n\tpath = x\n",
                "[submodule \"a\"]\n\turl = ../y\n",
                "[submodule \"a/b\"]\n\tpath = x/y\n\turl = u\n",
                "[submodule \"a\"]\n\tpath = x\n\tupdate = bogus\n",
                "[submodule \"a\"]\n\tpath = x\n\tignore = bogus\n",
                "[submodule \"a\"]\n\tpath = x\n\tfetchRecurseSubmodules = bogus\n",
                "[submodule \"a\"]\n\tpath = x\n[submodule \"b\"]\n\tpath = x\n",
                "[submodule \"A\"]\n\tpath = x\n\turl=u\n",
                "[submodule \"a\"]\n\tpath = \n",
            ].iter().enumerate() {
                let (td, repo) = init();
                fs::write(td.path().join(".gitmodules"), gm).unwrap();
                println!("case {} -> {:?}", i, repo.submodules().map(|v| v.len()).map_err(|e| e.message().to_string()));
            }
        }
        "time_ord" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let t = repo.find_commit(id).unwrap().tree().unwrap().id();
            let body = format!("tree {}\nauthor a <a@x> 1000 -0000\ncommitter a <a@x> 1000 -0000\n\nm\n", t);
            let odb = repo.odb().unwrap();
            let c = odb.write(ObjectType::Commit, body.as_bytes()).unwrap();
            let c = repo.find_commit(c).unwrap();
            let a = c.time();
            let b = c.committer().when();
            println!("time.sign {} sig.when.sign {} eq {} cmp {:?}", a.sign(), b.sign(), a == b, a.cmp(&b));
        }
        "index_flags_overread" => {
            let p = std::env::args().nth(2).unwrap();
            let i = Index::open(Path::new(&p)).unwrap();
            println!("entries {}", i.len());
            let e = i.get(0).unwrap();
            println!("path len {} head {:?}", e.path.len(), &e.path[..e.path.len().min(32)]);
        }
        "transport_err_msg" => {
            unsafe { git2::transport::register("xx", |_| Err(Error::from_str("factory says no"))).unwrap(); }
            let (_td, repo) = init();
            let mut r = repo.remote("o", "xx://h/p").unwrap();
            println!("{:?}", r.connect(Direction::Fetch).map_err(|e| e.to_string()));
        }
        "remote_create_err_msg" => {
            let td = TempDir::new().unwrap();
            let (td2, repo2) = init();
            commit(&repo2, &[(b"f", b"a\n")], "c");
            let mut b = git2::build::RepoBuilder::new();
            b.remote_create(|_, _, _| Err(Error::from_str("create says no")));
            println!("{:?}", b.clone(td2.path().to_str().unwrap(), &td.path().join("x")).map(|_| ()).map_err(|e| e.to_string()));
        }
        "skip_reuc" => {
            let (_td, repo) = init();
            let base = commit(&repo, &[(b"f", b"base\n")], "base");
            let ours = commit(&repo, &[(b"f", b"ours\n")], "ours");
            repo.set_head_detached(base).unwrap();
            let theirs = commit(&repo, &[(b"f", b"theirs\n")], "theirs");
            let (o, t) = (repo.find_commit(ours).unwrap(), repo.find_commit(theirs).unwrap());
            let r = repo.merge_commits(&o, &t, None);
            println!("default: {:?}", r.map(|i| i.has_conflicts()).map_err(|e| e.code()));
            let mut m = MergeOptions::new();
            m.skip_reuc(true);
            let r = repo.merge_commits(&o, &t, Some(&m));
            println!("skip_reuc(true): {:?}", r.map(|i| i.has_conflicts()).map_err(|e| e.code()));
        }
        "cred_inject" => {
            let td = TempDir::new().unwrap();
            let log = td.path().join("stdin.log");
            let cfgp = td.path().join("cfg");
            fs::write(&cfgp, "").unwrap();
            let mut cfg = Config::open(&cfgp).unwrap();
            cfg.set_str("credential.helper", &format!("!f() {{ cat > {}; echo username=u; echo password=p; }}; f", log.display())).unwrap();
            let r = CredentialHelper::new("https://real.example/repo").config(&cfg).username(Some("a\nhost=evil.example")).execute();
            println!("{:?}", r);
            println!("helper stdin: {:?}", fs::read_to_string(&log));
            let r = CredentialHelper::new("https://10.0.0.1:8443/repo").config(&cfg).execute();
            println!("{:?}", r);
            println!("ip helper stdin: {:?}", fs::read_to_string(&log));
        }
        "cred_multi" => {
            let td = TempDir::new().unwrap();
            let cfgp = td.path().join("cfg");
            fs::write(&cfgp, "[credential]\n\thelper = \"!f() { echo username=u; echo password=p; }; f\"\n\thelper = \"!f() { exit 1; }; f\"\n").unwrap();
            let cfg = Config::open(&cfgp).unwrap();
            println!("{:?}", CredentialHelper::new("https://real.example/repo").config(&cfg).execute());
        }
        "cred_empty_nosh" => {
            let td = TempDir::new().unwrap();
            let cfgp = td.path().join("cfg");
            fs::write(&cfgp, "[credential]\n\thelper = !\n").unwrap();
            let cfg = Config::open(&cfgp).unwrap();
            unsafe { std::env::set_var("PATH", "/nonexistent"); }
            println!("{:?}", CredentialHelper::new("https://real.example/repo").config(&cfg).execute());
        }
        "http_username_newline" => {
            use std::io::{Read, Write};
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = l.local_addr().unwrap().port();
            std::thread::spawn(move || {
                for s in l.incoming() {
                    let mut s = s.unwrap();
                    let mut b = [0u8; 4096];
                    let _ = s.read(&mut b);
                    let _ = s.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                }
            });
            let (_td, repo) = init();
            let url = format!("http://a%0Ahost=evil.example@127.0.0.1:{}/r.git", port);
            let mut r = repo.remote_anonymous(&url).unwrap();
            let mut cb = RemoteCallbacks::new();
            cb.credentials(|u, name, _| {
                println!("callback url={:?} username_from_url={:?}", u, name);
                Err(Error::from_str("stop"))
            });
            let c = r.connect_auth(Direction::Fetch, Some(cb), None);
            println!("{:?}", c.map(|_| ()).map_err(|e| e.message().to_string()));
        }
        "empty_bufs" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let c = repo.find_commit(id).unwrap();
            let t = c.tree().unwrap();
            let d = repo.diff_tree_to_tree(Some(&t), Some(&t), None).unwrap();
            let which = std::env::args().nth(2).unwrap();
            match which.as_str() {
                "stats" => { let b = d.stats().unwrap().to_buf(DiffStatsFormat::FULL, 80).unwrap(); println!("{:?}", b.as_str()); }
                "patch" => { let mut p = Patch::from_buffers(b"a\n", None, b"a\n", None, None).unwrap(); let b = p.to_buf().unwrap(); println!("{:?}", b.as_str()); }
                "print" => { let b = d.stats().unwrap().to_buf(DiffStatsFormat::SHORT, 80).unwrap(); println!("{:?}", b.len()); }
                "blob" => { let i = repo.blob(b"").unwrap(); let b = repo.find_blob(i).unwrap(); println!("{:?}", b.content()); let odb = repo.odb().unwrap(); let o = odb.read(i).unwrap(); println!("{:?}", o.data()); }
                "merge_empty" => {
                    let e = repo.blob(b"").unwrap();
                    let mk = |id| IndexEntry { ctime: IndexTime::new(0,0), mtime: IndexTime::new(0,0), dev:0, ino:0, mode:0o100644, uid:0, gid:0, file_size:0, id, flags:0, flags_extended:0, path: b"f".to_vec() };
                    let (a, b, c2) = (mk(e), mk(e), mk(e));
                    let r = repo.merge_file_from_index(&a, &b, &c2, None).unwrap();
                    println!("{:?}", r.content());
                }
                "header" => {
                    let body = format!("tree {}\nauthor a <a@x> 1000 +0000\ncommitter a <a@x> 1000 +0000\nencoding \n\nm\n", t.id());
                    let oid = repo.odb().unwrap().write(ObjectType::Commit, body.as_bytes()).unwrap();
                    let c = repo.find_commit(oid).unwrap();
                    println!("{:?}", c.header_field_bytes("encoding").map(|b| b.len()));
                }
                _ => panic!(),
            }
        }
        "push_negotiation_empty" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "base");
            let td2 = TempDir::new().unwrap();
            Repository::init_bare(td2.path()).unwrap();
            let mut r = repo.remote("o", td2.path().to_str().unwrap()).unwrap();
            let mut cb = RemoteCallbacks::new();
            cb.push_negotiation(|u| { println!("neg {}", u.len()); Ok(()) });
            let mut po = PushOptions::new();
            po.remote_callbacks(cb);
            println!("{:?}", r.push::<&str>(&[], Some(&mut po)));
        }
        "transport_io_nul" => {
            use git2::transport::*;
            struct S;
            impl std::io::Read for S { fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { Err(std::io::Error::new(std::io::ErrorKind::Other, "bad\0byte")) } }
            impl std::io::Write for S { fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { Ok(b.len()) } fn flush(&mut self) -> std::io::Result<()> { Ok(()) } }
            struct T;
            impl SmartSubtransport for T {
                fn action(&self, _: &str, _: Service) -> Result<Box<dyn SmartSubtransportStream>, Error> { Ok(Box::new(S)) }
                fn close(&self) -> Result<(), Error> { Ok(()) }
            }
            unsafe { register("yy", |r| Transport::smart(r, false, T)).unwrap(); }
            let (_td, repo) = init();
            let mut r = repo.remote("o", "yy://h/p").unwrap();
            println!("{:?}", r.connect(Direction::Fetch).map_err(|e| e.to_string()));
        }
        "diff_mode_panic" => {
            let d = Diff::from_buffer(b"diff --git a/f b/f\nold mode 100600\nnew mode 100777\n").unwrap();
            for x in d.deltas() { println!("{:?} {:?}", x.status(), x.old_file().path()); println!("{:?}", x.old_file().mode()); }
        }
        "pathspec_diff_dangling" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "base");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let ps = Pathspec::new(["f"].iter()).unwrap();
            let list = {
                let d = repo.diff_index_to_workdir(None, None).unwrap();
                ps.match_diff(&d, PathspecFlags::DEFAULT).unwrap()
            };
            let e = list.diff_entry(0).unwrap();
            println!("{:?}", e.old_file().path());
        }
        "reflog_after_repo" => {
            let mut log = {
                let (_td, repo) = init();
                commit(&repo, &[(b"f", b"a\n")], "base");
                repo.reflog("HEAD").unwrap()
            };
            println!("entries {}", log.len());
            println!("{:?}", log.write());
        }
        "packwriter_double" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            let (_td2, repo2) = init();
            let odb = repo2.odb().unwrap();
            let mut pw = odb.packwriter().unwrap();
            pw.write_all(&buf).unwrap();
            println!("commit1 {:?}", pw.commit());
            println!("commit2 {:?}", pw.commit());
            println!("write2 {:?}", pw.write(&buf).map(|_| ()));
        }
        "packbuilder_reuse" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let id2 = commit(&repo, &[(b"f", b"b\n")], "d");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            println!("first {}", buf.len());
            println!("second write_buf {:?}", b.write_buf(&mut buf).map(|_| buf.len()));
            println!("insert after {:?}", b.insert_commit(id2));
            println!("third write_buf {:?}", b.write_buf(&mut buf).map(|_| buf.len()));
            println!("foreach {:?}", b.foreach(|_| true));
        }
        "rebase_after_finish" => {
            let (_td, repo) = init();
            let base = commit(&repo, &[(b"f", b"a\n")], "base");
            let head_branch = repo.head().unwrap().name().unwrap().to_string();
            let _tip = commit(&repo, &[(b"g", b"b\n")], "tip");
            repo.branch("up", &repo.find_commit(base).unwrap(), true).unwrap();
            repo.set_head("refs/heads/up").unwrap();
            repo.checkout_head(Some(CheckoutBuilder::new().force())).unwrap();
            commit(&repo, &[(b"h", b"c\n")], "up");
            let one = repo.reference_to_annotated_commit(&repo.find_reference(&head_branch).unwrap()).unwrap();
            let up = repo.reference_to_annotated_commit(&repo.head().unwrap()).unwrap();
            let mut r = repo.rebase(Some(&one), Some(&up), None, None).unwrap();
            let sig = repo.signature().unwrap();
            while let Some(op) = r.next() { op.unwrap(); r.commit(None, &sig, None).unwrap(); }
            println!("finish {:?}", r.finish(None));
            println!("finish2 {:?}", r.finish(None).map_err(|e| e.message().to_string()));
            println!("abort {:?}", r.abort().map_err(|e| e.message().to_string()));
            println!("next {:?}", r.next().map(|x| x.map(|o| o.id()).map_err(|e| e.message().to_string())));
            println!("commit {:?}", r.commit(None, &sig, None).map_err(|e| e.message().to_string()));
        }
        "diff_send_race" => {
            let (td, repo) = init();
            let mut files: Vec<(Vec<u8>, Vec<u8>)> = vec![(b".gitattributes".to_vec(), b"*.x diff=foo\n*.y diff=bar\n*.z diff=baz\n".to_vec())];
            for i in 0..200 { files.push((format!("f{}.{}", i, ["x","y","z"][i%3]).into_bytes(), b"a\n".to_vec())); }
            let refs: Vec<(&[u8], &[u8])> = files.iter().map(|(a,b)| (&a[..], &b[..])).collect();
            commit(&repo, &refs, "base");
            for i in 0..200 { fs::write(td.path().join(format!("f{}.{}", i, ["x","y","z"][i%3])), "b\n").unwrap(); }
            let d1 = repo.diff_index_to_workdir(None, None).unwrap();
            let d2 = repo.diff_index_to_workdir(None, None).unwrap();
            std::thread::scope(|s| {
                s.spawn(move || { let mut n = 0; d1.print(DiffFormat::Patch, |_, _, _| { n += 1; true }).unwrap(); println!("t1 {}", n); });
                s.spawn(move || { let mut n = 0; d2.print(DiffFormat::Patch, |_, _, _| { n += 1; true }).unwrap(); println!("t2 {}", n); });
            });
        }
        "opts_race" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            std::thread::scope(|s| {
                s.spawn(|| { for i in 0..2000 { git2::opts::enable_caching(i % 2 == 0); git2::opts::strict_hash_verification(i % 2 == 0); } });
                let p = repo.path().to_path_buf();
                s.spawn(move || { let r = Repository::open(p).unwrap(); for _ in 0..500 { let _ = r.find_commit(id); } });
            });
        }
        "index_cb_empty_pathspec" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "base");
            fs::write(td.path().join("f"), "b\n").unwrap();
            fs::write(td.path().join("g"), "c\n").unwrap();
            let mut idx = repo.index().unwrap();
            let mut seen = 0;
            let mut cb = |_: &Path, _: &[u8]| -> i32 { seen += 1; 0 };
            let which = std::env::args().nth(2).unwrap();
            let r = match which.as_str() {
                "add" => idx.add_all(Vec::<&str>::new(), IndexAddOption::DEFAULT, Some(&mut cb)),
                "update" => idx.update_all(Vec::<&str>::new(), Some(&mut cb)),
                _ => idx.remove_all(Vec::<&str>::new(), Some(&mut cb)),
            };
            println!("{:?} seen {}", r, seen);
        }
        "fetchhead_forms" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            for (i, form) in [
                format!("{}\n", id),
                format!("{}\t\thttps://example.com/x.git\n", id),
                format!("{}\t\tbranch 'main' of https://example.com/x\n", id),
                format!("{}\tnot-for-merge\tbranch 'main'\n", id),
                format!("{}\t\t'refs/x' of https://example.com/x\n", id),
                format!("{}\t\ttag 'v1' of https://example.com/x\n", id),
                format!("{}\t\tremote-tracking branch 'o/m' of https://example.com/x\n", id),
            ].iter().enumerate().filter(|(i, _)| std::env::args().nth(2).map_or(true, |w| w == i.to_string())) {
                fs::write(repo.path().join("FETCH_HEAD"), form).unwrap();
                let mut seen = vec![];
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    repo.fetchhead_foreach(|n, u, _, m| { seen.push((n.to_string(), String::from_utf8_lossy(u).to_string(), m)); true })
                }));
                println!("form {} -> {:?} seen {:?}", i, r.map(|x| x.is_ok()).map_err(|_| "panic"), seen);
            }
        }
        "stash_no_message" => {
            let (td, mut repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let sig = repo.signature().unwrap();
            repo.stash_save(&sig, "m", None).unwrap();
            let rl = repo.path().join("logs/refs/stash");
            let d = fs::read_to_string(&rl).unwrap();
            let line = d.split('\t').next().unwrap().to_string() + "\n";
            fs::write(&rl, line).unwrap();
            let mut n = 0;
            let r = repo.stash_foreach(|_, m, _| { n += 1; println!("{:?}", m); true });
            println!("{:?} {}", r, n);
        }
        "diff_stress" => {
            for round in 0..40 {
                let (td, repo) = init();
                let mut ga = String::new();
                for i in 0..64 { ga.push_str(&format!("*.e{} diff=drv{}\n", i, i)); }
                let mut files: Vec<(Vec<u8>, Vec<u8>)> = vec![(b".gitattributes".to_vec(), ga.into_bytes())];
                for i in 0..64 { files.push((format!("f.e{}", i).into_bytes(), b"a\n".to_vec())); }
                let refs: Vec<(&[u8], &[u8])> = files.iter().map(|(a,b)| (&a[..], &b[..])).collect();
                commit(&repo, &refs, "base");
                for i in 0..64 { fs::write(td.path().join(format!("f.e{}", i)), "b\n").unwrap(); }
                let mut ds: Vec<Diff> = (0..4).map(|_| repo.diff_index_to_workdir(None, None).unwrap()).collect();
                std::thread::scope(|s| {
                    for d in ds.drain(..) {
                        s.spawn(move || { let mut n = 0; d.print(DiffFormat::Patch, |_, _, _| { n += 1; true }).unwrap(); assert!(n > 0); });
                    }
                });
                if round % 10 == 0 { println!("round {}", round); }
            }
        }
        "parents_fused" => {
            let (_td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            let t = repo.find_commit(a).unwrap().tree().unwrap().id();
            let missing = Oid::from_str("1111111111111111111111111111111111111111").unwrap();
            let body = format!("tree {t}\nparent {missing}\nparent {a}\nauthor a <a@x> 1000 +0000\ncommitter a <a@x> 1000 +0000\n\nm\n");
            let c = repo.odb().unwrap().write(ObjectType::Commit, body.as_bytes()).unwrap();
            let c = repo.find_commit(c).unwrap();
            let mut it = c.parents();
            println!("len {} first {:?} second {:?}", it.len(), it.next().map(|p| p.id()), it.next().map(|p| p.id()));
        }
        "index_alias" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n"), (b"g", b"a\n")], "a");
            let a = repo.index().unwrap();
            let mut b = repo.index().unwrap();
            let mut it = a.iter();
            b.clear().unwrap();
            println!("{:?}", it.next().map(|e| e.path));
        }
        _ => panic!("unknown"),
    }
}
