use git2::{build::CheckoutBuilder, Config, ObjectType, Oid, Pathspec, PathspecFlags,
           Repository, Signature};
use std::{fs, io::Write, panic::{catch_unwind, AssertUnwindSafe}, path::Path};
use tempfile::TempDir;

#[test]
fn decoy_1_get_path_returns_independent_owned_entry() {
    let td = TempDir::new().unwrap();
    let (entry, expected) = {
        let repo = Repository::init_bare(td.path()).unwrap();
        let blob = repo.blob(b"entry payload").unwrap();
        let child_id = {
            let mut child = repo.treebuilder(None).unwrap();
            child.insert("leaf", blob, 0o100644).unwrap();
            child.write().unwrap()
        };
        let root_id = {
            let mut root = repo.treebuilder(None).unwrap();
            root.insert("dir", child_id, 0o040000).unwrap();
            root.write().unwrap()
        };
        let tree = repo.find_tree(root_id).unwrap();
        let entry = tree.get_path(Path::new("dir/leaf")).unwrap();
        (entry, blob)
    }; // tree, builders and Repository have all been dropped.
    assert_eq!(entry.name_bytes(), b"leaf");
    assert_eq!(entry.id(), expected);
    assert_eq!(entry.kind(), Some(ObjectType::Blob));
    assert_eq!(entry.filemode(), 0o100644);
    println!("decoy-1: nested entry remains readable after Tree and Repository drop; name/id/type/mode match");
}

#[test]
fn decoy_2_checkout_paths_survive_vec_growth_and_builder_move() {
    let td = TempDir::new().unwrap();
    let repo = Repository::init(td.path()).unwrap();
    let payload = b"selected content\n";
    let blob = repo.blob(payload).unwrap();
    let mut treebuilder = repo.treebuilder(None).unwrap();
    let mut options = CheckoutBuilder::new();
    options.force().disable_pathspec_match(true);
    for i in 0..128 {
        let name = format!("selected-{i:03}");
        treebuilder.insert(&name, blob, 0o100644).unwrap();
        options.path(&name);
    }
    treebuilder.insert("excluded", blob, 0o100644).unwrap();
    let tree = repo.find_tree(treebuilder.write().unwrap()).unwrap();
    let mut moved = Box::new(options);
    repo.checkout_tree(tree.as_object(), Some(&mut moved)).unwrap();
    for i in 0..128 {
        assert_eq!(fs::read(td.path().join(format!("selected-{i:03}"))).unwrap(), payload);
    }
    assert!(!td.path().join("excluded").exists());
    println!("decoy-2: all 128 accumulated paths checked out after vector growth and builder move; excluded path absent");
}

#[test]
fn false_1_treebuilder_filter_panic_is_resumed_in_rust() {
    let td = TempDir::new().unwrap();
    let repo = Repository::init_bare(td.path()).unwrap();
    let blob = repo.blob(b"data").unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    for name in ["a", "b", "c"] {
        builder.insert(name, blob, 0o100644).unwrap();
    }
    let mut calls = 0;
    let result = catch_unwind(AssertUnwindSafe(|| {
        builder.filter(|_| {
            calls += 1;
            panic!("controlled-filter-panic");
        }).unwrap();
    }));
    let payload = result.expect_err("panic must be resumed at Rust boundary");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"controlled-filter-panic"));
    assert_eq!(calls, 1);
    assert_eq!(builder.len(), 3);
    let mut subsequent_calls = 0;
    builder.filter(|_| { subsequent_calls += 1; true }).unwrap();
    assert_eq!(subsequent_calls, 3);
    println!("false-1: expected panic caught by Rust caller, callback called once, all 3 entries retained, later filter succeeds");
}

#[test]
fn false_2_live_config_borrow_is_rejected_and_snapshot_is_stable() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("config");
    fs::write(&path, b"[probe]\nvalue = before\n").unwrap();
    let mut config = Config::open(&path).unwrap();
    let error = config.get_str("probe.value").unwrap_err();
    assert!(error.message().contains("live config"));
    let snapshot = config.snapshot().unwrap();
    let borrowed = snapshot.get_str("probe.value").unwrap();
    config.set_str("probe.value", "after").unwrap();
    assert_eq!(borrowed, "before");
    assert_eq!(snapshot.get_str("probe.value").unwrap(), "before");
    assert_eq!(config.get_string("probe.value").unwrap(), "after");
    println!("false-2: live get_str returns error: {}; borrowed snapshot remains 'before' after live value becomes 'after'", error.message());
}

