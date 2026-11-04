fn main() {
    for _ in 0..2 {
        assert_eq!(just::run(["just", "--justfile", "/work/tmp/review/public_api/justfile"].into_iter()), Ok(()));
    }
    let summary = just::summary::summary(std::path::Path::new("/work/tmp/review/public_api/justfile")).unwrap().unwrap();
    assert!(summary.recipes.contains_key("foo"));
    assert_eq!(just::unindent("\n  hello\n"), "hello\n");
    println!("public API smoke check passed");
}
