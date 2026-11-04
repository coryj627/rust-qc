use git2::{ApplyLocation, ApplyOptions, Diff, MergeOptions, ObjectType, RebaseOptions, Repository, Signature, Time};
use std::{cell::Cell, path::Path};
use tempfile::TempDir;

fn repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    (dir, repo)
}
fn tree(repo: &Repository, text: &[u8]) -> git2::Oid {
    let blob = repo.blob(text).unwrap();
    let mut b = repo.treebuilder(None).unwrap();
    b.insert("foo", blob, 0o100644).unwrap();
    b.write().unwrap()
}

#[test]
fn skip_reuc_does_not_enable_fail_on_conflict() {
    let (_dir, repo) = repo();
    let base = repo.find_tree(tree(&repo, b"base\n")).unwrap();
    let ours = repo.find_tree(tree(&repo, b"ours\n")).unwrap();
    let theirs = repo.find_tree(tree(&repo, b"theirs\n")).unwrap();
    assert!(repo.merge_trees(&base, &ours, &theirs, None).unwrap().has_conflicts());
    let mut opts = MergeOptions::new();
    opts.skip_reuc(true);
    let result = repo.merge_trees(&base, &ours, &theirs, Some(&mut opts));
    assert!(result.is_ok(), "skip_reuc unexpectedly failed: {:?}", result.err());
}

#[test]
fn moved_apply_options_keep_callbacks() {
    let (dir, repo) = repo();
    let patch = Diff::from_buffer(b"diff --git a/foo b/foo\n--- a/foo\n+++ b/foo\n@@ -1 +1 @@\n-base\n+new\n").unwrap();
    std::fs::write(dir.path().join("foo"), b"base\n").unwrap();
    let calls = Cell::new(0);
    let mut options = ApplyOptions::new();
    options.delta_callback(|_| { calls.set(calls.get() + 1); true });
    let mut moved = std::mem::replace(&mut options, ApplyOptions::new());
    let result = repo.apply(&patch, ApplyLocation::WorkDir, Some(&mut moved));
    // Both the original slot and moved value remain alive throughout the call.
    assert!(result.is_ok(), "moved options failed: {:?}; calls={}", result, calls.get());
    assert_eq!(calls.get(), 1);
    assert_eq!(std::fs::read(dir.path().join("foo")).unwrap(), b"new\n");
}

#[test]
fn source_only_fetch_refspec_has_no_destination() {
    let (_dir, repo) = repo();
    let remote = repo.remote_with_fetch("origin", "/unused-local-repo", "refs/heads/main").unwrap();
    let spec = remote.get_refspec(0).unwrap();
    assert_eq!(spec.src(), Some("refs/heads/main"));
    assert_eq!(spec.dst(), None);
}

#[test]
fn oid_annotated_commit_has_no_refname() {
    let (_dir, repo) = repo();
    let tree = repo.find_tree(tree(&repo, b"base\n")).unwrap();
    let sig = Signature::new("A", "a@example.com", &Time::new(1, 0)).unwrap();
    let id = repo.commit(None, &sig, &sig, "base", &tree, &[]).unwrap();
    let annotated = repo.find_annotated_commit(id).unwrap();
    assert_eq!(annotated.refname(), None);
}

#[test]
fn pick_operation_has_no_exec_command() {
    let (_dir, repo) = repo();
    let tree = repo.find_tree(tree(&repo, b"base\n")).unwrap();
    let sig = Signature::new("A", "a@example.com", &Time::new(1, 0)).unwrap();
    let base = repo.commit(None, &sig, &sig, "base", &tree, &[]).unwrap();
    let tip = repo.commit(None, &sig, &sig, "tip", &tree, &[&repo.find_commit(base).unwrap()]).unwrap();
    let upstream = repo.find_annotated_commit(base).unwrap();
    let branch = repo.find_annotated_commit(tip).unwrap();
    let mut opts = RebaseOptions::new();
    opts.inmemory(true);
    let mut rebase = repo.rebase(Some(&branch), Some(&upstream), None, Some(&mut opts)).unwrap();
    let op = rebase.next().unwrap().unwrap();
    assert_eq!(op.kind(), Some(git2::RebaseOperationType::Pick));
    assert_eq!(op.exec(), None);
}

#[test]
fn time_ordering_matches_equality_for_negative_zero() {
    let (_dir, repo) = repo();
    let tree_id = tree(&repo, b"base\n");
    let raw = format!("tree {}\nauthor A <a@example.com> 1 -0000\ncommitter A <a@example.com> 1 +0000\n\nmessage\n", tree_id);
    let id = repo.odb().unwrap().write(ObjectType::Commit, raw.as_bytes()).unwrap();
    let commit = repo.find_commit(id).unwrap();
    let a = commit.author().when();
    let b = commit.committer().when();
    assert_eq!(a.sign(), '-');
    assert_eq!(b.sign(), '+');
    assert_eq!(a.cmp(&b) == std::cmp::Ordering::Equal, a == b, "Ord and Eq disagree: {:?} vs {:?}", a, b);
}

#[test]
fn apply_control_without_move() {
    let (dir, repo) = repo();
    let patch = Diff::from_buffer(b"diff --git a/foo b/foo\n--- a/foo\n+++ b/foo\n@@ -1 +1 @@\n-base\n+new\n").unwrap();
    std::fs::write(dir.path().join("foo"), b"base\n").unwrap();
    let calls = Cell::new(0);
    let mut options = ApplyOptions::new();
    options.delta_callback(|_| { calls.set(calls.get() + 1); true });
    repo.apply(&patch, ApplyLocation::WorkDir, Some(&mut options)).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(std::fs::read(dir.path().join(Path::new("foo"))).unwrap(), b"new\n");
}

