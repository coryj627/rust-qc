use git2::{build::CheckoutBuilder, Error, Rebase, RebaseOptions, Repository};

fn rebase_outlives_options(repo: &Repository) -> Result<Rebase<'_>, Error> {
    let mut count = 0;
    let mut checkout = CheckoutBuilder::new();
    checkout.progress(|_, _, _| count += 1);
    let mut opts = RebaseOptions::new();
    opts.checkout_options(checkout);
    repo.rebase(None, None, None, Some(&mut opts))
}

fn main() {
    let _ = rebase_outlives_options;
}
