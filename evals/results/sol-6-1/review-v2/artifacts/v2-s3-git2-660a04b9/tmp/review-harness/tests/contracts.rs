use git2::{Error, MergeOptions, Oid, RebaseOptions, Repository, Signature, Time};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    (dir, repo)
}

fn tree(repo: &Repository, text: &[u8]) -> Oid {
    let blob = repo.blob(text).unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    builder.insert("file.txt", blob, 0o100644).unwrap();
    builder.write().unwrap()
}

fn commit(repo: &Repository, tree: Oid, parents: &[Oid]) -> Oid {
    let sig = Signature::new("Author", "a@example.com", &Time::new(1234567890, 0)).unwrap();
    let tree = repo.find_tree(tree).unwrap();
    let parents: Vec<_> = parents
        .iter()
        .map(|id| repo.find_commit(*id).unwrap())
        .collect();
    let parents: Vec<_> = parents.iter().collect();
    repo.commit(None, &sig, &sig, "a commit", &tree, &parents)
        .unwrap()
}

#[test]
fn skip_reuc_preserves_conflict_handling() -> Result<(), Error> {
    let (_dir, repo) = repo();
    let base = repo.find_tree(tree(&repo, b"base\n"))?;
    let ours = repo.find_tree(tree(&repo, b"ours\n"))?;
    let theirs = repo.find_tree(tree(&repo, b"theirs\n"))?;
    assert!(repo
        .merge_trees(&base, &ours, &theirs, None)?
        .has_conflicts());
    let mut opts = MergeOptions::new();
    opts.skip_reuc(true);
    let result = repo.merge_trees(&base, &ours, &theirs, Some(&opts));
    assert!(
        result.is_ok(),
        "skip_reuc must not turn conflicts into errors: {:?}",
        result.err()
    );
    Ok(())
}

#[test]
fn oid_annotated_commit_has_no_refname() {
    let (_dir, repo) = repo();
    let id = commit(&repo, tree(&repo, b"base\n"), &[]);
    let annotated = repo.find_annotated_commit(id).unwrap();
    assert_eq!(annotated.id(), id);
    assert_eq!(annotated.refname(), None);
}

#[test]
fn pick_operation_has_no_exec_command() {
    let (_dir, repo) = repo();
    let root = commit(&repo, tree(&repo, b"base\n"), &[]);
    let tip = commit(&repo, tree(&repo, b"changed\n"), &[root]);
    let branch = repo.find_annotated_commit(tip).unwrap();
    let upstream = repo.find_annotated_commit(root).unwrap();
    let mut opts = RebaseOptions::new();
    opts.inmemory(true);
    let mut rebase = repo
        .rebase(Some(&branch), Some(&upstream), None, Some(&mut opts))
        .unwrap();
    let op = rebase.next().unwrap().unwrap();
    assert_eq!(op.kind(), Some(git2::RebaseOperationType::Pick));
    assert_eq!(op.exec(), None);
}

#[test]
fn prettify_legacy_encoded_message_does_not_panic() {
    let result = catch_unwind(|| git2::message_prettify(&b"caf\xe9\n"[..], None));
    assert!(
        result.is_ok(),
        "non-UTF-8 input should return an error or documented conversion"
    );
}

#[test]
fn absent_fetch_destination_does_not_panic() {
    let (_dir, repo) = repo();
    drop(repo.remote("origin", "/unused/local/path").unwrap());
    repo.remote_add_fetch("origin", "refs/heads/topic").unwrap();
    let remote = repo.find_remote("origin").unwrap();
    let spec = remote
        .refspecs()
        .find(|s| s.str() == Some("refs/heads/topic"))
        .unwrap();
    assert_eq!(spec.src(), Some("refs/heads/topic"));
    assert_eq!(spec.dst(), None);
}

#[test]
fn matches_path_accepts_current_directory_prefix() {
    let spec = git2::Pathspec::new(["*.rs"]).unwrap();
    assert!(spec.matches_path(std::path::Path::new("lib.rs"), git2::PathspecFlags::DEFAULT));
    let result = catch_unwind(AssertUnwindSafe(|| {
        spec.matches_path(
            std::path::Path::new("./lib.rs"),
            git2::PathspecFlags::DEFAULT,
        )
    }));
    assert!(result.is_ok(), "a relative path with ./ should not panic");
}

