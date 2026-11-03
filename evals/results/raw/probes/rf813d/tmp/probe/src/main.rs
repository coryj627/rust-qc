#![allow(deprecated, unused)]
use git2::build::{CheckoutBuilder, RepoBuilder, TreeUpdateBuilder};
use git2::*;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, panic};
use tempfile::TempDir;

fn repo_init() -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let repo = Repository::init(td.path()).unwrap();
    {
        let mut c = repo.config().unwrap();
        c.set_str("user.name", "n").unwrap();
        c.set_str("user.email", "e@x").unwrap();
    }
    commit_file(&repo, "seed", b"seed\n", "initial");
    (td, repo)
}

fn commit_file(repo: &Repository, name: &str, data: &[u8], msg: &str) -> Oid {
    let wd = repo.workdir().unwrap().to_path_buf();
    let p = wd.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, data).unwrap();
    let mut idx = repo.index().unwrap();
    idx.add_path(Path::new(name)).unwrap();
    idx.write().unwrap();
    let tid = idx.write_tree().unwrap();
    let tree = repo.find_tree(tid).unwrap();
    let sig = Signature::new("n", "e@x", &Time::new(1600000000, 0)).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap()
}

fn head(repo: &Repository) -> Commit<'_> {
    repo.head().unwrap().peel_to_commit().unwrap()
}

fn p_bitflags() {
    println!("Sort::NONE.is_none()={}", Sort::NONE.is_none());
    println!("MergeAnalysis::ANALYSIS_NONE.is_none()={}", MergeAnalysis::ANALYSIS_NONE.is_none());
    println!("MergePreference::NONE.is_none()={}", MergePreference::NONE.is_none());
    println!("DiffStatsFormat::NONE.is_none()={}", DiffStatsFormat::NONE.is_none());
    println!("IndexAddOption::DEFAULT.is_default()={}", IndexAddOption::DEFAULT.is_default());
    println!("PathspecFlags::DEFAULT.is_default()={}", PathspecFlags::DEFAULT.is_default());
    println!("StashFlags::DEFAULT.is_default()={}", StashFlags::DEFAULT.is_default());
    println!("StashApplyFlags::DEFAULT.is_default()={}", StashApplyFlags::DEFAULT.is_default());
    println!("RepositoryInitMode::SHARED_UMASK.is_shared_umask()={}", RepositoryInitMode::SHARED_UMASK.is_shared_umask());
}

fn p_buf_new() {
    let b = Buf::new();
    println!("about to deref empty Buf");
    println!("{:?}", b.as_str());
}

fn p_binary_nodata() {
    let (_td, repo) = repo_init();
    let a = head(&repo).tree().unwrap();
    commit_file(&repo, "bin", b"\0\x01\x02binary\0data", "bin");
    let b = head(&repo).tree().unwrap();
    let diff = repo.diff_tree_to_tree(Some(&a), Some(&b), None).unwrap();
    diff.foreach(
        &mut |_, _| true,
        Some(&mut |_d, bin| {
            println!("contains_data={}", bin.contains_data());
            println!("data len={}", bin.new_file().data().len());
            true
        }),
        None,
        None,
    )
    .unwrap();
}

fn p_remote_list_empty() {
    let (_td, repo) = repo_init();
    let td2 = TempDir::new().unwrap();
    let _empty = Repository::init_bare(td2.path()).unwrap();
    let url = format!("file://{}", td2.path().display());
    let mut r = repo.remote_anonymous(&url).unwrap();
    r.connect(Direction::Fetch).unwrap();
    println!("connected; listing");
    println!("n={}", r.list().unwrap().len());
}

fn p_refspec_uaf() {
    let (_td, repo) = repo_init();
    repo.remote("origin", "https://example.invalid/x").unwrap();
    let spec = {
        let r = repo.find_remote("origin").unwrap();
        r.get_refspec(0).unwrap()
    };
    println!("{:?}", spec.str());
}

fn p_index_cb_null(which: &str) {
    let (td, repo) = repo_init();
    fs::write(td.path().join("seed"), "changed\n").unwrap();
    let mut idx = repo.index().unwrap();
    let mut cb = |p: &Path, m: &[u8]| {
        println!("cb {:?} {:?}", p, m);
        0
    };
    let none: Vec<&str> = vec![];
    let r = match which {
        "add" => idx.add_all(none, IndexAddOption::DEFAULT, Some(&mut cb)),
        "update" => idx.update_all(none, Some(&mut cb)),
        _ => idx.remove_all(none, Some(&mut cb)),
    };
    println!("{:?}", r);
}

fn p_index_get_path() {
    let (_td, repo) = repo_init();
    let idx = repo.index().unwrap();
    println!("{:?}", idx.get_path(Path::new("./seed"), 0).is_some());
}

fn p_pathspec_matches() {
    let ps = Pathspec::new(["a"]).unwrap();
    println!("{:?}", ps.matches_path(Path::new("/abs/a"), PathspecFlags::DEFAULT));
}

fn p_prettify() {
    println!("{:?}", message_prettify(&b"caf\xe9\n"[..], None));
}

fn add_packed(repo: &Repository, name: &[u8]) {
    let id = head(repo).id();
    let p = repo.path().join("packed-refs");
    let mut s = fs::read(&p).unwrap_or_else(|_| b"# pack-refs with: peeled fully-peeled sorted \n".to_vec());
    s.extend_from_slice(id.to_string().as_bytes());
    s.push(b' ');
    s.extend_from_slice(name);
    s.push(b'\n');
    fs::write(p, s).unwrap();
}

fn set_bad_ref(repo: &Repository) {
    add_packed(repo, b"refs/heads/caf\xe9");
}

fn p_refnames() {
    let (_td, repo) = repo_init();
    set_bad_ref(&repo);
    let mut refs = repo.references().unwrap();
    for n in refs.names() {
        println!("{:?}", n);
    }
}

fn p_remotehead() {
    let (_td, src) = repo_init();
    set_bad_ref(&src);
    let td = TempDir::new().unwrap();
    let dst = Repository::init(td.path()).unwrap();
    let url = format!("file://{}", src.workdir().unwrap().display());
    let mut r = dst.remote_anonymous(&url).unwrap();
    r.connect(Direction::Fetch).unwrap();
    for h in r.list().unwrap() {
        println!("{}", h.name());
    }
}

fn p_annotated() {
    let (_td, repo) = repo_init();
    let id = head(&repo).id();
    let ac = repo.find_annotated_commit(id).unwrap();
    println!("{:?}", ac.refname());
}

