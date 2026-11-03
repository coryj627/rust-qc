// 2027edb: ReferenceNames unwraps str::from_utf8, so a non-UTF-8 ref name panics.
use git2::{Repository, Signature};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

fn main() {
    let root = std::env::temp_dir().join(format!("git2-6-{}", std::process::id()));
    let repo = Repository::init(&root).unwrap();
    let tree = repo.find_tree(repo.index().unwrap().write_tree().unwrap()).unwrap();
    let sig = Signature::now("A", "a@example.com").unwrap();
    let oid = repo.commit(Some("refs/heads/ok"), &sig, &sig, "c", &tree, &[]).unwrap();
    // A loose ref as it would arrive in any repository on a Unix filesystem.
    let name = root.join(".git/refs/heads").join(OsStr::from_bytes(b"bad-\xff-name"));
    std::fs::write(&name, format!("{}\n", oid)).unwrap();

    println!("iterating reference names");
    for item in repo.references().unwrap().names() {
        match item {
            Ok(n) => println!("Ok({:?})", n),
            Err(e) => println!("Err({})", e.message()),
        }
    }
    println!("iteration finished without a panic");
}
