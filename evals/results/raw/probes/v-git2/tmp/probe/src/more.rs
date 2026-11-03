use git2::*;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::{dir, sig, commit_file, repo, sh};

fn sha1(b: &[u8]) -> Vec<u8> {
    let d = dir(); let p = d.join("x"); fs::write(&p, b).unwrap();
    let o = Command::new("sha1sum").arg(&p).output().unwrap();
    let s = String::from_utf8(o.stdout).unwrap();
    let h = s.split_whitespace().next().unwrap();
    Oid::from_str(h).unwrap().as_bytes().to_vec()
}
fn index_overread() {
    let r = repo();
    let p = r.path().join("index");
    let b = fs::read(&p).unwrap();
    let len = 1000usize;
    let esize = (62 + len + 8) & !7;
    let mut e = b[12..12+62].to_vec();
    let fl = u16::from_be_bytes([e[60], e[61]]);
    let nf = (fl & 0xF000) | len as u16;
    e[60..62].copy_from_slice(&nf.to_be_bytes());
    e.extend_from_slice(b"s\0");
    e.resize(esize, 0);
    let mut out = b"DIRC\0\0\0\x02\0\0\0\x01".to_vec();
    out.extend(e);
    let h = sha1(&out); out.extend(h);
    fs::write(&p, &out).unwrap();
    let i = Index::open(&p).unwrap();
    println!("count {}", i.len());
    let en = i.get(0).unwrap();
    println!("path.len {} first {:?}", en.path.len(), String::from_utf8_lossy(&en.path[..40.min(en.path.len())]));
    let nz = en.path.iter().filter(|c| **c != 0).count();
    println!("nonzero bytes in path {}", nz);
}
fn serve401() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || for s in l.incoming() { let mut s = s.unwrap(); let mut b = [0u8; 4096]; let _ = s.read(&mut b);
        let _ = s.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); });
    port
}
fn helper_cfg(script_out: &Path) -> Config {
    let d = dir(); let p = d.join("cfg");
    fs::write(&p, format!("[credential]\n\thelper = \"!f() {{ cat > {}; }}; f\"\n", script_out.display())).unwrap();
    Config::open(&p).unwrap()
}
fn cred_inject() {
    // End to end: URL -> callback -> helper stdin, plus real git credential-store
    let port = serve401();
    let d = dir();
    let store = d.join("store"); fs::write(&store, "https://alice:s3cret@github.com\n").unwrap();
    let cfgp = d.join("cfg");
    fs::write(&cfgp, format!("[credential]\n\thelper = store --file={}\n", store.display())).unwrap();
    let r = repo();
    let url = format!("http://alice%0Ahost=github.com%0Aprotocol=https@127.0.0.1:{}/r.git", port);
    let mut rm = r.remote_anonymous(&url).unwrap();
    let mut cbs = RemoteCallbacks::new();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen2 = seen.clone();
    cbs.credentials(move |u, un, at| {
        println!("cb url={:?} username_from_url={:?} types={:?}", u, un, at);
        *seen2.borrow_mut() = un.map(|s| s.to_string());
        Err(Error::from_str("stop"))
    });
    let res = rm.connect_auth(Direction::Fetch, Some(cbs), None);
    println!("connect {:?}", res.err().map(|e| e.message().to_string()));
    let un: Option<String> = seen.borrow().clone();
    let cfg = Config::open(&cfgp).unwrap();
    let mut h = CredentialHelper::new("https://evil.example/repo");
    h.username(Some("alice\nhost=github.com"));
    h.config(&cfg);
    println!("inject execute -> {:?}", h.execute());
    let mut h = CredentialHelper::new("https://evil.example/repo");
    h.config(&cfg);
    println!("clean execute -> {:?}", h.execute());
    if let Some(u) = un {
        let mut h = CredentialHelper::new(&url);
        h.username(Some(&u));
        h.config(&cfg);
        println!("real-url execute -> {:?}", h.execute());
    }
    let o = dir().join("stdin");
    let c = helper_cfg(&o);
    let mut h = CredentialHelper::new("https://real.example/repo");
    h.username(Some("a\nhost=evil.example")); h.config(&c); h.execute();
    println!("stdin {:?}", fs::read_to_string(&o));
}
fn ip_host() {
    let d = dir();
    let store = d.join("store"); fs::write(&store, "https://alice:s3cret@github.com\n").unwrap();
    let cfgp = d.join("cfg");
    fs::write(&cfgp, format!("[credential]\n\thelper = store --file={}\n", store.display())).unwrap();
    let cfg = Config::open(&cfgp).unwrap();
    for u in ["https://203.0.113.9/repo", "https://[::1]:8443/repo", "git@evil.example:x/y.git", "https://github.com/r"] {
        let mut h = CredentialHelper::new(u); h.config(&cfg);
        println!("{} -> {:?}", u, h.execute());
    }
    let o = dir().join("stdin");
    let c = helper_cfg(&o);
    for u in ["https://10.0.0.1:8443/repo", "git@evil.example:x/y.git"] {
        let mut h = CredentialHelper::new(u); h.config(&c); h.execute();
        println!("{} stdin {:?}", u, fs::read_to_string(&o));
    }
    println!("git:{}", sh(&d, &format!("printf 'url=https://203.0.113.9/repo\\n\\n' | git -c credential.helper='store --file={}' credential fill 2>&1 </dev/stdin | head -5; true", store.display())));
}
fn multi_helper() {
    let d = dir();
    let cfgp = d.join("cfg");
    fs::write(&cfgp, "[credential]\n\thelper = \"!f() { echo username=u; echo password=p; }; f\"\n\thelper = \"!f() { true; }; f\"\n").unwrap();
    let cfg = Config::open(&cfgp).unwrap();
    let mut h = CredentialHelper::new("https://example.com/r"); h.config(&cfg);
    println!("execute {:?}", h.execute());
    println!("git:{}", sh(&d, &format!("printf 'url=https://example.com/r\\n\\n' | GIT_CONFIG_GLOBAL={} GIT_TERMINAL_PROMPT=0 git credential fill 2>&1", cfgp.display())));
}
fn port() {
    let d = dir();
    let cfgp = d.join("cfg");
    fs::write(&cfgp, "[credential \"https://h.example\"]\n\thelper = \"!f() { echo username=u; echo password=noport; }; f\"\n[credential \"https://k.example:8443\"]\n\thelper = \"!f() { echo username=u; echo password=withport; }; f\"\n").unwrap();
    let cfg = Config::open(&cfgp).unwrap();
    for u in ["https://h.example:8443/r", "https://k.example:8443/r", "https://k.example/r", "https://h.example/r"] {
        let mut h = CredentialHelper::new(u); h.config(&cfg);
        println!("{} crate {:?}", u, h.execute());
        println!("  git: {}", sh(&d, &format!("printf 'url={}\\n\\n' | GIT_CONFIG_GLOBAL={} GIT_TERMINAL_PROMPT=0 git credential fill 2>&1 | tr '\\n' ' '", u, cfgp.display())));
    }
}
fn blank_helper() {
    let d = dir(); let p = d.join("cfg");
    fs::write(&p, "[credential]\n\thelper = !\n").unwrap();
    let cfg = Config::open(&p).unwrap();
    let mut h = CredentialHelper::new("https://e.com/r"); h.config(&cfg);
    std::env::set_var("PATH", "/nonexistent");
    println!("{:?}", h.execute());
}
// Non-UTF8 refs. Writes packed-refs directly.
fn bad_repo() -> (Repository, Oid) {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    let mut pr = format!("# pack-refs with: peeled fully-peeled sorted \n").into_bytes();
    pr.extend_from_slice(format!("{} refs/heads/bad", h).as_bytes()); pr.push(0xff); pr.push(b'\n');
    fs::write(r.path().join("packed-refs"), pr).unwrap();
    (r, h)
}
fn remote_head_name() {
    let (src, _) = bad_repo();
    let r = repo();
    let mut rm = r.remote_anonymous(src.path().to_str().unwrap()).unwrap();
    rm.connect(Direction::Fetch).unwrap();
    for h in rm.list().unwrap() { println!("head {:?}", h.name_bytes_probe()); println!("{}", h.name()); }
}
trait P { fn name_bytes_probe(&self) -> usize; }
impl P for RemoteHead<'_> { fn name_bytes_probe(&self) -> usize { 0 } }
fn update_tips() {
    let (src, _) = bad_repo();
    let r = repo();
    let mut rm = r.remote_anonymous(src.path().to_str().unwrap()).unwrap();
    let mut cb = RemoteCallbacks::new();
    cb.update_tips(|n, _, _| { println!("tip {}", n); true });
    let mut fo = FetchOptions::new(); fo.remote_callbacks(cb);
    println!("{:?}", rm.fetch(&["+refs/*:refs/remotes/o/*"], Some(&mut fo), None));
}
fn ref_names() {
    let (r, _) = bad_repo();
    for n in r.references().unwrap().names() { println!("{:?}", n); }
}
fn describe() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    fs::create_dir_all(r.path().join("refs/tags")).unwrap();
    let mut pr = b"# pack-refs with: peeled fully-peeled sorted \n".to_vec();
    pr.extend_from_slice(format!("{} refs/tags/bad", h).as_bytes()); pr.push(0xff); pr.push(b'\n');
    fs::write(r.path().join("packed-refs"), pr).unwrap();
    let d = r.describe(DescribeOptions::new().describe_tags()).unwrap();
    println!("{:?}", d.format(None));
}
fn repo_message() {
    let r = repo();
    fs::write(r.path().join("MERGE_MSG"), b"caf\xe9\n").unwrap();
    println!("{:?}", r.message());
}
fn note_ref() {
    let r = repo();
    fs::write(r.path().join("config"), { let mut b = fs::read(r.path().join("config")).unwrap(); b.extend_from_slice(b"[core]\n\tnotesRef = refs/notes/caf\xe9\n"); b }).unwrap();
    let r = Repository::open(r.path()).unwrap();
    println!("{:?}", r.note_default_ref());
}
fn worktree_locked() {
    let r = repo();
    let d = dir().join("wt");
    let wt = r.worktree("wt", &d, None).unwrap();
    fs::write(r.path().join("worktrees/wt/locked"), b"caf\xe9").unwrap();
    println!("{:?}", wt.is_locked().map(|_| ()));
}
fn prettify() { println!("{:?}", message_prettify(&b"caf\xe9\n"[..], None)); }
const PATCH: &[u8] = b"diff --git a/f b/f\nold mode 100600\nnew mode 100777\n";
fn mode_patch() {
    let d = Diff::from_buffer(PATCH).unwrap();
    for x in d.deltas() { println!("{:?}", x.old_file().mode()); }
}
fn mode_tree() {
    let r = repo();
    let b = r.blob(b"x").unwrap();
    // raw tree via odb
    let mut t = b"100600 f\0".to_vec(); t.extend_from_slice(b.as_bytes());
    let tid = r.odb().unwrap().write(ObjectType::Tree, &t).unwrap();
    let tr = r.find_tree(tid).unwrap();
    let dd = r.diff_tree_to_tree(None, Some(&tr), None).unwrap();
    for x in dd.deltas() { println!("{:?}", x.new_file().mode()); }
}
fn stash_latin() {
    let mut r = repo();
    for i in 0..2 {
        fs::write(r.workdir().unwrap().join("seed"), format!("d{}\n", i)).unwrap();
        r.stash_save(&sig(), "m", None).unwrap();
    }
    let p = r.path().join("logs/refs/stash");
    let b = fs::read(&p).unwrap();
    let s: Vec<u8> = String::from_utf8_lossy(&b).replace("On master: m", "caf_").into_bytes().iter().map(|c| if *c == b'_' { 0xe9 } else { *c }).collect();
    fs::write(&p, s).unwrap();
    let mut r2 = Repository::open(r.path()).unwrap();
    let mut n = 0;
    println!("{:?}", r2.stash_foreach(|_, m, _| { n += 1; println!("{:?}", m); true }));
    println!("seen {}", n);
}
fn fetchhead_latin() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    let mut b = format!("{}\t\tbranch 'caf", h).into_bytes(); b.push(0xe9); b.extend_from_slice(b"' of url\n");
    fs::write(r.path().join("FETCH_HEAD"), b).unwrap();
    let mut n = 0;
    println!("{:?}", r.fetchhead_foreach(|a, _, _, _| { n += 1; println!("{:?}", a); true }));
    println!("seen {}", n);
}
fn fetchhead_forms() {
    let hr = repo(); let h = hr.head().unwrap().target().unwrap().to_string();
    for form in [format!("{}\n", h), format!("{}\t\t/some/url\n", h), format!("{}\tnot-for-merge\tremote-tracking branch 'o/m' of url\n", h), format!("{}\t\tbranch 'master' of url\n", h), format!("{}\t\t'{}' of /some/url\n", h, h)] {
        std::thread::spawn(move || {
            let r = repo();
            fs::write(r.path().join("FETCH_HEAD"), &form).unwrap();
            let mut n = 0;
            let res = r.fetchhead_foreach(|a, _, _, _| { n += 1; true }).map_err(|e| e.message().to_string());
            println!("{:?} -> {:?} seen {}", form, res, n);
        }).join().ok();
    }
    // what git really writes
    let src = repo();
    let r = repo();
    let o = sh(r.workdir().unwrap(), &format!("git fetch {} 2>&1; cat .git/FETCH_HEAD", src.path().display()));
    println!("git fetch <url> wrote: {:?}", o);
    let mut n = 0;
    let res = r.fetchhead_foreach(|a, _, _, _| { n += 1; println!("{:?}", a); true });
    println!("{:?} seen {}", res, n);
    // libgit2's own fetch by url
    let r = repo();
    let mut rm = r.remote_anonymous(src.path().to_str().unwrap()).unwrap();
    rm.fetch(&["refs/heads/master"], None, None).unwrap();
    println!("libgit2 wrote {:?}", fs::read_to_string(r.path().join("FETCH_HEAD")));
    let mut n = 0;
    let res = r.fetchhead_foreach(|a, _, _, _| { n += 1; println!("{:?}", a); true });
    println!("{:?} seen {}", res, n);
}
fn later_walk(r: &Repository) {
    let t = r.head().unwrap().peel_to_tree().unwrap();
    let mut n = 0;
    let res = t.walk(TreeWalkMode::PreOrder, |_, _| { n += 1; TreeWalkResult::Ok });
    println!("later walk n={} res={:?}", n, res.is_ok());
    let e = r.find_branch("no-such", BranchType::Local);
    println!("later failing call {:?}", e.is_err());
}
fn panic_odb() {
    let r = repo(); let odb = r.odb().unwrap();
    println!("{:?}", odb.foreach(|_| panic!("boom")));
    let mut n = 0; println!("second {:?}", odb.foreach(|_| { n += 1; true })); println!("visited {}", n);
    later_walk(&r);
}
fn panic_stash() {
    let mut r = repo();
    fs::write(r.workdir().unwrap().join("seed"), "d\n").unwrap();
    r.stash_save(&sig(), "m", None).unwrap();
    println!("{:?}", r.stash_foreach(|_, _, _| panic!("boom")));
    later_walk(&r);
}
fn panic_tag() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    r.tag_lightweight("t", &r.find_object(h, None).unwrap(), false).unwrap();
    println!("{:?}", r.tag_foreach(|_, _| panic!("boom")));
    later_walk(&r);
}
fn panic_mergehead() {
    let mut r = repo();
    let h = r.head().unwrap().target().unwrap();
    fs::write(r.path().join("MERGE_HEAD"), format!("{}\n", h)).unwrap();
    println!("{:?}", r.mergehead_foreach(|_| panic!("boom")));
    later_walk(&r);
}
fn panic_fetchhead() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    fs::write(r.path().join("FETCH_HEAD"), format!("{}\t\tbranch 'm' of url\n", h)).unwrap();
    println!("{:?}", r.fetchhead_foreach(|_, _, _, _| panic!("boom")));
    later_walk(&r);
}
fn panic_checkout(notify: bool) {
    let r = repo();
    fs::write(r.workdir().unwrap().join("seed"), "dirty\n").unwrap();
    let mut co = build::CheckoutBuilder::new();
    co.force();
    if notify { co.notify_on(CheckoutNotificationType::all()); co.notify(|_, _, _, _, _| panic!("boom")); }
    else { co.progress(|_, _, _| panic!("boom")); }
    println!("{:?}", r.checkout_head(Some(&mut co)));
    later_walk(&r);
}
fn panic_packwriter() {
    let r = repo();
    let mut pb = r.packbuilder().unwrap();
    pb.insert_commit(r.head().unwrap().target().unwrap()).unwrap();
    let mut b = Buf::new(); pb.write_buf(&mut b).unwrap();
    let r2 = repo();
    let odb = r2.odb().unwrap();
    let mut w = odb.packwriter().unwrap();
    w.progress(|_| panic!("boom"));
    println!("write {:?}", w.write_all(&b));
    println!("commit {:?}", w.commit());
    later_walk(&r2);
}
fn panic_indexer() {
    let r = repo();
    let mut pb = r.packbuilder().unwrap();
    pb.insert_commit(r.head().unwrap().target().unwrap()).unwrap();
    let mut b = Buf::new(); pb.write_buf(&mut b).unwrap();
    let d = dir();
    let mut ix = Indexer::new(None, &d, 0, false).unwrap();
    ix.progress(|_| panic!("boom"));
    println!("write {:?}", ix.write_all(&b));
    later_walk(&r);
}
fn tag_foreach_err() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    r.tag_lightweight("a", &r.find_object(h, None).unwrap(), false).unwrap();
    r.tag_lightweight("zzz", &r.find_object(h, None).unwrap(), false).unwrap();
    fs::write(r.path().join("refs/tags/bad"), "garbage\n").unwrap();
    let mut n = vec![];
    let res = r.tag_foreach(|_, nm| { n.push(String::from_utf8_lossy(nm).to_string()); true });
    println!("{:?} seen {:?}", res, n);
    println!("refname_to_id {:?}", r.refname_to_id("refs/tags/bad").map_err(|e| e.message().to_string()));
}
fn exec_panic() {
    let (r, t, c1) = crate::rebase_repo();
    let tc = r.find_annotated_commit(t).unwrap();
    let up = r.find_annotated_commit(c1).unwrap();
    let mut rb = r.rebase(Some(&tc), Some(&up), None, None).unwrap();
    let op = rb.next().unwrap().unwrap();
    println!("{:?}", op.exec());
}
fn annotated() {
    let r = repo();
    let c = r.find_annotated_commit(r.head().unwrap().target().unwrap()).unwrap();
    println!("{:?}", c.refname());
}
fn config_value() {
    let d = dir(); let p = d.join("c"); fs::write(&p, "[core]\n\tbare\n").unwrap();
    let c = Config::open(&p).unwrap();
    let mut it = c.entries(None).unwrap();
    it.for_each(|e| { println!("{:?} has={}", e.name(), e.has_value()); println!("{:?}", e.value().unwrap_or("")); }).unwrap();
}
fn refspec_dst() {
    let r = repo();
    r.remote_add_fetch("o", "refs/heads/main").ok();
    r.remote("q", "/nonexistent").unwrap();
    r.remote_add_fetch("q", "refs/heads/main").unwrap();
    let rm = r.find_remote("q").unwrap();
    for s in rm.refspecs() { println!("{:?}", s.str()); println!("{:?}", s.dst()); }
}
fn get_path() { let r = repo(); let i = r.index().unwrap(); println!("{:?}", i.get_path(Path::new("./seed"), 0).is_some()); }
fn get_path_abs() { let r = repo(); let i = r.index().unwrap(); println!("{:?}", i.get_path(Path::new("/etc/passwd"), 0).is_some()); }
fn matches_path() { let p = Pathspec::new(["*"].iter()).unwrap(); println!("{}", p.matches_path(Path::new("/abs"), PathspecFlags::DEFAULT)); }
fn valid_ref() { println!("{}", Reference::is_valid_name("a\0b")); }
fn valid_tag() { println!("{}", Tag::is_valid_name("a\0b")); }
fn valid_remote() { println!("{}", Remote::is_valid_name("a\0b")); }
fn nul_tree() { let r = repo(); let t = r.head().unwrap().peel_to_tree().unwrap(); println!("{:?}", t.get_name("a\0b").is_some()); }
fn nul_objtype() { println!("{:?}", ObjectType::from_str("a\0b")); }
fn nul_refspec() { let r = repo(); r.remote("o", "/x").unwrap(); let rm = r.find_remote("o").unwrap(); let s = rm.get_refspec(0).unwrap(); println!("{:?}", s.transform(b"a\0b".as_ref().iter().map(|c| *c as char).collect::<String>().as_str()).map(|_|())); }
fn nul_tx() { let r = repo(); let mut t = r.transaction().unwrap(); println!("{:?}", t.lock_ref("a\0b")); }
fn nul_status() { let mut o = StatusOptions::new(); o.pathspec("a\0b"); println!("no panic"); }
fn skip_reuc() {
    let r = repo();
    let base = r.head().unwrap().target().unwrap();
    let ours = commit_file(&r, "seed", "ours\n", "o");
    { let b = r.find_commit(base).unwrap(); r.branch("t", &b, true).unwrap(); }
    r.set_head("refs/heads/t").unwrap(); r.checkout_head(Some(build::CheckoutBuilder::new().force())).unwrap();
    let theirs = commit_file(&r, "seed", "theirs\n", "t");
    let (o, t) = (r.find_commit(ours).unwrap(), r.find_commit(theirs).unwrap());
    let i = r.merge_commits(&o, &t, None).unwrap();
    println!("default conflicts={}", i.has_conflicts());
    let mut mo = MergeOptions::new(); mo.skip_reuc(true);
    println!("skip_reuc {:?}", r.merge_commits(&o, &t, Some(&mo)).map(|i| i.has_conflicts()).map_err(|e| e.code()));
    let mut mo = MergeOptions::new(); mo.fail_on_conflict(true);
    println!("fail_on_conflict {:?}", r.merge_commits(&o, &t, Some(&mo)).map(|i| i.has_conflicts()).map_err(|e| e.code()));
}
fn flags() {
    println!("ANALYSIS_NONE {}", MergeAnalysis::ANALYSIS_NONE.is_none());
    println!("MergePreference {}", MergePreference::NONE.is_none());
    println!("Sort {}", Sort::NONE.is_none());
    println!("IndexAdd {}", IndexAddOption::DEFAULT.is_default());
    println!("Pathspec {}", PathspecFlags::DEFAULT.is_default());
    println!("Stash {}", StashFlags::DEFAULT.is_default());
    println!("StashApply {}", StashApplyFlags::DEFAULT.is_default());
    println!("DiffStats {}", DiffStatsFormat::NONE.is_none());
    println!("SHARED_UMASK {}", RepositoryInitMode::SHARED_UMASK.is_shared_umask());
    println!("rev is_no_single(SINGLE) {}", RevparseMode::SINGLE.is_no_single());
}
fn checkout_state() -> (Repository, Oid) {
    let r = repo();
    let t = r.head().unwrap().peel_to_tree().unwrap().id();
    fs::write(r.workdir().unwrap().join("seed"), "dirty\n").unwrap();
    (r, t)
}
fn dry_run() {
    // conflict: head tree has seed v1; target tree has seed v2; workdir dirty.
    let r = repo();
    let c1 = r.head().unwrap().target().unwrap();
    let c2 = commit_file(&r, "seed", "v2\n", "v2");
    fs::write(r.workdir().unwrap().join("seed"), "dirty\n").unwrap();
    let target = r.find_commit(c1).unwrap().tree().unwrap();
    let wd = r.workdir().unwrap().join("seed");
    println!("safe {:?}", r.checkout_tree(target.as_object(), Some(build::CheckoutBuilder::new().safe())).map_err(|e| e.message().to_string()));
    let mut n = 0;
    let mut co = build::CheckoutBuilder::new(); co.dry_run(); co.notify_on(CheckoutNotificationType::all()); co.notify(|_, _, _, _, _| { n += 1; true });
    println!("dry_run {:?}", r.checkout_tree(target.as_object(), Some(&mut co)).map_err(|e| e.message().to_string()));
    drop(co);
    println!("notify calls {}", n);
    println!("dry.force {:?} file={:?}", r.checkout_tree(target.as_object(), Some(build::CheckoutBuilder::new().dry_run().force())).is_ok(), fs::read_to_string(&wd));
    println!("force {:?} file={:?}", r.checkout_tree(target.as_object(), Some(build::CheckoutBuilder::new().force())).is_ok(), fs::read_to_string(&wd));
}
fn recreate() {
    let r = repo();
    let wd = r.workdir().unwrap().join("seed");
    fs::remove_file(&wd).unwrap();
    r.checkout_head(Some(build::CheckoutBuilder::new().recreate_missing(true).safe())).unwrap();
    println!("recreate.safe exists={}", wd.exists());
    fs::remove_file(&wd).ok();
    r.checkout_head(Some(build::CheckoutBuilder::new().safe().recreate_missing(true))).unwrap();
    println!("safe.recreate exists={}", wd.exists());
}
fn cancel() {
    let (r, _) = checkout_state();
    let wd = r.workdir().unwrap().join("seed");
    let mut co = build::CheckoutBuilder::new(); co.force(); co.notify_on(CheckoutNotificationType::all()); co.notify(|_, _, _, _, _| false);
    println!("{:?} file={:?}", r.checkout_head(Some(&mut co)), fs::read_to_string(&wd));
}
fn tree_walk() {
    let r = repo();
    let b = r.blob(b"x").unwrap();
    let mut sub = b"100644 f\0".to_vec(); sub.extend_from_slice(b.as_bytes());
    let odb = r.odb().unwrap();
    let st = odb.write(ObjectType::Tree, &sub).unwrap();
    let mut t = b"100644 a\0".to_vec(); t.extend_from_slice(b.as_bytes());
    t.extend_from_slice(b"40000 caf\xe9\0"); t.extend_from_slice(st.as_bytes());
    t.extend_from_slice(b"100644 z\0"); t.extend_from_slice(b.as_bytes());
    let tid = odb.write(ObjectType::Tree, &t).unwrap();
    let tr = r.find_tree(tid).unwrap();
    let mut v = vec![];
    let res = tr.walk(TreeWalkMode::PreOrder, |root, e| { v.push(format!("{}{:?}", root, e.name())); TreeWalkResult::Ok });
    println!("{:?} visited {:?}", res.map_err(|e| e.message().to_string()), v);
}
fn tree_eq() {
    let r = repo();
    let a = r.head().unwrap().peel_to_tree().unwrap();
    commit_file(&r, "seed", "changed\n", "c");
    let b = r.head().unwrap().peel_to_tree().unwrap();
    let (x, y) = (a.get_name("seed").unwrap(), b.get_name("seed").unwrap());
    println!("ids {} {} eq={}", x.id(), y.id(), x == y);
}
fn ref_eq() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    let a = r.reference("refs/heads/aaa", h, true, "x").unwrap();
    let b = r.reference("refs/heads/bbb", h, true, "x").unwrap();
    println!("eq={}", a == b);
    println!("cmp={:?}", a.cmp(&b));
}
fn cb_err() {
    let src = repo();
    let d = dir().join("c");
    let res = build::RepoBuilder::new().remote_create(|_, _, _| Err(Error::from_str("policy forbids this remote"))).clone(src.path().to_str().unwrap(), &d);
    println!("{:?}", res.err().map(|e| e.message().to_string()));
}
fn cb_err_transport() {
    unsafe { transport::register("myp", |_r| Err(Error::from_str("my transport factory failed"))).unwrap(); }
    let r = repo();
    let mut rm = r.remote_anonymous("myp://x/y").unwrap();
    println!("{:?}", rm.connect(Direction::Fetch).map_err(|e| e.message().to_string()));
}
fn timeouts() {
    unsafe {
        println!("{:?}", opts::set_server_connect_timeout_in_milliseconds(-5));
        println!("{:?}", opts::set_server_timeout_in_milliseconds(-5));
        println!("get {:?} {:?}", opts::get_server_connect_timeout_in_milliseconds(), opts::get_server_timeout_in_milliseconds());
    }
}
struct S; struct St;
impl std::io::Read for St { fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { Err(std::io::Error::new(std::io::ErrorKind::Other, "bad\0byte")) } }
impl std::io::Write for St { fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { Ok(b.len()) } fn flush(&mut self) -> std::io::Result<()> { Ok(()) } }
impl transport::SmartSubtransport for S {
    fn action(&self, _: &str, _: transport::Service) -> Result<Box<dyn transport::SmartSubtransportStream>, Error> { Ok(Box::new(St)) }
    fn close(&self) -> Result<(), Error> { Ok(()) }
}
fn nul_err() {
    unsafe { transport::register("myq", |r| transport::Transport::smart(r, true, S)).unwrap(); }
    let r = repo();
    let mut rm = r.remote_anonymous("myq://x/y").unwrap();
    println!("{:?}", rm.connect(Direction::Fetch).map_err(|e| e.message().to_string()));
}
fn nul_err_cert() {
    println!("n/a");
}
fn narrowing() {
    let r = repo();
    commit_file(&r, "seed", "2\n", "2");
    let c = r.head().unwrap().peel_to_commit().unwrap();
    println!("parent_id(1<<32) {:?}", c.parent_id(1usize << 32).map(|x| x == c.parent_id(0).unwrap()));
    println!("parent(1<<32) {:?}", c.parent(1usize << 32).map(|x| x.id() == c.parent_id(0).unwrap()).map_err(|_| ()));
    let b = r.blame_file(Path::new("seed"), None).unwrap();
    println!("get_index(1<<32) {}", b.get_index(1usize << 32).is_some());
}
fn parents() {
    let r = repo();
    let h = r.head().unwrap().target().unwrap();
    let t = r.head().unwrap().peel_to_tree().unwrap().id();
    let missing = "1111111111111111111111111111111111111111";
    let body = format!("tree {}\nparent {}\nparent {}\nauthor a <a@b> 1 +0000\ncommitter a <a@b> 1 +0000\n\nm\n", t, missing, h);
    let id = r.odb().unwrap().write(ObjectType::Commit, body.as_bytes()).unwrap();
    let c = r.find_commit(id).unwrap();
    let mut p = c.parents();
    println!("len {}", p.len());
    println!("{:?}", p.next().map(|x| x.id()));
    println!("{:?}", p.next().map(|x| x.id()));
    println!("{:?}", p.next().map(|x| x.id()));
    let mut q = c.parent_ids();
    println!("ids {:?} {:?}", q.next(), q.next());
}
fn pack_hash() {
    let r = repo();
    let mut pb = r.packbuilder().unwrap();
    pb.insert_commit(r.head().unwrap().target().unwrap()).unwrap();
    let d = dir();
    pb.write(&d, 0).unwrap();
    println!("count {} hash {:?} name {:?}", pb.object_count(), pb.hash(), pb.name());
}
struct Cnt;
fn pack_leak() {
    let r = repo();
    let d = dir();
    for _ in 0..50 { let mut pb = r.packbuilder().unwrap(); pb.insert_commit(r.head().unwrap().target().unwrap()).unwrap(); pb.write(&d, 0).unwrap(); }
    println!("done");
}
fn odb_leak() { let odb = Odb::new().unwrap(); for _ in 0..100 { println!("{:?}", odb.packwriter().err().map(|e| e.code())); } }
fn ix_leak() { for _ in 0..10 { println!("{:?}", Indexer::new(None, Path::new("/nonexistent/dir"), 0, false).err().map(|e| e.code())); } }
fn clone_null2() { let src = repo(); let d = dir().join("c"); let mut co = build::CheckoutBuilder::new(); co.notify_on(CheckoutNotificationType::all()); co.notify(|_, _, _, _, _| false);
 let r = build::RepoBuilder::new().with_checkout(co).clone(src.path().to_str().unwrap(), &d).unwrap();
 println!("state"); println!("{:?}", r.state()); println!("is_bare"); println!("{:?}", r.is_bare()); println!("head {:?}", r.head().map(|_|())); println!("index {:?}", r.index().map(|_|())); println!("workdir"); println!("{:?}", r.workdir()); println!("cfg {:?}", r.config().map(|_|())); }