fn make_branches(repo: &Repository) -> (Oid, Oid, String) {
    // main: base -> m ; side: base -> s
    let base = head(repo).id();
    let base_c = repo.find_commit(base).unwrap();
    repo.branch("side", &base_c, false).unwrap();
    let m = commit_file(repo, "m", b"m\n", "main work");
    let mainref = repo.head().unwrap().name().unwrap().to_string();
    repo.set_head("refs/heads/side").unwrap();
    repo.checkout_head(Some(CheckoutBuilder::new().force())).unwrap();
    let s = commit_file(repo, "s", b"s\n", "side work");
    (m, s, mainref)
}

fn start_rebase<'r>(repo: &'r Repository, opts: Option<&mut RebaseOptions<'_>>) -> Rebase<'r> {
    let (m, _s, mainref) = make_branches(repo);
    let onto = repo.find_annotated_commit(m).unwrap();
    let up = repo.find_annotated_commit(m).unwrap();
    let branch_ref = repo.find_reference("refs/heads/side").unwrap();
    let br = repo.reference_to_annotated_commit(&branch_ref).unwrap();
    repo.rebase(Some(&br), Some(&up), Some(&onto), opts).unwrap()
}

fn p_rebase_exec() {
    let (_td, repo) = repo_init();
    let mut rb = start_rebase(&repo, None);
    let op = rb.next().unwrap().unwrap();
    println!("kind={:?}", op.kind());
    println!("exec={:?}", op.exec());
}

fn p_rebase_op_uaf() {
    let (_td, repo) = repo_init();
    let mut rb = start_rebase(&repo, None);
    let op = rb.next().unwrap().unwrap();
    let before = op.id();
    drop(rb);
    let junk: Vec<Vec<u8>> = (0..200).map(|_| vec![0xAA; 64]).collect();
    println!("before={} after_drop={} (junk {})", before, op.id(), junk.len());
}

fn mk_rebase_opts<'a>() -> Box<RebaseOptions<'a>> {
    let mut opts = Box::new(RebaseOptions::new());
    let mut cb = CheckoutBuilder::new();
    let v = vec![1u8, 2, 3];
    cb.progress(move |_, _, _| {
        println!("progress cb sees {:?}", v);
    });
    opts.checkout_options(cb);
    opts
}

fn p_rebase_opts_uaf() {
    let (_td, repo) = repo_init();
    let mut opts = mk_rebase_opts();
    let mut rb = start_rebase(&repo, Some(&mut *opts));
    drop(opts);
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 96]).collect();
    println!("options dropped; junk {}", junk.len());
    let op = rb.next().unwrap();
    println!("next ok={}", op.is_ok());
}

fn mk_apply_opts<'a>() -> ApplyOptions<'a> {
    let mut o = ApplyOptions::new();
    o.delta_callback(|_| {
        println!("delta cb called");
        true
    });
    o
}

#[inline(never)]
fn clobber_stack() -> u64 {
    let mut a = [0xAAu64; 512];
    for x in a.iter_mut() {
        *x = std::hint::black_box(*x);
    }
    a[17]
}

fn p_apply_move() {
    let (td, repo) = repo_init();
    let a = head(&repo).tree().unwrap();
    commit_file(&repo, "seed", b"seed2\n", "c2");
    let b = head(&repo).tree().unwrap();
    let diff = repo.diff_tree_to_tree(Some(&b), Some(&a), None).unwrap();
    let mut boxed = Box::new(mk_apply_opts());
    clobber_stack();
    println!("applying with moved options");
    let r = repo.apply(&diff, ApplyLocation::Index, Some(&mut *boxed));
    println!("{:?}", r);
}

fn mk_stash_opts<'a>() -> StashApplyOptions<'a> {
    let mut o = StashApplyOptions::new();
    o.progress_cb(|_| {
        println!("stash cb called");
        true
    });
    o
}

fn p_stash_move() {
    let (td, mut repo) = repo_init();
    fs::write(td.path().join("seed"), "dirty\n").unwrap();
    let sig = Signature::now("n", "e@x").unwrap();
    repo.stash_save(&sig, "s", None).unwrap();
    let mut boxed = Box::new(mk_stash_opts());
    clobber_stack();
    println!("applying with moved options");
    let r = repo.stash_apply(0, Some(&mut *boxed));
    println!("{:?}", r);
}

fn p_blame_sig() {
    let (_td, repo) = repo_init();
    let blame = repo.blame_file(Path::new("seed"), None).unwrap();
    let b2 = blame.blame_buffer(b"seed\nextra\n").unwrap();
    for h in b2.iter() {
        println!("hunk {} start {}", h.final_commit_id(), h.final_start_line());
        println!("name={:?}", h.final_signature().name());
    }
}

fn p_skip_reuc() {
    let (_td, repo) = repo_init();
    let base = head(&repo);
    repo.branch("o", &base, false).unwrap();
    commit_file(&repo, "seed", b"ours\n", "ours");
    let ours = head(&repo);
    repo.set_head("refs/heads/o").unwrap();
    repo.checkout_head(Some(CheckoutBuilder::new().force())).unwrap();
    commit_file(&repo, "seed", b"theirs\n", "theirs");
    let theirs = head(&repo);
    let mut o = MergeOptions::new();
    o.skip_reuc(true);
    match repo.merge_commits(&ours, &theirs, Some(&o)) {
        Ok(i) => println!("ok conflicts={}", i.has_conflicts()),
        Err(e) => println!("Err: {}", e),
    }
}

fn two_commits(repo: &Repository) -> (Oid, Oid) {
    let a = head(repo).id();
    let b = commit_file(repo, "seed", b"two\n", "two");
    (a, b)
}

fn p_dry_run() {
    let (td, repo) = repo_init();
    let (a, b) = two_commits(&repo);
    // Make a local edit which conflicts with checking out `a`.
    fs::write(td.path().join("seed"), "local edit\n").unwrap();
    let obj = repo.find_object(a, None).unwrap();
    let mut n = 0;
    let mut cb = CheckoutBuilder::new();
    cb.notify_on(CheckoutNotificationType::CONFLICT | CheckoutNotificationType::DIRTY | CheckoutNotificationType::UPDATED);
    cb.notify(|t, p, _, _, _| {
        println!("notify {:?} {:?}", t, p);
        true
    });
    cb.dry_run();
    println!("dry_run with conflicting edit: {:?}", repo.checkout_tree(&obj, Some(&mut cb)).map_err(|e| e.to_string()));
    let mut safe = CheckoutBuilder::new();
    println!("safe (real): {:?}", repo.checkout_tree(&obj, Some(&mut safe)).map_err(|e| e.to_string()));
}

