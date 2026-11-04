#[test]
fn new_buf_is_an_empty_string() {
    let buf = git2::Buf::new();
    assert_eq!(buf.as_str(), Some(""));
}
