use heapless::{CString, Vec};
use bytes::BufMut;
use std::{collections::HashMap, ffi::CStr};

#[test]
fn bytes_put_can_fill_all_capacity() {
    let mut v = Vec::<u8, 3>::new();
    v.put_slice(b"abc");
    assert_eq!(&*v, b"abc");
}

#[test]
fn bytes_zero_length_put_on_zero_capacity() {
    let mut v = Vec::<u8, 0>::new();
    v.put_slice(b"");
    assert_eq!(&*v, b"");
}

#[test]
fn cstring_borrowed_lookup_matches_owned_lookup() {
    let key = CString::<8>::from_bytes_with_nul(b"abc\0").unwrap();
    let mut map = HashMap::new();
    map.insert(key.clone(), 42);
    assert_eq!(map.get(&key), Some(&42));
    assert_eq!(map.get::<CStr>(c"abc"), Some(&42));
}

#[test]
fn cstring_constructor_rejects_missing_terminator() {
    assert!(CString::<8>::from_bytes_with_nul(b"abc").is_err());
    assert!(CString::<8>::from_bytes_with_nul(b"").is_err());
}

#[test]
fn bytes_put_u8_fills_single_element_vec() {
    let mut v = Vec::<u8, 1>::new();
    v.put_u8(7);
    assert_eq!(&*v, &[7]);
}

#[test]
fn bytes_put_slice_fills_view() {
    let mut v = Vec::<u8, 3, u8>::new();
    let view = v.as_mut_view();
    view.put_slice(b"abc");
    assert_eq!(&*view, b"abc");
}

#[test]
fn cstring_rejects_both_empty_and_unterminated_input() {
    let actual = [b"".as_slice(), b"abc".as_slice()]
        .map(|b| CString::<8>::from_bytes_with_nul(b).is_ok());
    assert_eq!(actual, [false, false]);
}