fn p_force_after_dry() {
    let (td, repo) = repo_init();
    let (a, _b) = two_commits(&repo);
    let obj = repo.find_object(a, None).unwrap();
    let mut cb = CheckoutBuilder::new();
    cb.dry_run().force();
    println!("{:?}", repo.checkout_tree(&obj, Some(&mut cb)).is_ok());
    println!("seed content after dry_run().force(): {:?}", fs::read_to_string(td.path().join("seed")).unwrap());
    let mut cb = CheckoutBuilder::new();
    cb.force();
    println!("{:?}", repo.checkout_tree(&obj, Some(&mut cb)).is_ok());
    println!("seed content after force(): {:?}", fs::read_to_string(td.path().join("seed")).unwrap());
}

fn p_recreate() {
    let (td, repo) = repo_init();
    fs::remove_file(td.path().join("seed")).unwrap();
    let mut cb = CheckoutBuilder::new();
    cb.recreate_missing(true).safe();
    repo.checkout_head(Some(&mut cb)).unwrap();
    println!("recreate_missing(true).safe(): exists={}", td.path().join("seed").exists());
    let mut cb = CheckoutBuilder::new();
    cb.safe().recreate_missing(true);
    repo.checkout_head(Some(&mut cb)).unwrap();
    println!("safe().recreate_missing(true): exists={}", td.path().join("seed").exists());
}

fn p_notify_cancel() {
    let (td, repo) = repo_init();
    let (a, _b) = two_commits(&repo);
    let obj = repo.find_object(a, None).unwrap();
    let mut cb = CheckoutBuilder::new();
    cb.force();
    cb.notify_on(CheckoutNotificationType::UPDATED);
    cb.notify(|t, p, _, _, _| {
        println!("notify {:?} {:?} -> false", t, p);
        false
    });
    let r = repo.checkout_tree(&obj, Some(&mut cb));
    println!("cancelled checkout returned {:?}", r);
    println!("seed={:?}", fs::read_to_string(td.path().join("seed")).unwrap());
    let mut cb = CheckoutBuilder::new();
    cb.force();
    cb.notify_on(CheckoutNotificationType::UPDATED);
    cb.notify(|_, _, _, _, _| panic!("panic in notify"));
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.checkout_tree(&obj, Some(&mut cb))));
    println!("panic in notify: caught_panic={} ", r.is_err());
    if let Ok(v) = r { println!(" returned {:?}", v); }
}

fn p_progress_panic() {
    let (_td, repo) = repo_init();
    let (a, _b) = two_commits(&repo);
    let obj = repo.find_object(a, None).unwrap();
    let mut cb = CheckoutBuilder::new();
    cb.force();
    cb.progress(|_, _, _| panic!("panic in progress"));
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.checkout_tree(&obj, Some(&mut cb))));
    println!("caught_panic={}", r.is_err());
    if let Ok(v) = r { println!("checkout returned {:?}", v); }
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.find_branch("no-such", BranchType::Local).map(|_| ())));
    println!("unrelated find_branch: panicked={}", r.is_err());
    if let Ok(v) = r { println!(" returned {:?}", v.map_err(|e| e.to_string())); }
}

fn p_foreach_panic() {
    let (td, mut repo) = repo_init();
    let id = head(&repo).id();
    {
        let odb = repo.odb().unwrap();
        let r = panic::catch_unwind(panic::AssertUnwindSafe(|| odb.foreach(|_| panic!("odb"))));
        println!("odb.foreach: caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
        println!("odb.foreach(false): {:?}", odb.foreach(|_| false));
    }
    let sig = Signature::now("n", "e@x").unwrap();
    fs::write(td.path().join("seed"), "dirty\n").unwrap();
    repo.stash_save(&sig, "s", None).unwrap();
    // clear any stale panic
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.find_branch("no", BranchType::Local).map(|_| ())));
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.stash_foreach(|_, _, _| panic!("stash"))));
    println!("stash_foreach: caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.find_branch("no", BranchType::Local).map(|_| ())));
    fs::write(repo.path().join("MERGE_HEAD"), format!("{}\n", id)).unwrap();
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.mergehead_foreach(|_| panic!("mh"))));
    println!("mergehead_foreach: caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.find_branch("no", BranchType::Local).map(|_| ())));
    fs::write(repo.path().join("FETCH_HEAD"), format!("{}\t\tbranch 'main' of /x\n", id)).unwrap();
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.fetchhead_foreach(|_, _, _, _| panic!("fh"))));
    println!("fetchhead_foreach: caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.find_branch("no", BranchType::Local).map(|_| ())));
    let obj = repo.find_object(id, None).unwrap();
    repo.tag_lightweight("t", &obj, false).unwrap();
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.tag_foreach(|_, _| panic!("tag"))));
    println!("tag_foreach: caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
}

fn p_tag_foreach_err() {
    let (_td, repo) = repo_init();
    let id = head(&repo).id();
    let obj = repo.find_object(id, None).unwrap();
    repo.tag_lightweight("t", &obj, false).unwrap();
    // A garbage tag ref makes git_tag_foreach fail; compare with the raw return via reference iteration.
    fs::write(repo.path().join("refs/tags/zzz"), "not-an-oid\n").unwrap();
    let mut seen = 0;
    let r = repo.tag_foreach(|_, _| { seen += 1; true });
    println!("tag_foreach on corrupt ref: seen={} ret={:?}", seen, r);
    let mut errs = 0;
    for e in repo.references().unwrap() { if e.is_err() { errs += 1; } }
    println!("references() errors={}", errs);
}

fn pack_of(repo: &Repository) -> Buf {
    let id = head(repo).id();
    let mut b = repo.packbuilder().unwrap();
    b.insert_commit(id).unwrap();
    let mut buf = Buf::new();
    b.write_buf(&mut buf).unwrap();
    buf
}

fn p_pack_panic() {
    let (_td, src) = repo_init();
    let buf = pack_of(&src);
    let td = TempDir::new().unwrap();
    let dst = Repository::init(td.path()).unwrap();
    let odb = dst.odb().unwrap();
    let mut pw = odb.packwriter().unwrap();
    pw.progress(|_| panic!("packwriter progress"));
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let a = pw.write(&buf).map_err(|e| e.to_string());
        let b = pw.commit().map_err(|e| e.to_string());
        (a, b)
    }));
    println!("packwriter: caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| dst.find_branch("no", BranchType::Local).map(|_| ())));
    println!("later unrelated find_branch panicked={}", r.is_err());
}

