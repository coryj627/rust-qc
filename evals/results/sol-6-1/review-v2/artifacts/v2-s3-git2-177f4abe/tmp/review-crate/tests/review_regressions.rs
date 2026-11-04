use git2::{build::CheckoutBuilder, CheckoutNotificationType, MergeOptions, Repository, Signature, StashApplyOptions, Tree};
use std::{cell::Cell, fs};
use tempfile::TempDir;

fn tree<'a>(repo: &'a Repository, data: &[u8]) -> Tree<'a> {
    let blob = repo.blob(data).unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    builder.insert("file.txt", blob, 0o100644).unwrap();
    let id = builder.write().unwrap();
    repo.find_tree(id).unwrap()
}
fn fixture() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let sig = Signature::now("Review", "review@example.com").unwrap();
    let tree = tree(&repo, b"base\n");
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[]).unwrap();
    drop(tree);
    repo.checkout_head(Some(CheckoutBuilder::new().force())).unwrap();
    (dir, repo)
}

#[test]
fn dry_run_detects_conflicts() {
    let (dir, repo) = fixture();
    let target = tree(&repo, b"target\n");
    fs::write(dir.path().join("file.txt"), b"local change\n").unwrap();
    let normal = repo.checkout_tree(target.as_object(), Some(CheckoutBuilder::new().safe()));
    assert!(normal.is_err(), "control: safe checkout detects this conflict");
    let result = repo.checkout_tree(target.as_object(), Some(CheckoutBuilder::new().dry_run()));
    assert_eq!(fs::read(dir.path().join("file.txt")).unwrap(), b"local change\n");
    assert!(result.is_err(), "dry_run must detect the same conflict; got {:?}", result);
}

#[test]
fn dry_run_reports_planned_updates() {
    let (dir, repo) = fixture();
    let target = tree(&repo, b"target\n");
    let calls = Cell::new(0);
    let mut opts = CheckoutBuilder::new();
    opts.dry_run().notify_on(CheckoutNotificationType::UPDATED).notify(|_,_,_,_,_| { calls.set(calls.get()+1); true });
    repo.checkout_tree(target.as_object(), Some(&mut opts)).unwrap();
    assert_eq!(fs::read(dir.path().join("file.txt")).unwrap(), b"base\n");
    assert!(calls.get() > 0, "dry_run should report planned updates");
}

#[test]
fn skip_reuc_does_not_abort_conflicting_merge() {
    let (_dir, repo) = fixture();
    let base = tree(&repo, b"base\n");
    let ours = tree(&repo, b"ours\n");
    let theirs = tree(&repo, b"theirs\n");
    assert!(repo.merge_trees(&base, &ours, &theirs, None).unwrap().has_conflicts());
    let mut opts = MergeOptions::new();
    opts.skip_reuc(true);
    let result = repo.merge_trees(&base, &ours, &theirs, Some(&opts));
    assert!(result.is_ok(), "skip_reuc only suppresses REUC, got {:?}", result.err());
}

#[test]
fn skip_reuc_false_does_not_clear_fail_on_conflict() {
    let (_dir, repo) = fixture();
    let base = tree(&repo, b"base\n");
    let ours = tree(&repo, b"ours\n");
    let theirs = tree(&repo, b"theirs\n");
    let mut opts = MergeOptions::new();
    opts.fail_on_conflict(true);
    assert!(repo.merge_trees(&base, &ours, &theirs, Some(&opts)).is_err());
    opts.skip_reuc(false);
    assert!(repo.merge_trees(&base, &ours, &theirs, Some(&opts)).is_err(), "skip_reuc(false) must preserve fail_on_conflict(true)");
}

#[test]
fn annotated_commit_without_ref_has_no_refname() {
    let (_dir, repo) = fixture();
    let oid = repo.head().unwrap().target().unwrap();
    let commit = repo.find_annotated_commit(oid).unwrap();
    assert_eq!(commit.id(), oid);
    assert_eq!(commit.refname(), None);
}

#[test]
fn source_only_fetch_refspec_has_no_destination() {
    let (_dir, repo) = fixture();
    repo.remote("origin", "/unused").unwrap();
    repo.remote_add_fetch("origin", "refs/heads/main").unwrap();
    let remote = repo.find_remote("origin").unwrap();
    let spec = remote.refspecs().find(|s| s.str() == Some("refs/heads/main")).unwrap();
    assert_eq!(spec.src(), Some("refs/heads/main"));
    assert_eq!(spec.dst(), None);
}

#[test]
fn stash_progress_payload_tracks_moved_options() {
    fn make_options() -> StashApplyOptions<'static> {
        let mut opts = StashApplyOptions::new();
        opts.progress_cb(|_| true);
        opts
    }
    let mut opts = Box::new(make_options());
    let current_address = opts.as_mut() as *mut StashApplyOptions<'static> as *mut std::ffi::c_void;
    assert_eq!(opts.raw().progress_payload, current_address, "callback payload must point at the live options after an ordinary move");
}


#[test]
fn zero_valued_flag_predicates_recognize_defaults() {
    let actual = [git2::Sort::NONE.is_none(),
                  git2::MergeAnalysis::ANALYSIS_NONE.is_none(),
                  git2::MergePreference::NONE.is_none(),
                  git2::RepositoryInitMode::SHARED_UMASK.is_shared_umask()];
    assert_eq!(actual, [true; 4]);
}

