#[test]
fn new_buffer_can_be_read_as_empty() {
    let buf = git2::Buf::new();
    assert!(buf.is_empty());
    assert_eq!(buf.as_str(), Some(""));
}

#[test]
fn new_buffer_can_be_mutably_borrowed_as_empty() {
    let mut buf = git2::Buf::new();
    let bytes: &mut [u8] = &mut buf;
    assert!(bytes.is_empty());
}

#[test]
fn empty_remote_reference_list_can_be_read() {
    let src_dir = tempfile::tempdir().unwrap();
    let dst_dir = tempfile::tempdir().unwrap();
    let _source = git2::Repository::init_bare(src_dir.path()).unwrap();
    let repo = git2::Repository::init(dst_dir.path()).unwrap();
    let mut remote = repo
        .remote_anonymous(src_dir.path().to_str().unwrap())
        .unwrap();
    remote.connect(git2::Direction::Fetch).unwrap();
    assert!(remote.list().unwrap().is_empty());
}

#[test]
fn binary_marker_without_data_returns_empty_slices() {
    let text = b"diff --git a/file.bin b/file.bin\nindex 0123456..abcdef0 100644\nBinary files a/file.bin and b/file.bin differ\n";
    let diff = git2::Diff::from_buffer(text).unwrap();
    let mut called = false;
    diff.foreach(
        &mut |_, _| true,
        Some(&mut |_, binary| {
            called = true;
            assert!(!binary.contains_data());
            assert!(binary.old_file().data().is_empty());
            assert!(binary.new_file().data().is_empty());
            true
        }),
        None,
        None,
    )
    .unwrap();
    assert!(called);
}

#[test]
fn buffer_blame_signature_can_be_queried() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let blob = repo.blob(b"before\n").unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    builder.insert("file.txt", blob, 0o100644).unwrap();
    let tree_id = builder.write().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Author", "a@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();
    let blame = repo
        .blame_file(std::path::Path::new("file.txt"), None)
        .unwrap();
    let changed = blame.blame_buffer(b"after\n").unwrap();
    let hunk = changed.get_line(1).unwrap();
    assert!(hunk.final_commit_id().is_zero());
    let _name = hunk.final_signature().name();
}
