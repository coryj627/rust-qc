use git2::{ObjectType, Repository, Signature, Time};
fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    match mode.as_str() {
        "empty_message" => { assert_eq!(git2::message_prettify("", None).unwrap(), ""); }
        "empty_stats" => {
            let diff = repo.diff_tree_to_index(None, None, None).unwrap();
            let stats = diff.stats().unwrap();
            assert_eq!(stats.files_changed(), 0);
            let buf = stats.to_buf(git2::DiffStatsFormat::NONE, 80).unwrap();
            assert_eq!(buf.as_str(), Some(""));
        }
        "empty_binary" => {
            let old = repo.find_blob(repo.blob(b"old\0").unwrap()).unwrap();
            let new = repo.find_blob(repo.blob(b"new\0").unwrap()).unwrap();
            let mut called = false;
            repo.diff_blobs(Some(&old), None, Some(&new), None, None, None,
                Some(&mut |_, binary| {
                    called = true;
                    assert!(!binary.contains_data());
                    assert!(matches!(binary.old_file().kind(), git2::DiffBinaryKind::None));
                    assert_eq!(binary.old_file().data(), &[]);
                    true
                }), None, None).unwrap();
            assert!(called);
        }
        "buffer_blame" => {
            let blob = repo.blob(b"base\n").unwrap();
            let mut builder = repo.treebuilder(None).unwrap();
            builder.insert("foo", blob, 0o100644).unwrap();
            let tree = repo.find_tree(builder.write().unwrap()).unwrap();
            let sig = Signature::new("A", "a@example.com", &Time::new(1, 0)).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[]).unwrap();
            let original = repo.blame_file(std::path::Path::new("foo"), None).unwrap();
            let blame = original.blame_buffer(b"modified\n").unwrap();
            let hunk = blame.get_line(1).unwrap();
            assert!(hunk.final_commit_id().is_zero());
            println!("final author: {:?}", hunk.final_signature().name());
        }
        "empty_buf_mut" => {
            let mut buf = git2::Buf::new();
            let bytes: &mut [u8] = &mut buf;
            assert!(bytes.is_empty());
        }
        _ => unreachable!(),
    }
    let _ = ObjectType::Blob;
}
