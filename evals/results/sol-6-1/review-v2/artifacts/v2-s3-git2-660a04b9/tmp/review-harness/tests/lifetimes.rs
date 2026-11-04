use git2::{Error, RebaseOperation, RebaseOptions, Refspec, Repository};

// Compile-only checks; these functions are never called. No freed handle is accessed.
#[allow(dead_code)]
fn refspec_from_dropped_remote<'repo>(repo: &'repo Repository) -> Refspec<'repo> {
    let remote = repo.find_remote("origin").unwrap();
    remote.get_refspec(0).unwrap()
}

#[allow(dead_code)]
fn operation_from_dropped_rebase<'repo>(
    repo: &'repo Repository,
) -> Result<RebaseOperation<'repo>, Error> {
    let mut opts = RebaseOptions::new();
    opts.inmemory(true);
    let mut rebase = repo.open_rebase(Some(&mut opts))?;
    rebase.next().unwrap()
}

#[test]
fn compile_only() {
    // Compiling the functions above is the check. They must be rejected if the
    // returned child handles borrow their owners as their C backing requires.
}
