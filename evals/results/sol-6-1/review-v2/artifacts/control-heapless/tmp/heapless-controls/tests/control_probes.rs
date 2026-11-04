use heapless::{CString, HistoryBuf, LinearMap, String, Vec};
use heapless::binary_heap::{BinaryHeap, Max};

#[test]
fn decoy_1_slice_capacity_error_preserves_vector() {
    let mut v = Vec::<u8, 4>::from_slice(&[10, 20, 30]).unwrap();
    assert!(v.extend_from_slice(&[40, 50]).is_err());
    assert_eq!(v.as_slice(), &[10, 20, 30]);
    assert_eq!(v.len(), 3);
    // A subsequent fitting append also demonstrates that no hidden item was appended.
    v.extend_from_slice(&[60]).unwrap();
    assert_eq!(v.as_slice(), &[10, 20, 30, 60]);
    println!("decoy-1: over-capacity append returned Err with [10,20,30] intact; fitting append then produced [10,20,30,60]");
}

#[test]
fn decoy_2_full_linear_map_replaces_existing_key() {
    let mut m = LinearMap::<u8, u8, 2>::new();
    assert_eq!(m.insert(1, 10), Ok(None));
    assert_eq!(m.insert(2, 20), Ok(None));
    assert!(m.is_full());
    assert_eq!(m.insert(1, 99), Ok(Some(10)));
    assert_eq!(m.get(&1), Some(&99));
    assert_eq!(m.get(&2), Some(&20));
    assert_eq!(m.len(), 2);
    assert_eq!(m.insert(3, 30), Err((3, 30)));
    assert_eq!(m.get(&1), Some(&99));
    println!("decoy-2: full map replacement returned Ok(Some(10)), stored 99, kept length 2; new key returned Err((3,30))");
}

#[test]
fn false_1_truncate_past_string_length_is_noop() {
    let mut s = String::<8>::try_from("aéz").unwrap();
    assert_eq!(s.len(), 4);
    // 5 is within capacity but beyond length; usize::MAX is beyond both.
    for length in [5, usize::MAX] {
        s.truncate(length);
        assert_eq!(s.as_str(), "aéz");
        assert_eq!(s.len(), 4);
    }
    println!("false-1: truncate(5) and truncate(usize::MAX) both returned normally and left a four-byte string unchanged");
}

#[test]
fn false_2_cstring_terminated_append_uses_shared_terminator_capacity() {
    let mut s = CString::<5>::from_bytes_with_nul(b"ab\0").unwrap();
    assert_eq!(s.as_bytes_with_nul(), b"ab\0");
    s.extend_from_bytes(b"cd\0").unwrap();
    assert_eq!(s.as_bytes(), b"abcd");
    assert_eq!(s.as_bytes_with_nul(), b"abcd\0");
    assert_eq!(s.to_str(), Ok("abcd"));
    // Appending just a terminator to an exactly full string remains a no-op.
    s.extend_from_bytes(b"\0").unwrap();
    assert_eq!(s.as_bytes_with_nul(), b"abcd\0");
    println!("false-2: CString<5> containing ab\\0 accepted cd\\0 and produced exactly abcd\\0 (5 bytes); appending only a terminator was a no-op");
}

#[test]
fn false_3_peek_mut_drop_restores_max_heap_order() {
    let mut heap = BinaryHeap::<i32, Max, 4>::new();
    for value in [9, 7, 8, 1] {
        heap.push(value).unwrap();
    }
    {
        let mut root = heap.peek_mut().unwrap();
        assert_eq!(*root, 9);
        *root = 0;
    }
    assert_eq!(heap.peek(), Some(&8));
    assert_eq!(heap.len(), 4);
    let mut popped = std::vec::Vec::new();
    while let Some(value) = heap.pop() {
        popped.push(value);
    }
    assert_eq!(popped, [8, 7, 1, 0]);
    println!("false-3: changing the max root 9 to 0 and dropping the guard made peek return 8; pop order [8,7,1,0]");
}

#[test]
fn false_4_history_recent_survives_exact_wrap() {
    let mut h = HistoryBuf::<u8, 4>::new();
    assert_eq!(h.recent(), None);
    for value in 1..=8 {
        h.write(value);
        assert_eq!(h.recent(), Some(&value));
        if value % 4 == 0 {
            assert!(h.is_full());
            assert_eq!(h.recent_index(), Some(3));
            assert_eq!(h.len(), 4);
        }
    }
    let mut single = HistoryBuf::<u8, 1>::new();
    for value in 1..=2 {
        single.write(value);
        assert_eq!(single.recent_index(), Some(0));
        assert_eq!(single.recent(), Some(&value));
    }
    println!("false-4: capacity 4 returned Some(4)/Some(8) at exact wraps, index Some(3); capacity 1 returned its latest value on each write");
}

#[test]
fn false_5_dropping_partial_drain_restores_untouched_suffix() {
    let mut v = Vec::<i32, 5>::from_slice(&[10, 20, 30, 40, 50]).unwrap();
    {
        let mut draining = v.drain(1..4);
        assert_eq!(draining.next(), Some(20));
        assert_eq!(draining.as_slice(), &[30, 40]);
        // Drop before consuming the rest of the drained range.
    }
    assert_eq!(v.as_slice(), &[10, 50]);
    assert_eq!(v.len(), 2);
    v.push(60).unwrap();
    assert_eq!(v.as_slice(), &[10, 50, 60]);
    println!("false-5: drain(1..4), consume one item, drop => [10,50] with length 2; subsequent push => [10,50,60]");
}
