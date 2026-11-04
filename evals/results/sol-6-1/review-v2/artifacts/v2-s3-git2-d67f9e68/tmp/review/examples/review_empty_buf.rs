fn main() {
    let buf = git2::Buf::new();
    assert_eq!(buf.len(), 0);
    assert_eq!(buf.as_str(), Some(""));
}
