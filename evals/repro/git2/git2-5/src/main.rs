// f016ecc: MergeOptions::skip_reuc sets GIT_MERGE_FAIL_ON_CONFLICT rather than GIT_MERGE_SKIP_REUC.
//
// Documented behaviour (src/merge.rs and libgit2 merge.h): "Do not write the REUC
// extension on the generated index". It says nothing about failing on conflict;
// fail_on_conflict() is the separate option for that.
use git2::{Commit, ErrorCode, MergeOptions, Oid, Repository, ResetType, Signature};
use std::path::Path;

fn commit<'r>(repo: &'r Repository, name: &str, parents: &[&Commit<'_>], file: &str, body: &str) -> Commit<'r> {
    std::fs::write(repo.workdir().unwrap().join(file), body).unwrap();
    let mut idx = repo.index().unwrap();
    idx.add_path(Path::new(file)).unwrap();
    idx.write().unwrap();
    let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
    let sig = Signature::new("A", "a@example.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    let oid: Oid = repo.commit(None, &sig, &sig, name, &tree, parents).unwrap();
    repo.find_commit(oid).unwrap()
}

fn main() {
    let root = std::env::temp_dir().join(format!("git2-5-{}", std::process::id()));
    let repo = Repository::init(&root).unwrap();
    let base_text = "1\n2\n3\n4\n5\n6\n7\n8\n9\n";
    let base = commit(&repo, "base", &[], "f.txt", base_text);
    repo.set_head_detached(base.id()).unwrap();
    let ours = commit(&repo, "ours", &[&base], "f.txt", "ONE\n2\n3\n4\n5\n6\n7\n8\n9\n");
    let theirs = commit(&repo, "theirs", &[&base], "f.txt", "1\n2\n3\n4\n5\n6\n7\n8\nNINE\n");
    let clash = commit(&repo, "clash", &[&base], "f.txt", "UNO\n2\n3\n4\n5\n6\n7\n8\n9\n");
    repo.set_head_detached(ours.id()).unwrap();
    repo.reset(ours.as_object(), ResetType::Hard, None).unwrap();

    let mut failures = 0;

    // Check 1: a conflicting merge with only skip_reuc(true) set must still return
    // an index holding the conflict.
    let mut opts = MergeOptions::new();
    opts.skip_reuc(true);
    match repo.merge_commits(&ours, &clash, Some(&opts)) {
        Ok(index) => println!("check1 ok: Ok(index), has_conflicts={}", index.has_conflicts()),
        Err(e) => {
            failures += 1;
            println!(
                "check1 WRONG: skip_reuc(true) made the merge fail: code={:?} is_merge_conflict={} msg={}",
                e.code(),
                e.code() == ErrorCode::MergeConflict,
                e.message()
            );
        }
    }

    // Check 2: a clean content merge with skip_reuc(true) must leave no REUC
    // extension in the index that git_merge writes to disk.
    let mut opts = MergeOptions::new();
    opts.skip_reuc(true);
    let annotated = repo.find_annotated_commit(theirs.id()).unwrap();
    repo.merge(&[&annotated], Some(&mut opts), None).unwrap();
    let on_disk = std::fs::read(root.join(".git/index")).unwrap();
    let has_reuc = on_disk.windows(4).any(|w| w == b"REUC");
    if has_reuc {
        failures += 1;
        println!("check2 WRONG: .git/index contains a REUC extension despite skip_reuc(true)");
    } else {
        println!("check2 ok: no REUC extension in .git/index");
    }

    println!("wrong_results={}", failures);
    std::process::exit(if failures == 2 { 42 } else { 0 });
}