fn p_pathspec_diff_uaf() {
    let (_td, repo) = repo_init();
    let a = head(&repo).tree().unwrap();
    commit_file(&repo, "seed", b"changed\n", "c");
    let b = head(&repo).tree().unwrap();
    let ps = Pathspec::new(["seed"]).unwrap();
    let list = {
        let diff = repo.diff_tree_to_tree(Some(&a), Some(&b), None).unwrap();
        ps.match_diff(&diff, PathspecFlags::DEFAULT).unwrap()
    };
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 96]).collect();
    println!("junk {}", junk.len());
    let d = list.diff_entry(0).unwrap();
    println!("path after diff dropped: {:?}", d.new_file().path());
}

fn p_reflog_uaf() {
    let td = TempDir::new().unwrap();
    {
        let r = Repository::init(td.path()).unwrap();
        let mut c = r.config().unwrap();
        c.set_str("user.name", "n").unwrap();
        c.set_str("user.email", "e@x").unwrap();
        commit_file(&r, "seed", b"x\n", "m");
    }
    let mut log = {
        let repo = Repository::open(td.path()).unwrap();
        repo.reflog("HEAD").unwrap()
    };
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 128]).collect();
    println!("repo dropped, junk {}, writing reflog", junk.len());
    println!("{:?}", log.write());
}

fn p_workdir_uaf() {
    let (td, repo) = repo_init();
    let td2 = TempDir::new().unwrap();
    let w = repo.workdir().unwrap();
    let before = w.to_path_buf();
    repo.set_workdir(td2.path(), false).unwrap();
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; before.as_os_str().len() + 2]).collect();
    println!("before={:?} stale_borrow={:?} junk {}", before, w, junk.len());
}

fn p_namespace_uaf() {
    let (_td, repo) = repo_init();
    repo.set_namespace("alpha").unwrap();
    let n = repo.namespace_bytes().unwrap();
    println!("before {:?}", String::from_utf8_lossy(n));
    repo.remove_namespace().unwrap();
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 6]).collect();
    println!("stale_borrow {:?} junk {}", String::from_utf8_lossy(n), junk.len());
}

fn p_attr_uaf() {
    let (td, repo) = repo_init();
    fs::write(td.path().join(".gitattributes"), "seed myattr=valueone\n").unwrap();
    let v1 = repo.get_attr_bytes(Path::new("seed"), "myattr", AttrCheckFlags::default()).unwrap().unwrap();
    println!("v1={:?}", String::from_utf8_lossy(v1));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::write(td.path().join(".gitattributes"), "seed myattr=second-value-longer other=2\n").unwrap();
    let v2 = repo.get_attr_bytes(Path::new("seed"), "myattr", AttrCheckFlags::default()).unwrap().unwrap();
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 64]).collect();
    println!("v2={:?} stale v1={:?} junk {}", String::from_utf8_lossy(v2), String::from_utf8_lossy(v1), junk.len());
}

fn p_indexer_odb_uaf() {
    let (_td, src) = repo_init();
    let buf = pack_of(&src);
    let td = TempDir::new().unwrap();
    let mut idx = {
        let odb = Odb::new().unwrap();
        Indexer::new(Some(&odb), td.path(), 0, true).unwrap()
    };
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 128]).collect();
    println!("odb dropped; junk {}", junk.len());
    idx.write_all(&buf).unwrap();
    println!("{:?}", idx.commit().map_err(|e| e.to_string()));
}

fn p_hide_cb_uaf() {
    let (_td, repo) = repo_init();
    let c1 = head(&repo).id();
    let c2 = commit_file(&repo, "x", b"x\n", "x");
    let mut walk = {
        let v = vec![0x41u8; 32];
        let mut cb = Box::new(move |_o: Oid| {
            println!("hide cb invoked, v[0]={:#x}", v[0]);
            false
        });
        let w = repo.revwalk().unwrap().with_hide_callback(&mut *cb).unwrap();
        w.into_inner().unwrap()
    };
    let junk: Vec<Vec<u8>> = (0..400).map(|_| vec![0xAA; 32]).collect();
    println!("callback dropped, junk {}", junk.len());
    walk.push(c2).unwrap();
    walk.hide(c1).unwrap();
    for o in walk.by_ref() {
        println!("{:?}", o.map(|o| o.to_string()));
    }
}

fn p_diff_buffer_uaf() {
    let text = b"diff --git a/f b/f\nindex 1234567..89abcde 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old line\n+new line\n".to_vec();
    let mut owned = text.clone();
    let diff = Diff::from_buffer(&owned).unwrap();
    for b in owned.iter_mut() { *b = b'Z'; }
    drop(owned);
    let junk: Vec<Vec<u8>> = (0..50).map(|_| vec![0xAA; text.len()]).collect();
    println!("junk {}", junk.len());
    let mut out = Vec::new();
    diff.print(DiffFormat::Patch, |_, _, l| {
        out.extend_from_slice(l.content());
        true
    })
    .unwrap();
    println!("{}", String::from_utf8_lossy(&out));
}

fn p_diff_mode() {
    let text = b"diff --git a/f b/f\nold mode 100600\nnew mode 100644\n";
    let diff = Diff::from_buffer(text).unwrap();
    let d = diff.get_delta(0).unwrap();
    println!("status={:?}", d.status());
    println!("mode={:?}", d.old_file().mode());
}