#[test]
fn moved_stash_options_keep_callback() {
    let (dir, mut repo) = repo();
    let tree = repo.find_tree(tree(&repo, b"base\n")).unwrap();
    let sig = Signature::new("A", "a@example.com", &Time::new(1, 0)).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[]).unwrap();
    drop(tree);
    repo.reset(&repo.head().unwrap().peel(ObjectType::Commit).unwrap(), git2::ResetType::Hard, None).unwrap();
    std::fs::write(dir.path().join("foo"), b"changed\n").unwrap();
    repo.stash_save(&sig, "stash", None).unwrap();
    let calls = Cell::new(0);
    let mut options = git2::StashApplyOptions::new();
    options.progress_cb(|_| { calls.set(calls.get() + 1); true });
    let mut moved = std::mem::replace(&mut options, git2::StashApplyOptions::new());
    repo.stash_apply(0, Some(&mut moved)).unwrap();
    assert!(calls.get() > 0);
    assert_eq!(std::fs::read(dir.path().join("foo")).unwrap(), b"changed\n");
}

#[test]
fn odb_reader_partial_and_eof() {
    use std::io::Read;
    let (_dir, repo) = repo();
    let data = b"abcdefg";
    let oid = repo.blob(data).unwrap();
    let odb = repo.odb().unwrap();
    let (mut reader, size, ty) = odb.reader(oid).unwrap();
    assert_eq!(size, data.len());
    assert_eq!(ty, ObjectType::Blob);
    let mut buf = [0; 3];
    let mut out = Vec::new();
    loop {
        let n = reader.read(&mut buf).unwrap();
        if n == 0 { break; }
        out.extend_from_slice(&buf[..n]);
    }
    assert_eq!(out, data);
    assert_eq!(reader.read(&mut buf).unwrap(), 0);
}

#[test]
fn config_implicit_bool_string_is_error_or_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "[foo]\nbar\n").unwrap();
    let mut config = git2::Config::open(&path).unwrap();
    assert_eq!(config.get_bool("foo.bar").unwrap(), true);
    let snapshot = config.snapshot().unwrap();
    let value = snapshot.get_str("foo.bar");
    assert!(value.is_err() || value == Ok(""));
}

#[test]
fn checkout_force_after_dry_run_performs_checkout() {
    let (dir, repo) = repo();
    let target = repo.find_tree(tree(&repo, b"target\n")).unwrap();
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.dry_run();
    repo.checkout_tree(target.as_object(), Some(&mut opts)).unwrap();
    assert!(!dir.path().join("foo").exists());
    opts.force();
    repo.checkout_tree(target.as_object(), Some(&mut opts)).unwrap();
    assert_eq!(std::fs::read(dir.path().join("foo")).unwrap(), b"target\n");
}

#[test]
fn checkout_force_control() {
    let (dir, repo) = repo();
    let target = repo.find_tree(tree(&repo, b"target\n")).unwrap();
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.force();
    repo.checkout_tree(target.as_object(), Some(&mut opts)).unwrap();
    assert_eq!(std::fs::read(dir.path().join("foo")).unwrap(), b"target\n");
}

#[test]
#[allow(deprecated)]
fn written_nonempty_pack_has_hash() {
    let (_dir, repo) = repo();
    let blob = repo.blob(b"base\n").unwrap();
    let mut pack = repo.packbuilder().unwrap();
    pack.insert_object(blob, None).unwrap();
    let output = TempDir::new().unwrap();
    pack.write(output.path(), 0).unwrap();
    assert!(pack.name().is_some());
    assert!(pack.hash().is_some(), "written pack with {} objects has no hash", pack.object_count());
}

#[test]
fn checkout_dry_run_detects_conflicts() {
    let (dir, repo) = repo();
    let base = repo.find_tree(tree(&repo, b"base\n")).unwrap();
    let sig = Signature::new("A", "a@example.com", &Time::new(1, 0)).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &base, &[]).unwrap();
    repo.reset(&repo.head().unwrap().peel(ObjectType::Commit).unwrap(), git2::ResetType::Hard, None).unwrap();
    std::fs::write(dir.path().join("foo"), b"dirty\n").unwrap();
    let target = repo.find_tree(tree(&repo, b"target\n")).unwrap();
    let safe = repo.checkout_tree(target.as_object(), None);
    assert!(safe.is_err(), "fixture should conflict");
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.dry_run();
    let dry_run = repo.checkout_tree(target.as_object(), Some(&mut opts));
    assert!(dry_run.is_err(), "dry run incorrectly reported success: {:?}", dry_run);
    assert_eq!(std::fs::read(dir.path().join("foo")).unwrap(), b"dirty\n");
}

#[test]
fn stash_control_without_move() {
    let (dir, mut repo) = repo();
    let tree = repo.find_tree(tree(&repo, b"base\n")).unwrap();
    let sig = Signature::new("A", "a@example.com", &Time::new(1, 0)).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[]).unwrap();
    drop(tree);
    repo.reset(&repo.head().unwrap().peel(ObjectType::Commit).unwrap(), git2::ResetType::Hard, None).unwrap();
    std::fs::write(dir.path().join("foo"), b"changed\n").unwrap();
    repo.stash_save(&sig, "stash", None).unwrap();
    let calls = Cell::new(0);
    let mut options = git2::StashApplyOptions::new();
    options.progress_cb(|_| { calls.set(calls.get() + 1); true });
    repo.stash_apply(0, Some(&mut options)).unwrap();
    assert!(calls.get() > 0);
    assert_eq!(std::fs::read(dir.path().join("foo")).unwrap(), b"changed\n");
}
