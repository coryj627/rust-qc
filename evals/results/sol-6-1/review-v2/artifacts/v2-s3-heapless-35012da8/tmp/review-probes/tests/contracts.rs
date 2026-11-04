use bytes::BufMut;
use heapless::sorted_linked_list::{Max, SortedLinkedList};
use heapless::{CString, Vec};

#[test]
fn bytes_exact_capacity() {
    let mut v = Vec::<u8, 4>::new();
    v.put_slice(b"abcd");
    assert_eq!(v.as_slice(), b"abcd");
    assert_eq!(v.remaining_mut(), 0);
}

#[test]
fn bytes_last_byte() {
    let mut v = Vec::<u8, 4>::from_slice(b"abc").unwrap();
    v.put_u8(b'd');
    assert_eq!(v.as_slice(), b"abcd");
}

#[test]
fn bytes_zero_capacity_empty_write() {
    let mut v = Vec::<u8, 0>::new();
    v.put_bytes(0, 0);
    assert!(v.is_empty());
}

#[test]
fn cstring_constructor_requires_terminator() {
    assert!(CString::<4>::from_bytes_with_nul(b"abc").is_err());
}

#[test]
fn cstring_constructor_requires_a_byte() {
    assert!(CString::<4>::from_bytes_with_nul(b"").is_err());
}

#[test]
fn sorted_modified_then_pop() {
    let mut list = SortedLinkedList::<i32, Max, 3, u8>::new_u8();
    for n in [1, 2, 3] {
        list.push(n).unwrap();
    }
    let mut entry = list.find_mut(|&n| n == 2).unwrap();
    *entry = 20;
    assert_eq!(entry.pop(), 20);
    let remaining: std::vec::Vec<_> = list.iter().copied().collect();
    assert_eq!(remaining, [3, 1]);
}

#[test]
fn map_truncate_preserves_lookups() {
    use heapless::index_map::FnvIndexMap;
    for a in 0..8u32 {
        for b in 0..8u32 {
            for c in 0..8u32 {
                if a == b || a == c || b == c {
                    continue;
                }
                let mut map = FnvIndexMap::<u32, u32, 4>::new();
                map.insert(a, a).unwrap();
                map.insert(b, b).unwrap();
                map.insert(c, c).unwrap();
                map.truncate(2);
                assert_eq!(map.get(&a), Some(&a), "keys = {a},{b},{c}");
                assert_eq!(map.get(&b), Some(&b), "keys = {a},{b},{c}");
                assert_eq!(map.get(&c), None, "keys = {a},{b},{c}");
            }
        }
    }
}

#[test]
fn cstring_borrowed_lookup() {
    let key = CString::<8>::from_bytes_with_nul(b"abc\0").unwrap();
    let mut map = std::collections::HashMap::new();
    map.insert(key, 7);
    assert_eq!(map.get(c"abc"), Some(&7));
}

#[test]
fn resize_default_creates_independent_values() {
    use std::{cell::Cell, rc::Rc};
    let mut values = Vec::<Rc<Cell<u8>>, 3>::new();
    values.resize_default(3).unwrap();
    values[0].set(7);
    assert_eq!(values[1].get(), 0);
    assert_eq!(values[2].get(), 0);
}

#[test]
fn owning_map_iteration_keeps_order() {
    let mut map = heapless::index_map::FnvIndexMap::<u8, u8, 4>::new();
    for n in [1, 2, 3] {
        map.insert(n, n).unwrap();
    }
    let borrowed: std::vec::Vec<_> = map.iter().map(|(&k, &v)| (k, v)).collect();
    let owned: std::vec::Vec<_> = map.into_iter().collect();
    assert_eq!(owned, borrowed);
}

#[test]
fn map_retain_unwind_preserves_kept_values() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let mut map = heapless::index_map::FnvIndexMap::<u32, u32, 4>::new();
    for n in [0, 1, 2] {
        map.insert(n, n).unwrap();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        map.retain(|&key, _| match key {
            0 => false,
            1 => panic!("predicate failed"),
            _ => true,
        });
    }));
    assert!(result.is_err());
    assert_eq!(
        map.iter()
            .map(|(&k, &v)| (k, v))
            .collect::<std::vec::Vec<_>>(),
        [(1, 1), (2, 2)]
    );
    assert_eq!(map.get(&1), Some(&1));
}

#[test]
fn deque_contiguous_full_then_drain() {
    let mut q = heapless::Deque::<u8, 2>::new();
    q.push_back(0).unwrap();
    q.push_back(1).unwrap();
    assert_eq!(q.pop_front(), Some(0));
    q.push_back(2).unwrap();
    assert_eq!(q.make_contiguous(), [1, 2]);
    assert_eq!(q.pop_front(), Some(1));
    assert_eq!(q.pop_front(), Some(2));
    assert!(
        q.is_empty(),
        "after draining: len={}, slices={:?}",
        q.len(),
        q.as_slices()
    );
}

#[test]
fn linear_map_collect_replaces_duplicates() {
    let map: heapless::LinearMap<u8, u8, 2> = [(1, 10), (1, 20)].into_iter().collect();
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&20));
}

#[test]
fn linear_map_duplicates_do_not_exhaust_capacity() {
    let map: heapless::LinearMap<u8, u8, 1> = [(1, 10), (1, 20)].into_iter().collect();
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&20));
}

#[test]
fn linear_map_eq_bound_is_reflexive() {
    fn requires_eq<T: Eq>(value: &T) { assert!(value == value); }
    let mut map = heapless::LinearMap::<u8, f32, 1>::new();
    map.insert(1, f32::NAN).unwrap();
    requires_eq(&map);
}
