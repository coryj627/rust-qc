use git2::{Diff, Pathspec, PathspecFlags, PathspecMatchList};

fn matches_outlive_diff(ps: &Pathspec) -> PathspecMatchList<'_> {
    let diff = Diff::from_buffer(
        b"diff --git a/file b/file\nindex 1234567..7654321 100644\n--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n",
    ).unwrap();
    ps.match_diff(&diff, PathspecFlags::DEFAULT).unwrap()
}

fn main() {
    let _ = matches_outlive_diff;
}
