// ae77ece/4f2e641: index_matched_path_cb runs CStr::from_ptr on a null matched_pathspec.
use git2::{IndexAddOption, Repository};

fn main() {
    let op = std::env::args().nth(1).expect("add_all|update_all|remove_all");
    let specs_kind = std::env::args().nth(2).unwrap_or_else(|| "none".into());
    let root = std::env::temp_dir().join(format!("git2-7-{}-{}", op, std::process::id()));
    let repo = Repository::init(&root).unwrap();
    std::fs::write(root.join("foo"), "x").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("foo")).unwrap();
    std::fs::write(root.join("foo"), "modified").unwrap();

    let none: &[&str] = &[];
    let empty_string: &[&str] = &[""];
    let specs = if specs_kind == "none" { none } else { empty_string };
    let mut seen = 0;
    let mut cb = |p: &std::path::Path, m: &[u8]| {
        seen += 1;
        println!("callback path={:?} matched_pathspec={:?}", p, m);
        0
    };
    println!("calling Index::{} with pathspecs {:?} and a callback", op, specs);
    let res = match op.as_str() {
        "add_all" => index.add_all(specs, IndexAddOption::DEFAULT, Some(&mut cb)),
        "update_all" => index.update_all(specs, Some(&mut cb)),
        "remove_all" => index.remove_all(specs, Some(&mut cb)),
        _ => panic!("bad op"),
    };
    println!("returned {:?}, callback ran {} times", res.map_err(|e| e.message().to_string()), seen);
}
