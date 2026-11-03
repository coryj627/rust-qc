// 2eafc45: BlameHunk::final_signature/orig_signature wrap null signatures of buffer blames.
use git2::{Repository, Signature};

fn main() {
    let which = std::env::args().nth(1).unwrap_or_else(|| "final".into());
    let root = std::env::temp_dir().join(format!("git2-3-{}", std::process::id()));
    let repo = Repository::init(&root).unwrap();
    std::fs::write(root.join("a.txt"), "one\n").unwrap();
    let mut idx = repo.index().unwrap();
    idx.add_path(std::path::Path::new("a.txt")).unwrap();
    let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
    let sig = Signature::now("A", "a@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "c", &tree, &[]).unwrap();

    let blame = repo.blame_file(std::path::Path::new("a.txt"), None).unwrap();
    let buffered = blame.blame_buffer(b"one\nuncommitted line\n").unwrap();
    println!("buffer blame has {} hunks", buffered.len());
    for hunk in buffered.iter() {
        println!(
            "hunk start={} final_commit_id={}",
            hunk.final_start_line(),
            hunk.final_commit_id()
        );
        let s = if which == "final" { hunk.final_signature() } else { hunk.orig_signature() };
        println!("{} signature name = {:?}", which, s.name());
    }
    println!("all hunks read without a fault");
}