fn helper_cfg(capture: &Path) -> (TempDir, Config) {
    let td = TempDir::new().unwrap();
    let script = td.path().join("helper.sh");
    fs::write(&script, format!("#!/bin/sh\ncat > {}\necho username=u\necho password=p\n", capture.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let mut cfg = Config::new().unwrap();
    cfg.add_file(&td.path().join("cfg"), ConfigLevel::App, false).unwrap();
    cfg.set_str("credential.helper", &format!("{}", script.display())).unwrap();
    (td, cfg)
}

fn p_cred_inject() {
    let t = TempDir::new().unwrap();
    let cap = t.path().join("cap");
    let (_td, cfg) = helper_cfg(&cap);
    let r = CredentialHelper::new("https://evil.example/repo")
        .username(Some("victim\nhost=github.com"))
        .config(&cfg)
        .execute();
    println!("result {:?}", r);
    println!("helper stdin:\n{}", fs::read_to_string(&cap).unwrap());
}

fn p_cred_ip() {
    let t = TempDir::new().unwrap();
    let cap = t.path().join("cap");
    let (_td, cfg) = helper_cfg(&cap);
    let r = CredentialHelper::new("https://203.0.113.9:8443/repo").config(&cfg).execute();
    println!("result {:?}", r);
    println!("helper stdin for IP url:\n{}", fs::read_to_string(&cap).unwrap());
    let r = CredentialHelper::new("https://[2001:db8::1]/repo").config(&cfg).execute();
    println!("helper stdin for IPv6 url:\n{}", fs::read_to_string(&cap).unwrap());
    // Real helper: git credential-store answers a host-less query with any stored credential.
    let store = t.path().join("store");
    fs::write(&store, "https://alice:s3cret@github.com\n").unwrap();
    let mut cfg2 = Config::new().unwrap();
    cfg2.add_file(&t.path().join("cfg2"), ConfigLevel::App, false).unwrap();
    cfg2.set_str("credential.helper", &format!("store --file={}", store.display())).unwrap();
    let r = CredentialHelper::new("https://203.0.113.9/repo").config(&cfg2).execute();
    println!("git credential-store for https://203.0.113.9/repo => {:?}", r);
    let r = CredentialHelper::new("https://other.example/repo").config(&cfg2).execute();
    println!("git credential-store for https://other.example/repo => {:?}", r);
}

fn p_remote_create_err() {
    let (_td, src) = repo_init();
    let td = TempDir::new().unwrap();
    let mut b = RepoBuilder::new();
    b.remote_create(|_, _, _| Err(Error::from_str("my helpful message")));
    let url = format!("file://{}", src.workdir().unwrap().display());
    match b.clone(&url, &td.path().join("c")) {
        Ok(_) => println!("ok?"),
        Err(e) => println!("clone Err: {:?} / {}", e.message(), e),
    }
}

fn p_transport_err() {
    unsafe {
        git2::transport::register("boom", |_r| Err(Error::from_str("factory says no"))).unwrap();
    }
    let (_td, repo) = repo_init();
    let mut r = repo.remote_anonymous("boom://x").unwrap();
    match r.connect(Direction::Fetch) {
        Ok(_) => println!("ok?"),
        Err(e) => println!("connect Err: {:?} / {}", e.message(), e),
    }
}

fn p_opts_timeout() {
    unsafe {
        let r = git2::opts::set_server_connect_timeout_in_milliseconds(-5);
        println!("{:?}", r);
        let r = git2::opts::set_server_timeout_in_milliseconds(-5);
        println!("{:?}", r);
    }
}

fn p_message_utf8() {
    let (_td, repo) = repo_init();
    fs::write(repo.path().join("MERGE_MSG"), b"caf\xe9\n").unwrap();
    println!("{:?}", repo.message());
}

fn p_note_ref_utf8() {
    let (_td, repo) = repo_init();
    let cfgp = repo.path().join("config");
    let mut s = fs::read(&cfgp).unwrap();
    s.extend_from_slice(b"[core]\n\tnotesRef = refs/notes/caf\xe9\n");
    fs::write(&cfgp, s).unwrap();
    println!("{:?}", repo.note_default_ref());
}

fn p_describe_utf8() {
    let (_td, repo) = repo_init();
    let id = head(&repo).id();
    add_packed(&repo, b"refs/tags/v\xe9");
    let mut o = DescribeOptions::new();
    o.describe_tags();
    let d = repo.describe(&o).unwrap();
    println!("{:?}", d.format(None));
}

fn p_lock_utf8() {
    let (td, repo) = repo_init();
    let t2 = TempDir::new().unwrap();
    let wt = repo.worktree("w", &t2.path().join("w"), None).unwrap();
    fs::write(repo.path().join("worktrees/w/locked"), b"caf\xe9").unwrap();
    println!("{:?}", wt.is_locked());
}

fn p_stash_msg_utf8() {
    let (td, mut repo) = repo_init();
    fs::write(td.path().join("seed"), "dirty\n").unwrap();
    let sig = Signature::now("n", "e@x").unwrap();
    repo.stash_save(&sig, "s", None).unwrap();
    // rewrite reflog message with invalid utf-8
    let p = repo.path().join("logs/refs/stash");
    let s = fs::read(&p).unwrap();
    let mut out = Vec::new();
    for b in s { if b == b's' { out.push(0xe9) } else { out.push(b) } }
    let tab = s_tab(&p);
    fs::write(&p, tab).unwrap();
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| repo.stash_foreach(|i, m, _| { println!("{} {:?}", i, m); true })));
    println!("caught_panic={} ret={:?}", r.is_err(), r.as_ref().ok());
}
fn s_tab(p: &Path) -> Vec<u8> {
    let s = fs::read(p).unwrap();
    let i = s.iter().position(|b| *b == b'\t').unwrap();
    let mut out = s[..=i].to_vec();
    out.extend_from_slice(b"caf\xe9 message\n");
    out
}

fn p_format_email() {
    let (_td, repo) = repo_init();
    let a = head(&repo).tree().unwrap();
    commit_file(&repo, "seed", b"changed\n", "subject line one\nsubject line two\n\nbody\n");
    let c = head(&repo);
    println!("summary={:?}", c.summary());
    let b = c.tree().unwrap();
    let mut diff = repo.diff_tree_to_tree(Some(&a), Some(&b), None).unwrap();
    let r = diff.format_email(1, 1, &c, None);
    println!("{:?}", r.map(|b| b.len()).map_err(|e| e.to_string()));
}

fn p_merge_empty() {
    let (_td, repo) = repo_init();
    let empty = repo.blob(b"").unwrap();
    let mk = |stage: u16| IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0, ino: 0, mode: 0o100644, uid: 0, gid: 0, file_size: 0,
        id: empty,
        flags: stage << 12,
        flags_extended: 0,
        path: b"f".to_vec(),
    };
    let r = repo.merge_file_from_index(&mk(1), &mk(2), &mk(3), None).unwrap();
    println!("automergeable={}", r.is_automergeable());
    println!("content len={}", r.content().len());
}

