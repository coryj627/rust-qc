use heapless::{CString, Deque};
use std::collections::HashMap;
use std::ffi::CStr;

#[test]
fn deque_full_make_contiguous_then_drain() {
    let mut d = Deque::<i32, 4>::new();
    for x in 0..4 { d.push_back(x).unwrap(); }
    for x in 0..3 { assert_eq!(d.pop_front(), Some(x)); }
    for x in 4..7 { d.push_back(x).unwrap(); }
    assert_eq!(d.make_contiguous(), &[3, 4, 5, 6]);
    for x in 3..7 { assert_eq!(d.pop_front(), Some(x)); }
    assert!(d.is_empty(), "after removing all items: len={}", d.len());
}

#[test]
fn cstring_constructor_rejects_missing_nul() {
    assert!(CString::<8>::from_bytes_with_nul(b"abc").is_err());
    assert!(CString::<8>::from_bytes_with_nul(b"").is_err());
}

#[test]
fn cstring_borrowed_lookup() {
    let mut map = HashMap::new();
    map.insert(CString::<8>::from_bytes_with_nul(b"abc\0").unwrap(), 42);
    assert_eq!(map.get::<CStr>(c"abc"), Some(&42));
}

#[derive(Default)]
struct IdentityHasher(u64);
impl std::hash::Hasher for IdentityHasher {
    fn write(&mut self, bytes: &[u8]) { self.0 = bytes[0] as u64; }
    fn write_u8(&mut self, x: u8) { self.0 = x as u64; }
    fn finish(&self) -> u64 { self.0 }
}
type IdentityMap = heapless::IndexMap<u8, u8, std::hash::BuildHasherDefault<IdentityHasher>, 4>;

#[test]
fn index_map_truncate_preserves_lookup() {
    let mut map = IdentityMap::default();
    for key in [0, 1, 4] { map.insert(key, key).unwrap(); }
    map.truncate(2);
    assert_eq!(map.iter().map(|(&k, &v)| (k,v)).collect::<Vec<_>>(), vec![(0,0),(1,1)]);
    assert_eq!(map.get(&1), Some(&1));
}

#[test]
fn index_map_retain_unwind_preserves_lookup() {
    let mut map = IdentityMap::default();
    for key in [0, 1, 2] { map.insert(key, key).unwrap(); }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        map.retain(|&key, _| match key { 0 => false, 1 => panic!("predicate failed"), _ => true });
    }));
    assert!(result.is_err());
    assert_eq!(map.iter().map(|(&k, &v)| (k,v)).collect::<Vec<_>>(), vec![(1,1),(2,2)]);
    assert_eq!(map.get(&1), Some(&1));
}

#[test]
fn sorted_list_modified_find_pop_removes_once() {
    use heapless::sorted_linked_list::{Max, SortedLinkedList};
    let mut list: SortedLinkedList<i32, Max, 3, u8> = SortedLinkedList::new_u8();
    for x in [1, 2, 3] { list.push(x).unwrap(); }
    let mut found = list.find_mut(|&x| x == 2).unwrap();
    *found = 20;
    assert_eq!(found.pop(), 20);
    assert_eq!(list.iter().copied().collect::<Vec<_>>(), vec![3, 1]);
}

#[test]
#[cfg(feature = "bytes")]
fn bytes_can_fill_exact_capacity() {
    use bytes::BufMut;
    let mut v = heapless::Vec::<u8, 4>::new();
    v.put_slice(b"abcd");
    assert_eq!(v.as_slice(), b"abcd");
    assert_eq!(v.remaining_mut(), 0);
}

#[test]
fn spsc_consumer_peek_cross_thread() {
    let mut q = heapless::spsc::Queue::<u32, 4>::new();
    let (mut producer, consumer) = q.split();
    std::thread::scope(|scope| {
        scope.spawn(move || { producer.enqueue(123).unwrap(); });
        scope.spawn(move || {
            loop {
                if let Some(x) = consumer.peek() {
                    assert_eq!(*x, 123);
                    break;
                }
                std::thread::yield_now();
            }
        });
    });
}

#[test]
fn index_map_fnv_truncate_preserves_lookup() {
    use heapless::index_map::FnvIndexMap;
    for a in 0u8..8 {
        for b in 0u8..8 {
            for c in 0u8..8 {
                if a == b || a == c || b == c { continue; }
                let mut map = FnvIndexMap::<u8, u8, 4>::new();
                for key in [a,b,c] { map.insert(key, key).unwrap(); }
                map.truncate(2);
                assert_eq!(map.get(&b), Some(&b), "insert [{a}, {b}, {c}], truncate(2)");
            }
        }
    }
}

#[test]
fn cstring_empty_input_requires_nul() {
    assert!(CString::<8>::from_bytes_with_nul(b"").is_err());
}

#[test]
fn linear_map_collect_replaces_duplicate_keys() {
    let map: heapless::LinearMap<u8, u8, 4> = [(1,10), (1,20)].into_iter().collect();
    assert_eq!((map.len(), map.get(&1).copied()), (1, Some(20)));
}

#[test]
fn linear_map_collect_duplicate_key_at_capacity() {
    let map: heapless::LinearMap<u8, u8, 1> = [(1,10), (1,20)].into_iter().collect();
    assert_eq!(map.get(&1), Some(&20));
}

#[test]
fn index_map_owned_iteration_preserves_insertion_order() {
    let map: heapless::index_map::FnvIndexMap<u8, u8, 4> = [(1,10), (2,20), (3,30)].into_iter().collect();
    let borrowed: Vec<_> = map.iter().map(|(&k, &v)| (k,v)).collect();
    let owned: Vec<_> = map.into_iter().collect();
    assert_eq!(owned, borrowed);
}