#[test]
fn repository_shared_group_is_not_shared_all() {
    assert!(!git2::RepositoryInitMode::SHARED_GROUP.is_shared_all());
}

#[test]
fn time_ordering_agrees_with_equality_for_negative_zero() {
    let (_dir, repo) = fixture();
    let sig = Signature::new("Review", "review@example.com", &git2::Time::new(1234567890, 0)).unwrap();
    let tr = tree(&repo, b"content\n");
    let data = repo.commit_create_buffer(&sig, &sig, "time sign", &tr, &[]).unwrap();
    let positive_id = repo.odb().unwrap().write(git2::ObjectType::Commit, &data).unwrap();
    let negative_data = data.as_str().unwrap().replace(" +0000\n", " -0000\n");
    let negative_id = repo.odb().unwrap().write(git2::ObjectType::Commit, negative_data.as_bytes()).unwrap();
    let pos = repo.find_commit(positive_id).unwrap().committer().when();
    let neg = repo.find_commit(negative_id).unwrap().committer().when();
    assert_eq!(pos.sign(), '+');
    assert_eq!(neg.sign(), '-');
    assert_eq!(pos == neg, pos.cmp(&neg) == std::cmp::Ordering::Equal,
               "Ord and Eq must agree: positive={:?}, negative={:?}", pos, neg);
}

#[test]
fn apply_payload_tracks_moved_options() {
    fn make_options() -> git2::ApplyOptions<'static> {
        let mut opts = git2::ApplyOptions::new();
        opts.delta_callback(|_| true);
        opts
    }
    let mut opts = Box::new(make_options());
    let current_address = opts.as_mut() as *mut git2::ApplyOptions<'static> as *mut std::ffi::c_void;
    // Read the current raw options only; never dereference the stale payload.
    let actual = unsafe { (*opts.raw()).payload };
    assert_eq!(actual, current_address, "callback payload must point at the live options after an ordinary move");
}

#[test]
fn parent_iterator_stays_exhausted_after_missing_parent() {
    let (_dir, repo) = fixture();
    let existing = repo.head().unwrap().target().unwrap();
    let tr = tree(&repo, b"merge result\n");
    let data = format!("tree {}\nparent 1111111111111111111111111111111111111111\nparent {}\nauthor Review <review@example.com> 1234567890 +0000\ncommitter Review <review@example.com> 1234567890 +0000\n\nshallow merge\n", tr.id(), existing);
    let id = repo.odb().unwrap().write(git2::ObjectType::Commit, data.as_bytes()).unwrap();
    // A shallow boundary can omit parent objects while another branch retains one.
    fs::write(repo.path().join("shallow"), format!("{}\n", id)).unwrap();
    assert!(repo.is_shallow());
    let commit = repo.find_commit(id).unwrap();
    assert!(commit.parent(0).is_err());
    assert_eq!(commit.parent(1).unwrap().id(), existing);
    let mut parents = commit.parents();
    assert!(parents.next().is_none());
    assert!(parents.next().is_none(), "documented abort and FusedIterator require permanent exhaustion");
}

#[test]
fn swapped_apply_options_use_their_own_callback() {
    let (_dir, repo) = fixture();
    let base = tree(&repo, b"base\n");
    let target = tree(&repo, b"target\n");
    let diff = repo.diff_tree_to_tree(Some(&base), Some(&target), None).unwrap();
    let first_calls = Cell::new(0);
    let second_calls = Cell::new(0);
    let mut first = Box::new(git2::ApplyOptions::new());
    first.delta_callback(|_| { first_calls.set(first_calls.get()+1); true });
    let mut second = Box::new(git2::ApplyOptions::new());
    second.delta_callback(|_| { second_calls.set(second_calls.get()+1); true });
    // Both callback payload addresses remain live and initialized throughout.
    std::mem::swap(first.as_mut(), second.as_mut());
    repo.apply_to_tree(&base, &diff, Some(first.as_mut())).unwrap();
    assert_eq!((first_calls.get(), second_calls.get()), (0, 1),
               "the options moved into first contain the second callback");
}

#[test]
fn swapped_stash_options_use_their_own_callback() {
    let (dir, mut repo) = fixture();
    let sig = Signature::now("Review", "review@example.com").unwrap();
    fs::write(dir.path().join("file.txt"), b"stash content\n").unwrap();
    repo.stash_save2(&sig, Some("review"), None).unwrap();
    let first_calls = Cell::new(0);
    let second_calls = Cell::new(0);
    let mut first = Box::new(StashApplyOptions::new());
    first.progress_cb(|_| { first_calls.set(first_calls.get()+1); true });
    let mut second = Box::new(StashApplyOptions::new());
    second.progress_cb(|_| { second_calls.set(second_calls.get()+1); true });
    // Both callback payload addresses remain live and initialized throughout.
    std::mem::swap(first.as_mut(), second.as_mut());
    repo.stash_apply(0, Some(first.as_mut())).unwrap();
    assert_eq!(fs::read(dir.path().join("file.txt")).unwrap(), b"stash content\n");
    assert_eq!(first_calls.get(), 0, "only the callback now stored in first should run");
    assert!(second_calls.get() > 0);
}
