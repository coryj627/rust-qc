
use heapless::{CString, Deque, IndexMap};
use heapless::sorted_linked_list::{Max, SortedLinkedList};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{BuildHasherDefault, Hash, Hasher};

#[test]
fn bytes_can_fill_exact_capacity() {
    use bytes::BufMut;
    let mut vec = heapless::Vec::<u8, 4>::new();
    vec.put_slice(b"abcd");
    assert_eq!(vec.as_slice(), b"abcd");
}

#[test]
fn cstring_constructor_rejects_missing_nul() {
    assert!(CString::<8>::from_bytes_with_nul(b"hello").is_err());
}

#[test]
fn cstring_hash_matches_borrowed_cstr() {
    let key = CString::<8>::from_bytes_with_nul(b"hello\0").unwrap();
    let mut owned_hash = DefaultHasher::new();
    let mut borrowed_hash = DefaultHasher::new();
    key.hash(&mut owned_hash);
    key.as_c_str().hash(&mut borrowed_hash);
    assert_eq!(owned_hash.finish(), borrowed_hash.finish());
}

#[test]
fn cstring_borrowed_key_lookup() {
    let key = CString::<8>::from_bytes_with_nul(b"hello\0").unwrap();
    let mut map = HashMap::new();
    map.insert(key, 7);
    assert_eq!(map.get(c"hello"), Some(&7));
}

#[derive(Default)]
struct IdentityHasher(u64);
impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, bytes: &[u8]) { self.0 = bytes[0].into(); }
}

#[test]
fn map_truncate_preserves_lookup_for_retained_entries() {
    let mut map = IndexMap::<usize, u8, BuildHasherDefault<IdentityHasher>, 4>::default();
    map.insert(1, 10).unwrap();
    map.insert(0, 20).unwrap();
    map.insert(4, 30).unwrap();
    assert_eq!(map.get(&1), Some(&10));
    map.truncate(1);
    assert_eq!(map.iter().map(|(k,v)| (*k,*v)).collect::<Vec<_>>(), [(1,10)]);
    assert_eq!(map.get(&1), Some(&10));
}

#[test]
fn sorted_list_pop_after_mutation_removes_only_selected_item() {
    let mut list = SortedLinkedList::<i32, Max, 3, u8>::new_u8();
    list.push(1).unwrap();
    list.push(2).unwrap();
    list.push(3).unwrap();
    let mut found = list.find_mut(|v| *v == 2).unwrap();
    *found = 20;
    assert_eq!(found.pop(), 20);
    assert_eq!(list.iter().copied().take(4).collect::<Vec<_>>(), [3, 1]);
}

#[test]
fn deque_full_make_contiguous_can_be_emptied() {
    let mut deque = Deque::<i32, 4>::new();
    deque.extend([0,1,2,3]);
    assert_eq!(deque.pop_front(), Some(0));
    assert_eq!(deque.pop_front(), Some(1));
    deque.extend([4,5]);
    assert_eq!(deque.make_contiguous(), [2,3,4,5]);
    for value in 2..6 { assert_eq!(deque.pop_front(), Some(value)); }
    assert!(deque.is_empty(), "after removing all four elements, len = {}", deque.len());
}

#[test]
fn linear_map_collect_uses_one_entry_per_key() {
    let map: heapless::LinearMap<_, _, 4> = [(1, "old"), (1, "new")].into_iter().collect();
    assert_eq!((map.len(), map.get(&1).copied()), (1, Some("new")));
}

#[test]
fn linear_map_collect_duplicate_keys_do_not_exhaust_capacity() {
    let map: heapless::LinearMap<_, _, 1> = [(1, "old"), (1, "new")].into_iter().collect();
    assert_eq!(map.get(&1), Some(&"new"));
}

#[test]
fn resize_default_initializes_each_element_independently() {
    use std::{cell::Cell, rc::Rc};
    let mut vec = heapless::Vec::<Rc<Cell<u8>>, 4>::new();
    vec.resize_default(2).unwrap();
    vec[0].set(1);
    assert_eq!(vec[1].get(), 0, "the second default must be independent of the first");
}

#[test]
fn fnv_map_truncate_preserves_retained_entry() {
    let mut map = heapless::index_map::FnvIndexMap::<u8, u8, 4>::new();
    map.insert(1, 10).unwrap();
    map.insert(0, 20).unwrap();
    map.insert(4, 30).unwrap();
    assert_eq!(map.get(&1), Some(&10));
    map.truncate(1);
    assert_eq!(map.iter().map(|(k,v)| (*k,*v)).collect::<Vec<_>>(), [(1,10)]);
    assert_eq!(map.get(&1), Some(&10));
}

#[test]
fn history_clone_unwind_drops_completed_clones() {
    use std::{cell::Cell, rc::Rc, panic::{catch_unwind, AssertUnwindSafe}};
    struct Item { fail_clone: bool, is_clone: bool, live_clones: Rc<Cell<usize>> }
    impl Clone for Item {
        fn clone(&self) -> Self {
            assert!(!self.fail_clone, "second clone fails");
            self.live_clones.set(self.live_clones.get() + 1);
            Self { fail_clone: false, is_clone: true, live_clones: self.live_clones.clone() }
        }
    }
    impl Drop for Item {
        fn drop(&mut self) {
            if self.is_clone { self.live_clones.set(self.live_clones.get() - 1); }
        }
    }
    let live_clones = Rc::new(Cell::new(0));
    let mut history = heapless::HistoryBuf::<Item, 2>::new();
    history.write(Item { fail_clone: false, is_clone: false, live_clones: live_clones.clone() });
    history.write(Item { fail_clone: true, is_clone: false, live_clones: live_clones.clone() });
    assert!(catch_unwind(AssertUnwindSafe(|| { let _ = history.clone(); })).is_err());
    assert_eq!(live_clones.get(), 0, "a completed clone was leaked on unwind");
}

#[test]
fn occupied_entry_key_is_the_stored_key() {
    #[derive(Debug)]
    struct Key { id: u8, label: &'static str }
    impl PartialEq for Key { fn eq(&self, rhs: &Self) -> bool { self.id == rhs.id } }
    impl Eq for Key {}
    impl Hash for Key { fn hash<H: Hasher>(&self, h: &mut H) { self.id.hash(h) } }
    let mut map = heapless::index_map::FnvIndexMap::<Key, (), 4>::new();
    map.insert(Key { id: 1, label: "stored" }, ()).unwrap();
    assert_eq!(map.entry(Key { id: 1, label: "query" }).key().label, "stored");
}