fn p_submodules(kind: &str) {
    let (td, repo) = repo_init();
    match kind {
        "nopath" => fs::write(td.path().join(".gitmodules"), "[submodule \"x\"]\n\turl = https://e.invalid/x\n").unwrap(),
        "nourl" => fs::write(td.path().join(".gitmodules"), "[submodule \"x\"]\n\tpath = y\n").unwrap(),
        "weird" => fs::write(td.path().join(".gitmodules"), "[submodule \"a/../b\"]\n\tpath = y\n\turl = u\n[submodule \"\"]\n\tpath = z\n\turl = u\n[submodule \"q\"]\n\tpath = ../../evil\n\turl = u\n").unwrap(),
        "cfgonly" => {
            let mut c = repo.config().unwrap();
            c.set_str("submodule.cfg.url", "https://e.invalid/x").unwrap();
        }
        "dup" => fs::write(td.path().join(".gitmodules"), "[submodule \"x\"]\n\tpath = y\n\turl = u\n[submodule \"z\"]\n\tpath = y\n\turl = v\n").unwrap(),
        _ => {}
    }
    let subs = repo.submodules();
    println!("{:?}", subs.map(|s| s.iter().map(|m| m.name().map(|n| n.to_string())).collect::<Vec<_>>()).map_err(|e| e.to_string()));
}

fn p_small() {
    let (_td, repo) = repo_init();
    let c0 = head(&repo).id();
    commit_file(&repo, "seed", b"2\n", "two");
    let c = head(&repo);
    println!("parent_id(1<<32)={:?}", c.parent_id(1usize << 32).map(|o| o.to_string()).map_err(|e| e.to_string()));
    println!("parent(1<<32)={:?}", c.parent(1usize << 32).map(|o| o.id().to_string()).map_err(|e| e.to_string()));
    let blame = repo.blame_file(Path::new("seed"), None).unwrap();
    println!("blame len={} get_index(1<<32).is_some()={}", blame.len(), blame.get_index(1usize << 32).is_some());

    // TreeEntry equality ignores id and mode content
    let t1 = repo.find_commit(c0).unwrap().tree().unwrap();
    let t2 = c.tree().unwrap();
    let (e1, e2) = (t1.get_name("seed").unwrap(), t2.get_name("seed").unwrap());
    println!("entries differ in id: {} vs {}, ==: {}", e1.id(), e2.id(), e1 == e2);

    // Reference equality ignores the name
    let r1 = repo.reference("refs/heads/a1", c.id(), false, "x").unwrap();
    let r2 = repo.reference("refs/heads/a2", c.id(), false, "x").unwrap();
    println!("refs {:?} {:?} ==: {}", r1.name(), r2.name(), r1 == r2);

    // Time Ord vs Eq
    let sig = b"tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\nauthor a <a@b> 100 -0000\ncommitter a <a@b> 100 -0000\n\nm\n";
    let odb = repo.odb().unwrap();
    let id = odb.write(ObjectType::Commit, sig).unwrap();
    let cm = repo.find_commit(id).unwrap();
    let w = cm.author().when();
    let t = Time::new(100, 0);
    println!("sign {:?} {:?}; eq={} cmp={:?}", w.sign(), t.sign(), w == t, w.cmp(&t));
    println!("commit.time().sign()={:?} author.when().sign()={:?}", cm.time().sign(), w.sign());

    // PackBuilder::hash
    let mut b = repo.packbuilder().unwrap();
    b.insert_commit(c.id()).unwrap();
    let mut buf = Buf::new();
    b.write_buf(&mut buf).unwrap();
    println!("after write: hash()={:?} name()={:?}", b.hash(), b.name());

    // refspec transform NUL panics, tree get_name NUL panics
    let p = panic::catch_unwind(|| ObjectType::from_str("a\0b"));
    println!("ObjectType::from_str NUL panicked={}", p.is_err());
    let p = panic::catch_unwind(panic::AssertUnwindSafe(|| t1.get_name("a\0b").is_some()));
    println!("Tree::get_name NUL panicked={}", p.is_err());
    repo.remote("o2", "https://e.invalid/x").unwrap();
    let rm = repo.find_remote("o2").unwrap();
    let rs = rm.get_refspec(0).unwrap();
    let p = panic::catch_unwind(panic::AssertUnwindSafe(|| rs.transform("a\0b").is_ok()));
    println!("Refspec::transform NUL panicked={}", p.is_err());
    let mut tx = repo.transaction().unwrap();
    let p = panic::catch_unwind(panic::AssertUnwindSafe(|| tx.lock_ref("a\0b").is_ok()));
    println!("Transaction::lock_ref NUL panicked={}", p.is_err());
}

fn p_error_last() {
    // Error::last_error when no libgit2 error is set.
    let e = Error::last_error(-1);
    println!("message={:?}", e.message());
}

fn p_packwrite_leak() {
    let (td, repo) = repo_init();
    let mut b = repo.packbuilder().unwrap();
    b.insert_commit(head(&repo).id()).unwrap();
    let out = TempDir::new().unwrap();
    b.write(out.path(), 0).unwrap();
    println!("written");
}