#[test]
fn configured_helpers_are_tried_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "[credential]\n\thelper = \"!f() { echo username=alice; }; f\"\n\thelper = \"!f() { echo password=dummy; }; f\"\n").unwrap();
    let cfg = git2::Config::open(&path).unwrap();
    assert_eq!(
        git2::CredentialHelper::new("https://example.com/project")
            .config(&cfg)
            .execute(),
        Some(("alice".into(), "dummy".into()))
    );
}

#[test]
fn ip_address_is_passed_as_host_to_helper() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "[credential]\n\thelper = \"!f() { while IFS= read -r line; do case $line in host=*) echo username=${line#host=};; esac; done; echo password=dummy; }; f\"\n").unwrap();
    let cfg = git2::Config::open(&path).unwrap();
    assert_eq!(
        git2::CredentialHelper::new("https://127.0.0.1/project")
            .config(&cfg)
            .execute(),
        Some(("127.0.0.1".into(), "dummy".into()))
    );
}

#[test]
fn into_inner_removes_hide_callback_for_full_history() {
    let (_dir, repo) = repo();
    let root = commit(&repo, tree(&repo, b"base\n"), &[]);
    let tip = commit(&repo, tree(&repo, b"changed\n"), &[root]);
    let calls = std::cell::Cell::new(0);
    let mut hide = |_| {
        calls.set(calls.get() + 1);
        true
    };
    let walk = repo.revwalk().unwrap();
    let wrapper = walk.with_hide_callback(&mut hide).unwrap();
    let mut walk = wrapper.into_inner().unwrap();
    walk.push(tip).unwrap();
    let visited = walk.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        (visited.len(), calls.get()),
        (2, 0),
        "the returned walker should traverse both commits without calling the old callback"
    );
    assert_eq!(calls.get(), 0, "the removed callback must not run");
}

#[test]
fn time_ordering_agrees_with_equality_for_negative_zero() {
    let (_dir, repo) = repo();
    let tree = tree(&repo, b"base\n");
    let data = format!("tree {tree}\nauthor Author <a@example.com> 1234567890 -0000\ncommitter Author <a@example.com> 1234567890 +0000\n\nmessage\n");
    let id = repo
        .odb()
        .unwrap()
        .write(git2::ObjectType::Commit, data.as_bytes())
        .unwrap();
    let commit = repo.find_commit(id).unwrap();
    let author = commit.author().when();
    let committer = commit.committer().when();
    assert_eq!(author.sign(), '-');
    assert_eq!(committer.sign(), '+');
    assert_eq!(
        author.cmp(&committer) == std::cmp::Ordering::Equal,
        author == committer
    );
}

#[test]
#[allow(deprecated)]
fn nonempty_written_pack_has_a_hash() {
    let (_dir, repo) = repo();
    let blob = repo.blob(b"ordinary blob").unwrap();
    let mut builder = repo.packbuilder().unwrap();
    builder.insert_object(blob, None).unwrap();
    builder.write(repo.path(), 0).unwrap();
    let name = builder.name().unwrap();
    let expected = Oid::from_str(name).unwrap();
    assert_eq!(builder.hash(), Some(expected));
}

#[test]
fn parent_iterator_stays_finished_after_missing_parent() {
    let (_dir, repo) = repo();
    let tree = tree(&repo, b"base\n");
    let root = commit(&repo, tree, &[]);
    let missing = "1111111111111111111111111111111111111111";
    let data = format!("tree {tree}\nparent {missing}\nparent {root}\nauthor Author <a@example.com> 1234567890 +0000\ncommitter Author <a@example.com> 1234567890 +0000\n\nmerge\n");
    let id = repo
        .odb()
        .unwrap()
        .write(git2::ObjectType::Commit, data.as_bytes())
        .unwrap();
    let merge = repo.find_commit(id).unwrap();
    let mut parents = merge.parents();
    assert!(parents.next().is_none());
    assert!(
        parents.next().is_none(),
        "FusedIterator must continue returning None after a missing parent"
    );
}
