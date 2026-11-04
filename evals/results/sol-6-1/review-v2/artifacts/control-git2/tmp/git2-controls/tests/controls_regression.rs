use git2::{Config, ErrorCode, IndexEntry, IndexTime, ObjectType, Oid, Repository,
           Signature, StatusOptions, Time, TreeWalkMode, TreeWalkResult};
use std::{fs, path::Path};
use tempfile::TempDir;

fn repo() -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let repo = Repository::init(td.path()).unwrap();
    repo.config().unwrap().set_bool("core.autocrlf", false).unwrap();
    (td, repo)
}

fn commit(repo: &Repository, message: &str, parent: Option<Oid>) -> Oid {
    let tree_id = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::new("Control Author", "control@example.invalid", &Time::new(1_700_000_000, 0)).unwrap();
    let parents: Vec<_> = parent.into_iter().map(|id| repo.find_commit(id).unwrap()).collect();
    let refs: Vec<_> = parents.iter().collect();
    repo.commit(None, &sig, &sig, message, &tree, &refs).unwrap()
}

#[test]
fn decoy_1_config_i32_rejects_out_of_range_without_wrapping() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("config");
    fs::write(&path, "[control]\nupper = 2147483648\nlower = -2147483649\nmax = 2147483647\nmin = -2147483648\n").unwrap();
    let cfg = Config::open(&path).unwrap();
    assert_eq!(cfg.get_i64("control.upper").unwrap(), 2_147_483_648);
    assert_eq!(cfg.get_i64("control.lower").unwrap(), -2_147_483_649);
    for key in ["control.upper", "control.lower"] {
        let error = cfg.get_i32(key).unwrap_err();
        println!("decoy-1: {key}: get_i32 returned Err({:?}); get_i64 preserved value", error.code());
    }
    assert_eq!(cfg.get_i32("control.max").unwrap(), i32::MAX);
    assert_eq!(cfg.get_i32("control.min").unwrap(), i32::MIN);
    println!("decoy-1: exact i32 endpoints accepted");
}

#[test]
fn decoy_2_tree_skip_prunes_only_the_selected_subtree() {
    let (_td, repo) = repo();
    let blob = repo.blob(b"payload\n").unwrap();
    let mut child = repo.treebuilder(None).unwrap();
    child.insert("child.txt", blob, 0o100644).unwrap();
    let child_id = child.write().unwrap();
    let mut root = repo.treebuilder(None).unwrap();
    root.insert("a_skip", child_id, 0o040000).unwrap();
    root.insert("z_keep.txt", blob, 0o100644).unwrap();
    let root_id = root.write().unwrap();
    let tree = repo.find_tree(root_id).unwrap();
    let mut seen = Vec::new();
    tree.walk(TreeWalkMode::PreOrder, |prefix, entry| {
        let name = entry.name().unwrap();
        seen.push(format!("{prefix}{name}"));
        if name == "a_skip" { TreeWalkResult::Skip } else { TreeWalkResult::Ok }
    }).unwrap();
    assert_eq!(seen, ["a_skip", "z_keep.txt"]);
    println!("decoy-2: walk returned Ok, visited {seen:?}; skipped child was not visited");
}

#[test]
fn false_1_revwalk_range_excludes_left_endpoint_and_ancestors() {
    let (_td, repo) = repo();
    let a = commit(&repo, "a", None);
    let b = commit(&repo, "b", Some(a));
    let c = commit(&repo, "c", Some(b));
    let mut walk = repo.revwalk().unwrap();
    walk.push_range(&format!("{b}..{c}")).unwrap();
    let got: Vec<Oid> = walk.collect::<Result<_, _>>().unwrap();
    assert_eq!(got, [c]);
    println!("false-1: linear A <- B <- C, B..C yielded only C; neither B nor A appeared");
    let mut empty = repo.revwalk().unwrap();
    empty.push_range(&format!("{b}..{b}")).unwrap();
    assert!(empty.next().is_none());
    println!("false-1: B..B yielded no commits");
}