fn p_odb_packwriter_leak() {
    let odb = Odb::new().unwrap();
    println!("{:?}", odb.packwriter().err().map(|e| e.to_string()));
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let which = a.get(1).map(|s| s.as_str()).unwrap_or("");
    match which {
        "bitflags" => p_bitflags(),
        "buf_new" => p_buf_new(),
        "binary_nodata" => p_binary_nodata(),
        "remote_list_empty" => p_remote_list_empty(),
        "refspec_uaf" => p_refspec_uaf(),
        "index_add" => p_index_cb_null("add"),
        "index_update" => p_index_cb_null("update"),
        "index_remove" => p_index_cb_null("remove"),
        "index_get_path" => p_index_get_path(),
        "pathspec_matches" => p_pathspec_matches(),
        "prettify" => p_prettify(),
        "refnames" => p_refnames(),
        "remotehead" => p_remotehead(),
        "annotated" => p_annotated(),
        "rebase_exec" => p_rebase_exec(),
        "rebase_op_uaf" => p_rebase_op_uaf(),
        "rebase_opts_uaf" => p_rebase_opts_uaf(),
        "apply_move" => p_apply_move(),
        "stash_move" => p_stash_move(),
        "blame_sig" => p_blame_sig(),
        "skip_reuc" => p_skip_reuc(),
        "dry_run" => p_dry_run(),
        "force_after_dry" => p_force_after_dry(),
        "recreate" => p_recreate(),
        "notify_cancel" => p_notify_cancel(),
        "progress_panic" => p_progress_panic(),
        "foreach_panic" => p_foreach_panic(),
        "tag_foreach_err" => p_tag_foreach_err(),
        "pack_panic" => p_pack_panic(),
        "pathspec_diff_uaf" => p_pathspec_diff_uaf(),
        "reflog_uaf" => p_reflog_uaf(),
        "workdir_uaf" => p_workdir_uaf(),
        "namespace_uaf" => p_namespace_uaf(),
        "attr_uaf" => p_attr_uaf(),
        "indexer_odb_uaf" => p_indexer_odb_uaf(),
        "hide_cb_uaf" => p_hide_cb_uaf(),
        "diff_buffer_uaf" => p_diff_buffer_uaf(),
        "diff_mode" => p_diff_mode(),
        "cred_inject" => p_cred_inject(),
        "cred_ip" => p_cred_ip(),
        "remote_create_err" => p_remote_create_err(),
        "transport_err" => p_transport_err(),
        "opts_timeout" => p_opts_timeout(),
        "message_utf8" => p_message_utf8(),
        "note_ref_utf8" => p_note_ref_utf8(),
        "describe_utf8" => p_describe_utf8(),
        "lock_utf8" => p_lock_utf8(),
        "stash_msg_utf8" => p_stash_msg_utf8(),
        "format_email" => p_format_email(),
        "merge_empty" => p_merge_empty(),
        "sub_nopath" => p_submodules("nopath"),
        "sub_nourl" => p_submodules("nourl"),
        "sub_weird" => p_submodules("weird"),
        "sub_cfgonly" => p_submodules("cfgonly"),
        "sub_dup" => p_submodules("dup"),
        "small" => p_small(),
        "error_last" => p_error_last(),
        "packwrite_leak" => p_packwrite_leak(),
        "odb_packwriter_leak" => p_odb_packwriter_leak(),
        "clone_null" => p_clone_null(),
        "tree_walk_utf8" => p_tree_walk_utf8(),
        "tag_foreach_broken" => p_tag_foreach_broken(),
        "empty_blob" => p_empty_blob(),
        "binary_from_buffer" => p_binary_from_buffer(),
        "diff_send" => p_diff_send(),
        "index_alias" => p_index_alias(),
        "version" => p_version(),
        "parents" => p_parents(),
        "cred_store_inject" => p_cred_store_inject(),
        "url_username" => p_url_username(),
        "refspec_dst" => p_refspec_dst(),
        "push_neg_empty" => p_push_neg_empty(),
        "indexer_fail_leak" => p_indexer_fail_leak(),
        "cred_multi" => p_cred_multi(),
        "status_rename" => p_status_rename(),
        "transport_nul" => p_transport_nul(),
        "sub_names" => p_sub_names(),
        _ => eprintln!("unknown"),
    }
}

pub fn p_clone_null() {
    let (_td, src) = repo_init();
    let td = TempDir::new().unwrap();
    let url = format!("file://{}", src.workdir().unwrap().display());
    let mut co = CheckoutBuilder::new();
    co.notify_on(CheckoutNotificationType::UPDATED);
    co.notify(|_, _, _, _, _| false);
    let mut b = RepoBuilder::new();
    b.with_checkout(co);
    let r = b.clone(&url, &td.path().join("c"));
    println!("clone is_ok={}", r.is_ok());
    if let Ok(repo) = r {
        println!("calling repo.path()");
        println!("{:?}", repo.path());
    }
}

pub fn p_tree_walk_utf8() {
    let (_td, repo) = repo_init();
    let t = head(&repo).tree().unwrap();
    let blob = repo.blob(b"x").unwrap();
    let mut sub = repo.treebuilder(None).unwrap();
    sub.insert("inner", blob, 0o100644).unwrap();
    let sub_id = sub.write().unwrap();
    let mut top = repo.treebuilder(None).unwrap();
    top.insert(&b"caf\xe9"[..], sub_id, 0o040000).unwrap();
    top.insert("later", blob, 0o100644).unwrap();
    let top_id = top.write().unwrap();
    let tree = repo.find_tree(top_id).unwrap();
    let mut seen = vec![];
    let r = tree.walk(TreeWalkMode::PreOrder, |root, e| {
        seen.push(format!("{}{:?}", root, e.name()));
        TreeWalkResult::Ok
    });
    println!("seen={:?} ret={:?}", seen, r.map_err(|e| e.to_string()));
}

pub fn p_tag_foreach_broken() {
    let (_td, repo) = repo_init();
    let id = head(&repo).id();
    let obj = repo.find_object(id, None).unwrap();
    repo.tag_lightweight("a", &obj, false).unwrap();
    repo.tag_lightweight("zzz", &obj, false).unwrap();
    fs::write(repo.path().join("refs/tags/m"), "ref: refs/tags/missing\n").unwrap();
    let mut seen = vec![];
    let r = repo.tag_foreach(|_, n| { seen.push(String::from_utf8_lossy(n).into_owned()); true });
    println!("seen={:?} ret={:?}", seen, r);
    let names = repo.tag_names(None).unwrap();
    println!("tag_names={:?}", names.iter().collect::<Vec<_>>());
}

pub fn p_empty_blob() {
    let (_td, repo) = repo_init();
    let id = repo.blob(b"").unwrap();
    let b = repo.find_blob(id).unwrap();
    println!("blob len={}", b.content().len());
    let odb = repo.odb().unwrap();
    println!("odb len={}", odb.read(id).unwrap().data().len());
}

pub fn p_binary_from_buffer() {
    let text = b"diff --git a/x b/x\nindex 1234567..89abcde 100644\nBinary files a/x and b/x differ\n";
    let d = Diff::from_buffer(text).unwrap();
    d.foreach(&mut |_, _| true, Some(&mut |_, b| { println!("contains={}", b.contains_data()); println!("{}", b.old_file().data().len()); true }), None, None).unwrap();
}

pub fn p_diff_send() {
    fn assert_send<T: Send>(_: &T) {}
    let (_td, repo) = repo_init();
    let t = head(&repo).tree().unwrap();
    let diff = repo.diff_tree_to_tree(Some(&t), Some(&t), None).unwrap();
    std::thread::scope(|s| {
        s.spawn(move || { let _ = diff.stats(); });
        let _ = repo.head();
    });
    println!("compiled and ran: Diff moved to thread while &Repository still used");
}

pub fn p_index_alias() {
    let (_td, repo) = repo_init();
    let a = repo.index().unwrap();
    let mut b = repo.index().unwrap();
    let mut it = a.iter();
    b.clear().unwrap();
    println!("{:?}", it.next().map(|e| e.id));
}

pub fn p_version() {
    println!("crate_version={} libgit2={:?}", Version::get().crate_version(), Version::get().libgit2_version());
}

