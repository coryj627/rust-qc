#[cfg(test)]
mod tests {
    use git2::{
        MergeOptions, ObjectType, Oid, RebaseOperationType, RebaseOptions, Repository, Signature,
        Time,
    };
    use std::cmp::Ordering;

    fn repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    fn tree(repo: &Repository, contents: &[u8]) -> Oid {
        let blob = repo.blob(contents).unwrap();
        let mut builder = repo.treebuilder(None).unwrap();
        builder.insert("file.txt", blob, 0o100644).unwrap();
        builder.write().unwrap()
    }

    fn commit(repo: &Repository, tree: Oid, parents: &[Oid]) -> Oid {
        let sig =
            Signature::new("Review", "review@example.com", &Time::new(1234567890, 0)).unwrap();
        let tree = repo.find_tree(tree).unwrap();
        let parents: Vec<_> = parents
            .iter()
            .map(|id| repo.find_commit(*id).unwrap())
            .collect();
        let parents: Vec<_> = parents.iter().collect();
        repo.commit(None, &sig, &sig, "review", &tree, &parents)
            .unwrap()
    }

    #[test]
    fn skip_reuc_keeps_conflicted_merge_available() {
        let (_dir, repo) = repo();
        let base = repo.find_tree(tree(&repo, b"base\n")).unwrap();
        let ours = repo.find_tree(tree(&repo, b"ours\n")).unwrap();
        let theirs = repo.find_tree(tree(&repo, b"theirs\n")).unwrap();
        assert!(repo
            .merge_trees(&base, &ours, &theirs, None)
            .unwrap()
            .has_conflicts());
        let mut opts = MergeOptions::new();
        opts.skip_reuc(true);
        let actual = repo.merge_trees(&base, &ours, &theirs, Some(&opts));
        assert!(
            actual.is_ok(),
            "skip_reuc must not enable fail_on_conflict: {:?}",
            actual.err()
        );
        assert!(actual.unwrap().has_conflicts());
    }

    #[test]
    fn skip_reuc_false_preserves_fail_on_conflict() {
        let (_dir, repo) = repo();
        let base = repo.find_tree(tree(&repo, b"base\n")).unwrap();
        let ours = repo.find_tree(tree(&repo, b"ours\n")).unwrap();
        let theirs = repo.find_tree(tree(&repo, b"theirs\n")).unwrap();
        let mut opts = MergeOptions::new();
        opts.fail_on_conflict(true).skip_reuc(false);
        assert!(
            repo.merge_trees(&base, &ours, &theirs, Some(&opts))
                .is_err(),
            "skip_reuc(false) must not clear fail_on_conflict"
        );
    }

    #[test]
    fn annotated_commit_lookup_has_no_refname() {
        let (_dir, repo) = repo();
        let id = commit(&repo, tree(&repo, b"base\n"), &[]);
        let annotated = repo.find_annotated_commit(id).unwrap();
        assert_eq!(annotated.id(), id);
        assert_eq!(annotated.refname(), None);
    }

    #[test]
    fn pick_operation_has_no_exec() {
        let (_dir, repo) = repo();
        let first = commit(&repo, tree(&repo, b"base\n"), &[]);
        let second = commit(&repo, tree(&repo, b"next\n"), &[first]);
        let branch = repo.find_annotated_commit(second).unwrap();
        let upstream = repo.find_annotated_commit(first).unwrap();
        let mut opts = RebaseOptions::new();
        opts.inmemory(true);
        let mut rebase = repo
            .rebase(Some(&branch), Some(&upstream), None, Some(&mut opts))
            .unwrap();
        let op = rebase.next().unwrap().unwrap();
        assert_eq!(op.kind(), Some(RebaseOperationType::Pick));
        assert_eq!(op.exec(), None);
    }

    #[test]
    fn time_ordering_agrees_with_equality_for_negative_zero() {
        let (_dir, repo) = repo();
        let tree = tree(&repo, b"base\n");
        let raw = format!("tree {}\nauthor Review <review@example.com> 1234567890 -0000\ncommitter Review <review@example.com> 1234567890 -0000\n\nreview\n", tree);
        let id = repo
            .odb()
            .unwrap()
            .write(ObjectType::Commit, raw.as_bytes())
            .unwrap();
        let commit = repo.find_commit(id).unwrap();
        let negative = commit.author().when();
        let positive = Time::new(negative.seconds(), 0);
        assert_eq!(negative.sign(), '-');
        assert_eq!(
            negative.cmp(&positive) == Ordering::Equal,
            negative == positive,
            "Ord and Eq must agree: {:?} vs {:?}",
            negative,
            positive
        );
    }

    #[test]
    fn prettify_legacy_encoded_message_returns_result() {
        let result = git2::message_prettify(&b"caf\xe9\n"[..], None);
        assert!(
            result.is_err(),
            "a String-returning API should reject non-UTF-8 instead of panicking"
        );
    }

