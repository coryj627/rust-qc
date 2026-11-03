// Seeded mutation harness over the crate's parsers of untrusted bytes.
use git2::*;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::Mutex;

static mut LAST: [u8; 8192] = [0; 8192];
static mut LAST_LEN: usize = 0;
static mut TARGET: usize = 0;
static LOCS: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);
thread_local!(static CUR: std::cell::RefCell<String> = std::cell::RefCell::new(String::new()));

extern "C" fn on_sig(sig: i32) {
    unsafe {
        let msg = format!("\nSIGNAL {} target {} input: ", sig, TARGET);
        libc::write(2, msg.as_ptr() as *const _, msg.len());
        let p = std::ptr::addr_of!(LAST) as *const u8;
        let s: Vec<u8> = std::slice::from_raw_parts(p, LAST_LEN).iter().flat_map(|b| std::ascii::escape_default(*b)).collect();
        libc::write(2, s.as_ptr() as *const _, s.len());
        libc::write(2, b"\n".as_ptr() as *const _, 1);
        libc::_exit(99);
    }
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn below(&mut self, n: usize) -> usize { (self.next() % n.max(1) as u64) as usize }
}
const TOKENS: &[&[u8]] = &[b"\n", b" ", b"+", b"-", b"@@", b"\\", b"\xff", b"\xc3", b"\0", b"100644", b"100600", b"160000", b"040000", b"120000", b"GIT binary patch\n", b"literal 0\n", b"delta 3\n", b"rename from ", b"copy to ", b"similarity index 200%\n", b"old mode ", b"new file mode ", b"deleted file mode ", b"Binary files a b differ\n", b"--- ", b"+++ ", b"/dev/null", b"a/", b"b/", b"-1,99999999999 ", b"\\ No newline at end of file\n", b"<", b">", b"=", b":", b"HEAD", b"@{", b"~", b"^", b"99999999999", b"[", b"]", b"\"", b"%", b"Signed-off-by: ", b"---\n", b"gpgsig ", b"encoding ", b"parent ", b"author ", b"committer "];
fn mutate(r: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let mut v = seed.to_vec();
    for _ in 0..(1 + r.below(6)) {
        let at = r.below(v.len() + 1);
        match r.below(6) {
            0 => { if !v.is_empty() { let i = r.below(v.len()); v[i] = r.next() as u8; } }
            1 => { if at < v.len() { let n = r.below(v.len() - at + 1).min(40); v.drain(at..at + n); } }
            2 | 3 => { let t = TOKENS[r.below(TOKENS.len())]; let tail = v.split_off(at); v.extend_from_slice(t); v.extend(tail); }
            4 => { if at < v.len() { let n = r.below(v.len() - at).min(60); let c = v[at..at + n].to_vec(); let j = r.below(v.len() + 1); let tail = v.split_off(j); v.extend(c); v.extend(tail); } }
            _ => { v.truncate(at); }
        }
    }
    v
}
fn touch_diff(d: &Diff<'_>) {
    for x in d.deltas() {
        let _ = (x.status(), x.flags(), x.nfiles());
        for f in [x.old_file(), x.new_file()] { let _ = (f.path_bytes(), f.path(), f.id(), f.size(), f.mode(), f.is_binary(), f.exists()); }
    }
    let _ = d.stats().map(|s| s.to_buf(DiffStatsFormat::FULL, 80).map(|b| b.len()));
    let _ = d.print(DiffFormat::Patch, |_, h, l| { let _ = h.map(|h| h.header().len()); let _ = (l.content().len(), l.origin(), l.origin_value(), l.old_lineno()); true });
    let _ = d.print(DiffFormat::Raw, |_, _, l| l.content().len() < usize::MAX);
    let _ = d.foreach(&mut |_, _| true, Some(&mut |_, b| { let _ = (b.old_file().kind(), b.new_file().kind(), b.contains_data()); let _ = (b.old_file().inflated_len(), b.new_file().inflated_len()); true }), Some(&mut |_, h| h.header().len() < usize::MAX), Some(&mut |_, _, l| l.content().len() < usize::MAX));
    for i in 0..d.deltas().len() {
        if let Ok(Some(mut p)) = Patch::from_diff(d, i) {
            let _ = p.line_stats();
            for h in 0..p.num_hunks() {
                let _ = p.hunk(h).map(|(hh, _)| hh.header().len());
                let n = p.num_lines_in_hunk(h).unwrap_or(0);
                for l in 0..n { let _ = p.line_in_hunk(h, l).map(|x| x.content().len()); }
            }
            let _ = p.to_buf().map(|b| b.len());
        }
    }
    let _ = d.patchid(None);
}
fn main() {
    unsafe {
        libc::signal(libc::SIGSEGV, on_sig as *const () as usize);
        libc::signal(libc::SIGBUS, on_sig as *const () as usize);
        libc::signal(libc::SIGABRT, on_sig as *const () as usize);
    }
    std::panic::set_hook(Box::new(|i| {
        let loc = i.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = format!("{}", i).lines().nth(1).unwrap_or("").chars().take(120).collect::<String>();
        let cur = CUR.with(|c| c.borrow().clone());
        if msg.contains("unsafe precondition") || loc.contains("panicking.rs") { eprintln!("NONUNWIND {} {}", loc, msg); }
        let mut g = LOCS.lock().unwrap();
        g.get_or_insert_with(BTreeMap::new).entry(loc.clone()).or_insert(cur);
    }));
    let iters: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let td = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(td.path()).unwrap();
    let odb = repo.odb().unwrap();
    let blob = repo.blob(b"x\n").unwrap();
    let mut tb = repo.treebuilder(None).unwrap();
    tb.insert("f", blob, 0o100644).unwrap();
    let tree = tb.write().unwrap();
    let commit_seed = format!("tree {t}\nparent {b}\nauthor A U Thor <a@x> 1000 +0100\ncommitter C <c@x> 1000 -0000\nencoding latin1\ngpgsig x\n y\n\nSummary line\nmore\n\nBody\n", t = tree, b = blob).into_bytes();
    let tag_seed = format!("object {b}\ntype blob\ntag v1\ntagger T <t@x> 1000 +0000\n\nmsg\n", b = blob).into_bytes();
    let mut tree_seed = b"100644 f\0".to_vec(); tree_seed.extend_from_slice(blob.as_bytes()); tree_seed.extend_from_slice(b"40000 d\0"); tree_seed.extend_from_slice(tree.as_bytes());
    let patches: &[&[u8]] = &[
        b"diff --git a/f b/f\nindex 1111111..2222222 100644\n--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@ ctx\n-a\n+b\n c\n\\ No newline at end of file\n",
        b"diff --git a/o b/n\nsimilarity index 90%\nrename from o\nrename to n\nold mode 100644\nnew mode 100755\n",
        b"diff --git a/f b/f\nnew file mode 100644\nindex 0000000..e69de29\n",
        b"diff --git a/b b/b\nindex 1111111..2222222 100644\nGIT binary patch\nliteral 5\nMc${NkU|?imU|;|M00aO5\n\nliteral 0\nHc$@<O00001\n\n",
        b"diff --git a/b b/b\nindex 1..2 100644\nBinary files a/b and b/b differ\n",
    ];
    let trailers: &[u8] = b"Subject\n\nBody text\n\nSigned-off-by: A <a@x>\nCo-authored: B\n continued\n";
    let mailmap: &[u8] = b"Real <r@x> <o@x>\nReal2 <r2@x> Old <o2@x>\n# comment\n";
    let cfg: &[u8] = b"[core]\n\tbare = false\n\tn = 12k\n[a \"b.c\"]\n\tv = \"q\\n\"\n\tmulti = 1\n\tmulti = 2\n\tnoval\n[include]\n\tpath = none\n";
    let specs: &[&[u8]] = &[b"HEAD~2^{tree}:f", b"HEAD@{1}..v1", b":/foo", b"master@{upstream}", b"refs/heads/*:refs/remotes/o/*", b"1234abc"];
    let cfgp = td.path().join("fuzzcfg");
    let sig = Signature::new("n", "e@x", &Time::new(1000, 0)).unwrap();
    let mm = Mailmap::new().unwrap();
    let mut r = Rng(0x9E3779B97F4A7C15);
    for it in 0..iters {
        let target = it % 9;
        let input: Vec<u8> = match target {
            0 | 1 => mutate(&mut r, patches[it / 9 % patches.len()]),
            2 => mutate(&mut r, trailers),
            3 => mutate(&mut r, mailmap),
            4 => mutate(&mut r, cfg),
            5 => mutate(&mut r, &commit_seed),
            6 => mutate(&mut r, &tag_seed),
            7 => mutate(&mut r, &tree_seed),
            _ => mutate(&mut r, specs[it / 9 % specs.len()]),
        };
        unsafe {
            TARGET = target;
            LAST_LEN = input.len().min(8192);
            let p = std::ptr::addr_of_mut!(LAST) as *mut u8;
            std::ptr::copy_nonoverlapping(input.as_ptr(), p, LAST_LEN);
        }
        CUR.with(|c| *c.borrow_mut() = format!("t{} {:?}", target, String::from_utf8_lossy(&input)));
        let _ = catch_unwind(AssertUnwindSafe(|| match target {
            0 | 1 => { if let Ok(d) = Diff::from_buffer(&input) { touch_diff(&d); } }
            2 => {
                let _ = message_trailers_bytes(&input[..]).map(|t| t.iter().count());
                if let Ok(s) = std::str::from_utf8(&input) {
                    let _ = message_trailers_strs(s).map(|t| t.iter().count());
                    let _ = message_prettify(s, Some(b'#'));
                }
            }
            3 => { if let Ok(s) = std::str::from_utf8(&input) { if let Ok(m) = Mailmap::from_buffer(s) { let _ = m.resolve_signature(&sig); } } let _ = mm.resolve_signature(&sig); }
            4 => {
                if std::fs::write(&cfgp, &input).is_ok() {
                    if let Ok(c) = Config::open(&cfgp) {
                        if let Ok(mut e) = c.entries(None) { while let Some(Ok(x)) = e.next() { let _ = (x.name_bytes(), x.level(), x.has_value(), x.include_depth()); if x.has_value() { let _ = x.value_bytes(); } } }
                        let _ = (c.get_string("core.bare"), c.get_bool("core.bare"), c.get_i64("core.n"), c.get_path("a.b.c.v"));
                    }
                }
                if let Ok(s) = std::str::from_utf8(&input) { let _ = (Config::parse_bool(s), Config::parse_i32(s), Config::parse_i64(s)); }
            }
            5 => {
                if let Ok(id) = odb.write(ObjectType::Commit, &input) {
                    if let Ok(c) = repo.find_commit(id) {
                        let _ = (c.message_bytes(), c.message_raw_bytes(), c.raw_header_bytes(), c.summary_bytes(), c.body_bytes(), c.message_encoding(), c.time(), c.tree_id(), c.parent_count());
                        let _ = (c.author().name_bytes().len(), c.author().email_bytes().len(), c.author().when(), c.committer().name_bytes().len(), c.committer().email_bytes().len());
                        let _ = (c.header_field_bytes("gpgsig").map(|b| b.len()), c.parent_ids().count(), c.parents().count());
                        let _ = repo.extract_signature(&id, None);
                        let _ = c.author_with_mailmap(&mm).map(|s| s.name_bytes().len());
                    }
                }
            }
            6 => {
                if let Ok(id) = odb.write(ObjectType::Tag, &input) {
                    if let Ok(t) = repo.find_tag(id) {
                        let _ = (t.name_bytes(), t.message_bytes(), t.target_id(), t.target_type());
                        if let Some(s) = t.tagger() { let _ = (s.name_bytes().len(), s.email_bytes().len()); }
                        let _ = t.target(); let _ = t.peel();
                    }
                }
            }
            7 => {
                if let Ok(id) = odb.write(ObjectType::Tree, &input) {
                    if let Ok(t) = repo.find_tree(id) {
                        for e in t.iter() { let _ = (e.name_bytes(), e.id(), e.kind(), e.filemode(), e.filemode_raw()); let _ = e.to_owned(); }
                        let _ = t.walk(TreeWalkMode::PreOrder, |_, _| 0);
                        let _ = t.get_path(Path::new("f"));
                        let _ = repo.diff_tree_to_tree(Some(&t), None, None).map(|d| touch_diff(&d));
                    }
                }
            }
            _ => {
                if let Ok(s) = std::str::from_utf8(&input) {
                    let _ = repo.revparse(s).map(|r| r.mode());
                    let _ = repo.revparse_ext(s).map(|r| r.0.kind());
                    let _ = Oid::from_str(s);
                    let _ = Reference::normalize_name(s, ReferenceFormat::ALLOW_ONELEVEL | ReferenceFormat::REFSPEC_PATTERN);
                    let _ = CredentialHelper::new(s).username(None).execute();
                    if !s.contains('\0') { let _ = (Reference::is_valid_name(s), Remote::is_valid_name(s), Tag::is_valid_name(s), ObjectType::from_str(s)); }
                    let _ = Signature::new(s, s, &Time::new(0, 0)).map(|x| x.name_bytes().len());
                    let _ = Pathspec::new([s].iter()).map(|p| p.match_tree(&repo.find_tree(tree).unwrap(), PathspecFlags::DEFAULT).map(|l| l.entries().count()));
                    if let Ok(mut rm) = repo.remote_anonymous("file:///nonexistent") { let _ = rm.fetch_refspecs(); let _ = rm.stop(); }
                }
            }
        }));
        let _ = git2::Error::last_error(0); // keep libgit2 error state from growing stale
    }
    let g = LOCS.lock().unwrap();
    println!("done {} iterations; distinct panics:", iters);
    if let Some(m) = &*g { for (k, v) in m { println!("{}\n   e.g. {}", k, v.chars().take(400).collect::<String>()); } }
}
