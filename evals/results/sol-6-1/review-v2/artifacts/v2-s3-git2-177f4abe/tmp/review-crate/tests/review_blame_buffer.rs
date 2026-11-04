use git2::{Repository, Signature};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn changed_buffer_line_has_usable_signature() {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let blob = repo.blob(b"base\n").unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    builder.insert("file.txt", blob, 0o100644).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let sig = Signature::now("Review", "review@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[]).unwrap();
    let blame = repo.blame_file(Path::new("file.txt"), None).unwrap();
    let changed = blame.blame_buffer(b"new line\n").unwrap();
    let hunk = changed.get_line(1).unwrap();
    assert!(hunk.final_commit_id().is_zero());
    // No documented precondition excludes ordinary uncommitted hunks.
    let _name = hunk.final_signature().name();
}