    #[test]
    fn empty_buf_is_usable() {
        let buf = git2::Buf::new();
        assert!(buf.is_empty());
        assert_eq!(buf.as_str(), Some(""));
    }
    #[test]
    fn destination_is_optional_in_source_only_fetch_refspec() {
        let (_dir, repo) = repo();
        let remote = repo
            .remote_with_fetch("origin", "/work/tmp/no-server-needed", "refs/heads/main")
            .unwrap();
        let spec = remote.get_refspec(0).unwrap();
        assert_eq!(spec.src(), Some("refs/heads/main"));
        assert_eq!(spec.dst(), None);
    }

    #[test]
    fn into_inner_removes_hide_callback() {
        let (_dir, repo) = repo();
        let base = commit(&repo, tree(&repo, b"base\n"), &[]);
        let middle = commit(&repo, tree(&repo, b"middle\n"), &[base]);
        let tip = commit(&repo, tree(&repo, b"tip\n"), &[middle]);
        let calls = std::cell::Cell::new(0);
        let mut callback = |id| {
            calls.set(calls.get() + 1);
            id == tip
        };
        let walk = repo.revwalk().unwrap();
        let wrapped = walk.with_hide_callback(&mut callback).unwrap();
        let mut walk = wrapped.into_inner().unwrap();
        walk.push(tip).unwrap();
        walk.hide(base).unwrap();
        let ids = walk.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(
            ids,
            vec![tip, middle],
            "old callback must not filter the unwrapped walk; callback calls={}",
            calls.get()
        );
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn moved_apply_options_payload_tracks_current_address() {
        let mut opts = git2::ApplyOptions::new();
        opts.hunk_callback(|_| true);
        let mut opts = Box::new(opts);
        let current = (&mut *opts as *mut git2::ApplyOptions<'_>).cast::<std::ffi::c_void>();
        // SAFETY: The raw options are read immediately while their owner stays
        // alive at a stable address, and no mutation occurs during the read.
        let payload = unsafe { (*opts.raw()).payload };
        assert_eq!(
            payload, current,
            "callback payload must reference the live options after a move"
        );
    }

    #[test]
    fn moved_stash_options_payload_tracks_current_address() {
        let mut opts = git2::StashApplyOptions::new();
        opts.progress_cb(|_| true);
        let mut opts = Box::new(opts);
        let current = (&mut *opts as *mut git2::StashApplyOptions<'_>).cast::<std::ffi::c_void>();
        assert_eq!(
            opts.raw().progress_payload,
            current,
            "callback payload must reference the live options after a move"
        );
    }

    #[test]
    fn empty_remote_advertisement_is_usable() {
        let (source_dir, _source) = repo();
        let (_dir, repo) = repo();
        let mut remote = repo
            .remote_anonymous(source_dir.path().to_str().unwrap())
            .unwrap();
        remote.connect(git2::Direction::Fetch).unwrap();
        assert!(remote.list().unwrap().is_empty());
    }

    #[test]
    fn empty_blob_contents_are_usable() {
        let (_dir, repo) = repo();
        let id = repo.blob(b"").unwrap();
        assert!(repo.find_blob(id).unwrap().content().is_empty());
        assert!(repo.odb().unwrap().read(id).unwrap().data().is_empty());
    }

    #[test]
    fn empty_binary_diff_payload_is_usable() {
        let patch = b"diff --git a/file.bin b/file.bin\nindex 1234567..7654321 100644\nBinary files a/file.bin and b/file.bin differ\n";
        let diff = git2::Diff::from_buffer(patch).unwrap();
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
    fn ordinary_boundary_controls() {
        assert_eq!(git2::message_prettify("café\n\n", None).unwrap(), "café\n");
        let trailers = git2::message_trailers_strs("Title\n\nKey: first\nKey: second\n").unwrap();
        assert_eq!(
            trailers.iter().collect::<Vec<_>>(),
            vec![("Key", "first"), ("Key", "second")]
        );
        assert!(Oid::from_str("").is_err());
        assert!(Oid::from_str(&"a".repeat(41)).is_err());
        assert!(Oid::from_str(&"a".repeat(40)).is_ok());
        assert!(!git2::Config::parse_bool("").unwrap());
        assert!(git2::Config::parse_bool("yes").unwrap());
        assert_eq!(git2::Config::parse_i64("2k").unwrap(), 2048);
        let (_dir, repo) = repo();
        let id = commit(&repo, tree(&repo, b"base\n"), &[]);
        let reference = repo
            .reference("refs/heads/main", id, true, "review")
            .unwrap();
        let annotated = repo.reference_to_annotated_commit(&reference).unwrap();
        assert_eq!(annotated.refname(), Some("refs/heads/main"));
        let mut walk = repo.revwalk().unwrap();
        walk.push(id).unwrap();
        let mut cb = |_| true;
        assert_eq!(walk.with_hide_callback(&mut cb).unwrap().count(), 0);
    }
    #[test]
    fn empty_prettified_message_returns_result() {
        assert!(git2::message_prettify("", None).is_ok());
    }

    #[test]
    fn empty_config_string_returns_result() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(&path, "[review]\nempty =\n").unwrap();
        let config = git2::Config::open(&path).unwrap();
        assert_eq!(config.get_string("review.empty").unwrap(), "");
    }
}
