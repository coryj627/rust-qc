// 241e629: Remote::list passes the base pointer from git_remote_ls to slice::from_raw_parts.
use git2::{Direction, Repository};
use std::hint::black_box;

fn main() {
    let root = std::env::temp_dir().join(format!("git2-2-{}", std::process::id()));
    let local = root.join("local");
    let bare = root.join("bare");
    let repo = Repository::init(&local).unwrap();
    Repository::init_bare(&bare).unwrap();
    let url = format!("file://{}", bare.display());
    let mut remote = repo.remote("origin", &url).unwrap();
    remote.connect(Direction::Fetch).unwrap();
    println!("connected to empty remote, calling list()");
    let heads = remote.list().unwrap();
    let wrapped = black_box(Some(heads));
    // In release builds expect ptr_is_null=false next to is_none()=true. rustc tells LLVM that
    // a reference is never null, so the is_null test on it is folded to false even though the
    // same value reads back as the None niche. That contradiction is the UB showing through.
    println!(
        "len={} ptr_is_null={} Some(list).is_none()={}",
        heads.len(),
        black_box(heads).as_ptr().is_null(),
        wrapped.is_none()
    );
}
