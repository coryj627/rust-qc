use std::cell::Cell;

#[inline(never)]
fn options(calls: &Cell<usize>) -> git2::ApplyOptions<'_> {
    let mut opts = git2::ApplyOptions::new();
    opts.delta_callback(|_| {
        calls.set(calls.get() + 1);
        true
    });
    opts
}

#[test]
fn apply_options_can_be_returned_from_helper() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let patch = b"diff --git a/file.txt b/file.txt\nnew file mode 100644\nindex 0000000..ce01362\n--- /dev/null\n+++ b/file.txt\n@@ -0,0 +1 @@\n+hello\n";
    let diff = git2::Diff::from_buffer(patch).unwrap();
    let calls = Cell::new(0);
    let mut opts = options(&calls);
    repo.apply(&diff, git2::ApplyLocation::Index, Some(&mut opts))
        .unwrap();
    assert_eq!(calls.get(), 1);
}