fn sub_race() { let r = repo(); let f = r.workdir().unwrap().join(".gitmodules"); let mut body = String::new(); for i in 0..300 { body += &format!("[submodule \"s{}\"]\n\tpath = s{}\n\turl = /x\n", i, i); }
 std::thread::spawn({ let f=f.clone(); move || { loop { fs::write(&f, &body).unwrap(); std::thread::sleep(std::time::Duration::from_millis(3)); fs::write(&f, "").unwrap(); std::thread::sleep(std::time::Duration::from_millis(3)); } } });
 let mut ok=0; let mut er=0; let t=std::time::Instant::now(); while t.elapsed().as_secs()<40 { match r.submodules() { Ok(_)=>ok+=1, Err(_)=>er+=1 } } println!("ok {} err {}", ok, er); }
fn sub_shapes() { for (i,s) in ["[submodule \"a\"]\n\tpath = a\n[submodule \"a\"]\n\tpath = b\n","[submodule \"a\"]\n\turl = /x\n","[submodule \"a\"]\n\tignore = none\n","[submodule \"a\"]\n\tpath = ../x\n\turl = /x\n","[submodule \"a//b\"]\n\tpath = q\n\turl=/x\n","[submodule \"A\"]\n\tpath=a\n[submodule \"a\"]\n\tpath=A\n","[submodule \"\"]\n\tpath=a\n","[submodule \".\"]\n\tpath=a\n\turl=/x\n","[submodule \"a\\\\b\"]\n\tpath=q\n\turl=/x\n"].iter().enumerate() { let r = repo(); fs::write(r.workdir().unwrap().join(".gitmodules"), s).unwrap(); println!("shape {} -> {:?}", i, r.submodules().map(|v| v.len())); } }
fn time_ord() {
    let r = repo();
    let t = r.head().unwrap().peel_to_tree().unwrap().id();
    let body = format!("tree {}\nauthor a <a@b> 1000 -0000\ncommitter a <a@b> 1000 -0000\n\nm\n", t);
    let id = r.odb().unwrap().write(ObjectType::Commit, body.as_bytes()).unwrap();
    let c = r.find_commit(id).unwrap();
    let (a, b) = (c.time(), c.committer().when());
    println!("time.sign {} when.sign {} eq {} cmp {:?}", a.sign(), b.sign(), a == b, a.cmp(&b));
}
fn format_email() {
    let r = repo();
    let t = r.head().unwrap().peel_to_tree().unwrap();
    let id = r.commit(None, &sig(), &sig(), "first line\nsecond line\n\nbody\n", &t, &[]).unwrap();
    let c = r.find_commit(id).unwrap();
    let mut d = r.diff_tree_to_tree(None, Some(&t), None).unwrap();
    println!("{:?}", d.format_email(1, 1, &c, None).map(|_| ()));
}
fn pack_err() {
    let r = repo(); let odb = r.odb().unwrap();
    let mut w = odb.packwriter().unwrap();
    println!("{:?}", w.write_all(b"garbagegarbagegarbage___not a pack at all"));
    println!("last {:?}", Error::last_error(-1).message());
}
fn version() { println!("{} libgit2-sys is in crate", Version::get().crate_version()); }
fn err_fallback() { let e = Error::last_error(-1); println!("{:?}", e.message()); }
fn index_shrink() {
    let r = repo();
    commit_file(&r, "b", "b", "b");
    let a = r.index().unwrap();
    let mut b = r.index().unwrap();
    let mut it = a.iter();
    b.clear().unwrap();
    println!("{:?}", it.next().map(|e| e.path.len()));
}
fn submodule_valueless() {
    let r = repo();
    fs::write(r.workdir().unwrap().join(".gitmodules"), "[submodule \"a\"]\n\tpath\n").unwrap();
    println!("{:?}", r.submodules().map(|v| v.len()));
}
fn submodule_assert() {
    let r = repo();
    fs::write(r.workdir().unwrap().join(".gitmodules"), "[submodule \"a\"]\n\tpath = a\n\turl = /x\n[submodule \"../evil\"]\n\tpath = b\n\turl = /x\n").unwrap();
    println!("{:?}", r.submodules().map(|v| v.len()));
}
fn stash_msg() {
    let mut r = repo();
    fs::write(r.workdir().unwrap().join("seed"), "d\n").unwrap();
    let mut o = StashSaveOptions::new(sig());
    r.stash_save_ext(Some(&mut o)).unwrap();
    r.stash_foreach(|_, m, _| { println!("{:?}", m); true }).unwrap();
}
fn send_diff() {
    let r = repo();
    let t = r.head().unwrap().peel_to_tree().unwrap();
    let d = r.diff_tree_to_tree(None, Some(&t), None).unwrap();
    std::thread::scope(|s| { s.spawn(move || { println!("{}", d.deltas().len()); drop(d); }); println!("{:?}", r.path()); });
}
pub fn dispatch(n: &str) {
    match n {
        "index_overread" => index_overread(), "cred_inject" => cred_inject(), "ip_host" => ip_host(), "multi_helper" => multi_helper(), "port" => port(), "blank_helper" => blank_helper(),
        "remote_head_name" => remote_head_name(), "update_tips" => update_tips(), "ref_names" => ref_names(), "describe" => describe(), "repo_message" => repo_message(), "note_ref" => note_ref(), "worktree_locked" => worktree_locked(), "prettify" => prettify(),
        "mode_patch" => mode_patch(), "mode_tree" => mode_tree(), "stash_latin" => stash_latin(), "fetchhead_latin" => fetchhead_latin(), "fetchhead_forms" => fetchhead_forms(),
        "panic_odb" => panic_odb(), "panic_stash" => panic_stash(), "panic_tag" => panic_tag(), "panic_mergehead" => panic_mergehead(), "panic_fetchhead" => panic_fetchhead(), "panic_co_prog" => panic_checkout(false), "panic_co_notify" => panic_checkout(true), "panic_packwriter" => panic_packwriter(), "panic_indexer" => panic_indexer(),
        "tag_foreach_err" => tag_foreach_err(), "exec_panic" => exec_panic(), "annotated" => annotated(), "config_value" => config_value(), "refspec_dst" => refspec_dst(), "get_path" => get_path(), "get_path_abs" => get_path_abs(), "matches_path" => matches_path(),
        "valid_ref" => valid_ref(), "valid_tag" => valid_tag(), "valid_remote" => valid_remote(), "nul_tree" => nul_tree(), "nul_objtype" => nul_objtype(), "nul_refspec" => nul_refspec(), "nul_tx" => nul_tx(), "nul_status" => nul_status(),
        "skip_reuc" => skip_reuc(), "flags" => flags(), "dry_run" => dry_run(), "recreate" => recreate(), "cancel" => cancel(), "tree_walk" => tree_walk(), "tree_eq" => tree_eq(), "ref_eq" => ref_eq(), "cb_err" => cb_err(), "cb_err_transport" => cb_err_transport(), "timeouts" => timeouts(), "nul_err" => nul_err(),
        "narrowing" => narrowing(), "parents" => parents(), "pack_hash" => pack_hash(), "pack_leak" => pack_leak(), "time_ord" => time_ord(), "format_email" => format_email(), "pack_err" => pack_err(), "version" => version(), "err_fallback" => err_fallback(), "index_shrink" => index_shrink(), "submodule_valueless" => submodule_valueless(), "submodule_assert" => submodule_assert(), "stash_msg" => stash_msg(), "send_diff" => send_diff(), "odb_leak" => odb_leak(), "ix_leak" => ix_leak(), "clone_null2" => clone_null2(), "sub_race" => sub_race(), "sub_shapes" => sub_shapes(),
        _ => extra(n),
    }
}
pub fn extra(n: &str) {
    match n {
        "tag_symref" => {
            let r = repo();
            let h = r.head().unwrap().target().unwrap();
            r.tag_lightweight("a", &r.find_object(h, None).unwrap(), false).unwrap();
            r.tag_lightweight("zzz", &r.find_object(h, None).unwrap(), false).unwrap();
            fs::write(r.path().join("refs/tags/m"), "ref: refs/heads/nope\n").unwrap();
            let mut v = vec![];
            let res = r.tag_foreach(|_, nm| { v.push(String::from_utf8_lossy(nm).to_string()); true });
            println!("{:?} seen {:?}", res.map_err(|e| e.message().to_string()), v);
        }
        "diff_race" => {
            let r = repo();
            for i in 0..50 { commit_file(&r, &format!("f{}", i), &"x\n".repeat(50), "c"); }
            let t = r.head().unwrap().peel_to_tree().unwrap();
            let t0 = r.find_commit(r.head().unwrap().peel_to_commit().unwrap().parent_id(0).unwrap()).unwrap();
            let d = r.diff_tree_to_tree(None, Some(&t), None).unwrap();
            let stop = std::sync::atomic::AtomicBool::new(false);
            let st = &stop;
            std::thread::scope(|s| {
                s.spawn(move || {
                    for _ in 0..3000 { let mut n = 0; let _ = d.print(DiffFormat::Patch, |_, _, _| { n += 1; true }); }
                    st.store(true, std::sync::atomic::Ordering::SeqCst);
                });
                let path = r.path().join("objects");
                while !st.load(std::sync::atomic::Ordering::SeqCst) {
                    let o = Odb::new().unwrap();
                    o.add_disk_alternate(path.to_str().unwrap()).unwrap();
                    r.set_odb(&o).unwrap();
                }
            });
            println!("survived");
        }
        c if c.starts_with("clonenull_") => {
            let src = repo(); let d = dir().join("c");
            let mut co = build::CheckoutBuilder::new(); co.notify_on(CheckoutNotificationType::all()); co.notify(|_, _, _, _, _| false);
            let r = build::RepoBuilder::new().with_checkout(co).clone(src.path().to_str().unwrap(), &d).unwrap();
            println!("built");
            match &c[10..] {
                "head" => println!("{:?}", r.head().map(|_| ())),
                "index" => println!("{:?}", r.index().map(|_| ())),
                "config" => println!("{:?}", r.config().map(|_| ())),
                "odb" => println!("{:?}", r.odb().map(|_| ())),
                "status" => println!("{:?}", r.statuses(None).map(|_| ())),
                "revwalk" => println!("{:?}", r.revwalk().map(|_| ())),
                "refs" => println!("{:?}", r.references().map(|_| ())),
                "find" => println!("{:?}", r.find_commit(Oid::from_bytes(&[1;20]).unwrap()).map(|_| ())),
                "is_bare" => println!("{:?}", r.is_bare()),
                "workdir" => println!("{:?}", r.workdir()),
                "remotes" => println!("{:?}", r.remotes().map(|_| ())),
                "branches" => println!("{:?}", r.branches(None).map(|_| ())),
                _ => {}
            }
        }
        _ => println!("unknown {}", n),
    }
}
