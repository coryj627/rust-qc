// 09269c6: Remote::get_refspec returns Refspec<'repo>, so it outlives the Remote that owns it.
use git2::Repository;

fn main() {
    let root = std::env::temp_dir().join(format!("git2-4-{}", std::process::id()));
    let repo = Repository::init(&root).unwrap();
    let refspec = {
        let remote = repo.remote("origin", "file:///nowhere").unwrap();
        remote.get_refspec(0).unwrap()
    }; // the git_remote and its refspec vector are freed here
    println!("remote dropped, reading refspec");
    println!("refspec = {:?}", refspec.str());
}
