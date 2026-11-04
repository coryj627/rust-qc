use git2::*;
use std::{path::Path, io::{Read, Write}};
fn repo() -> Repository {
    let p = format!("/work/tmp/probe-{}-{}", std::process::id(), std::env::args().nth(1).unwrap());
    Repository::init(p).unwrap()
}
fn commit(repo: &Repository, parent: Option<Oid>, content: &[u8]) -> Oid {
    let blob = repo.blob(content).unwrap();
    let mut tb = repo.treebuilder(None).unwrap();
    tb.insert("f", blob, 0o100644).unwrap();
    let tid = tb.write().unwrap();
    let tree = repo.find_tree(tid).unwrap();
    let sig = Signature::now("Audit", "<email>").unwrap();
    let parent = parent.map(|id| repo.find_commit(id).unwrap());
    let parents: Vec<_> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "test", &tree, &parents).unwrap()
}
fn main() {
    let mode = std::env::args().nth(1).unwrap();
    match mode.as_str() {
        "buf" => { println!("empty length: {}", Buf::new().len()); }
        "buf-mut" => { let mut b = Buf::new(); println!("empty mutable slice: {:?}", &mut b[..]); }
        "rebase" | "rebase-exec" => {
            let r = repo();
            let a = commit(&r, None, b"a\n");
            let b = commit(&r, Some(a), b"b\n");
            let branch = r.find_annotated_commit(b).unwrap();
            let upstream = r.find_annotated_commit(a).unwrap();
            let mut opts = RebaseOptions::new(); opts.inmemory(true);
            let op = {
                let mut rb = r.rebase(Some(&branch), Some(&upstream), None, Some(&mut opts)).unwrap();
                let op = rb.next().unwrap().unwrap();
                println!("before drop: {}", op.id());
                if mode == "rebase-exec" { println!("exec: {:?}", op.exec()); }
                op
            };
            println!("after drop: {} (expected {})", op.id(), b);
        }
        "revwalk" => {
            let r = repo();
            let head = commit(&r, None, b"a\n");
            let mut walk = {
                let captured = Box::new([b'X'; 1024]);
                let mut cb = move |id| { println!("callback after into_inner: {} {}", captured[0], id); false };
                r.revwalk().unwrap().with_hide_callback(&mut cb).unwrap().into_inner().unwrap()
            };
            walk.push(head).unwrap();
            println!("walk result: {:?}", walk.next());
        }
        "pathspec" => {
            let ps = Pathspec::new(["f"]).unwrap();
            let matches = {
                let diff = Diff::from_buffer(b"diff --git a/f b/f\nindex 7898192..6178079 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n").unwrap();
                let matches = ps.match_diff(&diff, PathspecFlags::DEFAULT).unwrap();
                println!("before drop: {:?}", matches.diff_entry(0).unwrap().new_file().path());
                matches
            };
            println!("after drop: {:?}", matches.diff_entry(0).unwrap().new_file().path());
        }
        "reader" => {
            let r = repo();
            let id = r.blob(b"abc").unwrap();
            let odb = r.odb().unwrap();
            let (mut reader, size, _) = odb.reader(id).unwrap();
            let mut buf = [0; 3];
            println!("size={} read returned {:?}, buffer={:?}", size, reader.read(&mut buf), buf);
            let (mut reader, _, _) = odb.reader(id).unwrap();
            let mut buf = Vec::new();
            println!("read_to_end returned {:?}, buffer={:?}", reader.read_to_end(&mut buf), buf);
        }
        "skip-reuc" => {
            let r = repo();
            let base = commit(&r, None, b"base\n");
            let ours = commit(&r, Some(base), b"ours\n");
            r.set_head_detached(base).unwrap();
            let theirs = commit(&r, Some(base), b"theirs\n");
            let b = r.find_commit(base).unwrap().tree().unwrap();
            let o = r.find_commit(ours).unwrap().tree().unwrap();
            let t = r.find_commit(theirs).unwrap().tree().unwrap();
            println!("default merge has conflicts: {}", r.merge_trees(&b, &o, &t, None).unwrap().has_conflicts());
            let mut opts = MergeOptions::new(); opts.skip_reuc(true);
            println!("skip_reuc merge result: {:?}", r.merge_trees(&b, &o, &t, Some(&opts)).map(|i| i.has_conflicts()));
        }
        "indexer" => {
            let r = repo();
            let head = commit(&r, None, b"abc\n");
            let mut pb = r.packbuilder().unwrap(); pb.insert_commit(head).unwrap();
            let mut buf = Buf::new(); pb.write_buf(&mut buf).unwrap();
            let path = r.path().join("objects/pack");
            let mut ix = {
                let odb = Odb::new().unwrap();
                Indexer::new(Some(&odb), &path, 0, true).unwrap()
            };
            println!("writing with dropped odb");
            ix.write_all(&buf).unwrap();
            println!("committing with dropped odb");
            println!("result: {:?}", ix.commit());
        }
        "annotated" => { let r = repo(); let id = commit(&r, None, b"a"); let a = r.find_annotated_commit(id).unwrap(); println!("refname: {:?}", a.refname()); }
        "refnames" => {
            use std::os::unix::ffi::OsStringExt;
            let r = repo(); let id = commit(&r, None, b"a");
            let path = r.path().join("refs/heads").join(std::ffi::OsString::from_vec(b"bad\xff".to_vec()));
            std::fs::write(path, format!("{}\n", id)).unwrap();
            let mut refs = r.references().unwrap();
            for name in refs.names() { println!("name: {:?}", name); }
        }
        "hash" => {
            let r = repo(); let id = r.blob(b"abc").unwrap(); let mut pb = r.packbuilder().unwrap(); pb.insert_object(id, None).unwrap();
            let mut buf = Buf::new(); pb.write_buf(&mut buf).unwrap();
            #[allow(deprecated)] { println!("nonempty pack hash: {:?}, name: {:?}", pb.hash(), pb.name()); }
        }
        _ => panic!("unknown mode"),
    }
}