#[test]
fn false_3_truncated_odb_stream_cannot_finalize() {
    let td = TempDir::new().unwrap();
    let repo = Repository::init_bare(td.path()).unwrap();
    let odb = repo.odb().unwrap();
    let mut writer = odb.writer(4, ObjectType::Blob).unwrap();
    writer.write_all(b"abc").unwrap();
    let error = writer.finalize().unwrap_err();
    assert!(error.message().contains("Invalid length"));
    let mut count = 0;
    odb.foreach(|_| { count += 1; true }).unwrap();
    assert_eq!(count, 0);
    let mut complete = odb.writer(4, ObjectType::Blob).unwrap();
    complete.write_all(b"abcd").unwrap();
    let oid = complete.finalize().unwrap();
    assert_eq!(oid, Oid::hash_object(ObjectType::Blob, b"abcd").unwrap());
    assert_eq!(odb.read(oid).unwrap().data(), b"abcd");
    println!("false-3: 3/4-byte finalize rejected with '{}'; no object published; complete 4-byte stream round-trips", error.message());
}

#[test]
fn false_4_diff_match_list_string_iterator_reports_zero() {
    let td = TempDir::new().unwrap();
    let repo = Repository::init_bare(td.path()).unwrap();
    let blob = repo.blob(b"text\n").unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    let empty = repo.find_tree(builder.write().unwrap()).unwrap();
    builder.insert("match.txt", blob, 0o100644).unwrap();
    builder.insert("other.bin", blob, 0o100644).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let diff = repo.diff_tree_to_tree(Some(&empty), Some(&tree), None).unwrap();
    assert_eq!(diff.deltas().len(), 2);
    let spec = Pathspec::new(["*.txt"]).unwrap();
    let matches = spec.match_diff(&diff, PathspecFlags::DEFAULT).unwrap();
    assert!(matches.entry(0).is_none());
    let mut strings = matches.entries();
    assert_eq!(strings.len(), 0);
    assert_eq!(strings.size_hint(), (0, Some(0)));
    assert!(strings.next().is_none());
    assert!(strings.next_back().is_none());
    let mut deltas = matches.diff_entries();
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas.next().unwrap().new_file().path(), Some(Path::new("match.txt")));
    assert_eq!(deltas.len(), 0);
    println!("false-4: nonempty diff match list has string len/size_hint/count=0; diff iterator correctly yields match.txt once");
}

fn commit(repo: &Repository, parent: Option<Oid>, label: &str) -> Oid {
    let builder = repo.treebuilder(None).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let signature = Signature::now("Probe", "<email>").unwrap();
    let parent = parent.map(|id| repo.find_commit(id).unwrap());
    let parents: Vec<_> = parent.iter().collect();
    repo.commit(None, &signature, &signature, label, &tree, &parents).unwrap()
}

#[test]
fn false_5_revwalk_reset_clears_explicit_hide_state() {
    let td = TempDir::new().unwrap();
    let repo = Repository::init_bare(td.path()).unwrap();
    let root = commit(&repo, None, "root");
    let middle = commit(&repo, Some(root), "middle");
    let tip = commit(&repo, Some(middle), "tip");
    let mut walk = repo.revwalk().unwrap();
    walk.push(tip).unwrap();
    walk.hide(middle).unwrap();
    walk.reset().unwrap(); // reset before the hidden walk is exhausted
    walk.push(tip).unwrap();
    assert_eq!(walk.by_ref().collect::<Result<Vec<_>, _>>().unwrap(), [tip, middle, root]);
    walk.push(tip).unwrap();
    walk.hide(middle).unwrap();
    assert_eq!(walk.by_ref().collect::<Result<Vec<_>, _>>().unwrap(), [tip]);
    walk.push(tip).unwrap(); // exhaustion resets automatically as well
    assert_eq!(walk.collect::<Result<Vec<_>, _>>().unwrap(), [tip, middle, root]);
    println!("false-5: explicit reset restores all 3 commits; hide still excludes ancestors when used; exhaustion also clears hide state");
}