#[test]
fn false_2_raw_commit_message_preserves_leading_newlines() {
    let (_td, repo) = repo();
    let message = "\n\nsubject\n\nbody\n";
    let id = commit(&repo, message, None);
    let commit = repo.find_commit(id).unwrap();
    assert_eq!(commit.message_raw_bytes(), message.as_bytes());
    assert_eq!(commit.message_raw(), Some(message));
    assert_eq!(commit.message(), Some("subject\n\nbody\n"));
    println!("false-2: message_raw() = {:?}; message() = {:?}", commit.message_raw(), commit.message());
}

#[test]
fn false_3_index_buffer_replaces_stale_object_id_and_size() {
    let (_td, repo) = repo();
    let old_id = repo.blob(b"old\n").unwrap();
    let entry = IndexEntry {
        ctime: IndexTime::new(0, 0), mtime: IndexTime::new(0, 0),
        dev: 0, ino: 0, mode: 0o100644, uid: 0, gid: 0,
        file_size: 1, id: old_id, flags: 0, flags_extended: 0,
        path: b"buffer.txt".to_vec(),
    };
    let data = b"new buffer contents\n";
    let expected_id = Oid::hash_object(ObjectType::Blob, data).unwrap();
    let mut index = repo.index().unwrap();
    index.add_frombuffer(&entry, data).unwrap();
    index.write().unwrap();
    let mut reopened = repo.index().unwrap();
    reopened.read(true).unwrap();
    let staged = reopened.get_path(Path::new("buffer.txt"), 0).unwrap();
    assert_ne!(staged.id, old_id);
    assert_eq!(staged.id, expected_id);
    assert_eq!(staged.file_size as usize, data.len());
    assert_eq!(repo.find_blob(staged.id).unwrap().content(), data);
    let tree_id = reopened.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    assert_eq!(tree.get_name("buffer.txt").unwrap().id(), expected_id);
    println!("false-3: persisted index and written tree use new blob; size {} -> {}; blob contains all {} bytes", entry.file_size, staged.file_size, data.len());
}

#[test]
fn false_4_force_does_not_bypass_reference_expected_target() {
    let (_td, repo) = repo();
    let a = commit(&repo, "a", None);
    let b = commit(&repo, "b", Some(a));
    let c = commit(&repo, "c", Some(b));
    let name = "refs/heads/control";
    repo.reference(name, b, false, "initialize").unwrap();
    let error = match repo.reference_matching(name, c, true, a, "stale expected target") {
        Err(error) => error,
        Ok(_) => panic!("stale expected target unexpectedly succeeded"),
    };
    assert_eq!(error.code(), ErrorCode::Modified);
    assert_eq!(repo.find_reference(name).unwrap().target(), Some(b));
    repo.reference_matching(name, c, true, b, "correct expected target").unwrap();
    assert_eq!(repo.find_reference(name).unwrap().target(), Some(c));
    println!("false-4: force=true with stale expected A returned Modified and retained B; expected B updated to C");
}

#[test]
fn false_5_reused_status_options_can_disable_untracked() {
    let (td, repo) = repo();
    fs::write(td.path().join("untracked.txt"), "contents\n").unwrap();
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let shown = repo.statuses(Some(&mut opts)).unwrap();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown.get(0).unwrap().path(), Some("untracked.txt"));
    assert!(shown.get(0).unwrap().status().is_wt_new());
    drop(shown);
    opts.include_untracked(false);
    assert!(repo.statuses(Some(&mut opts)).unwrap().is_empty());
    opts.include_untracked(true);
    assert_eq!(repo.statuses(Some(&mut opts)).unwrap().len(), 1);
    println!("false-5: same options object with include_untracked true -> false -> true produced 1 -> 0 -> 1 status entries");
}
