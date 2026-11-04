use git2::{Refspec, Repository};

fn first_refspec(repo: &Repository) -> Option<Refspec<'_>> {
    let remote = repo.find_remote("origin").ok()?;
    remote.get_refspec(0)
}

fn main() {
    let _ = first_refspec;
}