pub fn p_parents() {
    let (_td, repo) = repo_init();
    let real = head(&repo).id();
    git2::opts::strict_object_creation(false);
    let odb = repo.odb().unwrap();
    let txt = format!("tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\nparent 1111111111111111111111111111111111111111\nparent {}\nauthor a <a@b> 100 +0000\ncommitter a <a@b> 100 +0000\n\nm\n", real);
    let id = odb.write(ObjectType::Commit, txt.as_bytes()).unwrap();
    let c = repo.find_commit(id).unwrap();
    let mut p = c.parents();
    println!("len()={}", p.len());
    let a = p.next().map(|c| c.id());
    let b = p.next().map(|c| c.id());
    println!("next={:?} next={:?} (FusedIterator requires None forever after None)", a, b);
}

pub fn p_cred_store_inject() {
    let t = TempDir::new().unwrap();
    let store = t.path().join("store");
    fs::write(&store, "https://alice:s3cret@github.com\n").unwrap();
    let mut cfg = Config::new().unwrap();
    cfg.add_file(&t.path().join("cfg"), ConfigLevel::App, false).unwrap();
    cfg.set_str("credential.helper", &format!("store --file={}", store.display())).unwrap();
    let r = CredentialHelper::new("https://evil.example/repo").username(Some("alice")).config(&cfg).execute();
    println!("plain username for evil.example: {:?}", r);
    let r = CredentialHelper::new("https://evil.example/repo").username(Some("alice\nhost=github.com")).config(&cfg).execute();
    println!("injected username for evil.example: {:?}", r);
    let r = CredentialHelper::new("git@evil.example:x/y.git").config(&cfg).execute();
    println!("scp-style url: {:?}", r);
}

pub fn p_url_username() {
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
    let (_td, repo) = repo_init();
    let url = format!("http://alice%0Ahost=github.com@127.0.0.1:{}/r.git", port);
    let mut r = repo.remote_anonymous(&url).unwrap();
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|u, name, _| {
        println!("callback url={:?} username_from_url={:?}", u, name);
        Err(Error::from_str("stop"))
    });
    let res = r.connect_auth(Direction::Fetch, Some(cb), None).map(|_| ());
    println!("{:?}", res.map_err(|e| e.message().to_string()));
}

pub fn p_refspec_dst() {
    let (_td, repo) = repo_init();
    repo.remote_with_fetch("x", "https://e.invalid/x", "refs/heads/main").unwrap();
    let r = repo.find_remote("x").unwrap();
    let s = r.get_refspec(0).unwrap();
    println!("str={:?} src={:?}", s.str(), s.src());
    println!("dst={:?}", s.dst());
}

pub fn p_push_neg_empty() {
    let (_td, repo) = repo_init();
    let td2 = TempDir::new().unwrap();
    let _b = Repository::init_bare(td2.path()).unwrap();
    let url = format!("file://{}", td2.path().display());
    let mut r = repo.remote("dest", &url).unwrap();
    let mut cb = RemoteCallbacks::new();
    cb.push_negotiation(|u| { println!("negotiation with {} updates", u.len()); Ok(()) });
    let mut o = PushOptions::new();
    o.remote_callbacks(cb);
    let e: [&str; 0] = [];
    println!("{:?}", r.push(&e, Some(&mut o)).map_err(|e| e.to_string()));
}

pub fn p_indexer_fail_leak() {
    let t = TempDir::new().unwrap();
    println!("{:?}", Indexer::new(None, &t.path().join("no/such/dir"), 0, false).err().map(|e| e.to_string()));
}

pub fn p_cred_multi() {
    let t = TempDir::new().unwrap();
    let mk = |name: &str, body: &str| {
        let s = t.path().join(name);
        fs::write(&s, format!("#!/bin/sh\necho {} >> {}/log\n{}\n", name, t.path().display(), body)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&s, fs::Permissions::from_mode(0o755)).unwrap();
        s
    };
    let a = mk("first", "echo username=u; echo password=p");
    let b = mk("second", "exit 0");
    let mut cfg = Config::new().unwrap();
    cfg.add_file(&t.path().join("cfg"), ConfigLevel::App, false).unwrap();
    cfg.set_multivar("credential.helper", "^$", a.to_str().unwrap()).unwrap();
    cfg.set_multivar("credential.helper", "^$", b.to_str().unwrap()).unwrap();
    let mut n = 0;
    let mut es = cfg.multivar("credential.helper", None).unwrap();
    while let Some(e) = es.next() { let _ = e.unwrap(); n += 1; }
    println!("configured helpers={}", n);
    let r = CredentialHelper::new("https://example.com/r").config(&cfg).execute();
    println!("result={:?} invoked={:?}", r, fs::read_to_string(t.path().join("log")).ok());
}

pub fn p_status_rename() {
    let (td, repo) = repo_init();
    fs::rename(td.path().join("seed"), td.path().join("renamed")).unwrap();
    let mut idx = repo.index().unwrap();
    idx.remove_path(Path::new("seed")).unwrap();
    idx.add_path(Path::new("renamed")).unwrap();
    idx.write().unwrap();
    let mut o = StatusOptions::new();
    o.renames_head_to_index(true);
    let st = repo.statuses(Some(&mut o)).unwrap();
    for e in st.iter() {
        println!("path={:?} status={:?} new={:?}", e.path(), e.status(), e.head_to_index().and_then(|d| d.new_file().path().map(|p| p.to_path_buf())));
    }
}

struct NulStream;
impl std::io::Read for NulStream {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "bad\0text"))
    }
}
impl std::io::Write for NulStream {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { Ok(b.len()) }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
struct NulSub;
impl git2::transport::SmartSubtransport for NulSub {
    fn action(&self, _: &str, _: git2::transport::Service) -> Result<Box<dyn git2::transport::SmartSubtransportStream>, Error> {
        Ok(Box::new(NulStream))
    }
    fn close(&self) -> Result<(), Error> { Ok(()) }
}

pub fn p_transport_nul() {
    unsafe {
        git2::transport::register("nul", |r| git2::transport::Transport::smart(r, false, NulSub)).unwrap();
    }
    let (_td, repo) = repo_init();
    let mut r = repo.remote_anonymous("nul://x").unwrap();
    println!("connecting");
    println!("{:?}", r.connect(Direction::Fetch).map_err(|e| e.to_string()));
}

pub fn p_sub_names() {
    for name in ["a.b", "a b", "a(b", "a[b", "a*b", "a+", "a\\\\b", "A", "x.", "é"] {
        let (td, repo) = repo_init();
        fs::write(td.path().join(".gitmodules"), format!("[submodule \"{}\"]\n\tpath = y\n\turl = u\n", name)).unwrap();
        let r = repo.submodules().map(|s| s.len()).map_err(|e| e.to_string());
        println!("{:?} => {:?}", name, r);
    }
}
