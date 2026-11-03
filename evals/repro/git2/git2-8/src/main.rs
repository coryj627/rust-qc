// 2e3054e: Repository::submodules asserts git_submodule_lookup succeeded inside an extern "C"
// callback. No race needed: git_submodule_foreach enumerates gitlinks in HEAD without checking
// the name, while git_submodule_lookup refuses a name that fails git_submodule_name_is_valid
// (backslashes are normalized to '/', so a\..\b counts as path traversal) and returns ENOTFOUND.
use git2::{ObjectType, Oid, Repository, Signature};

fn main() {
    let root = std::env::temp_dir().join(format!("git2-8-{}", std::process::id()));
    let repo = Repository::init(&root).unwrap();

    // A tree with one gitlink entry whose name is a\..\b. A hostile remote can
    // serve this object; here it is written directly.
    let mut tree_bytes = Vec::new();
    tree_bytes.extend_from_slice(b"160000 a\\..\\b\0");
    tree_bytes.extend_from_slice(Oid::from_str("1111111111111111111111111111111111111111").unwrap().as_bytes());
    let tree_oid = repo.odb().unwrap().write(ObjectType::Tree, &tree_bytes).unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("A", "a@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "c", &tree, &[]).unwrap();

    // git_submodule__map only consults HEAD when a .gitmodules can be read.
    std::fs::write(root.join(".gitmodules"), "# empty\n").unwrap();

    println!("calling Repository::submodules");
    match repo.submodules() {
        Ok(v) => println!("Ok, {} submodules", v.len()),
        Err(e) => println!("Err(code={:?}, msg={})", e.code(), e.message()),
    }
    println!("returned normally");
}
