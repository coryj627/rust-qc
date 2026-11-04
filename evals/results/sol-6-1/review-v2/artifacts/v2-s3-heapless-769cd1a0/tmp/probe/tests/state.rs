use heapless::{Deque, LinearMap};
use heapless::sorted_linked_list::{Max, SortedLinkedList};

#[test]
fn linear_map_collect_replaces_duplicate_keys() {
    let m: LinearMap<_, _, 4> = [("a", 1), ("a", 2)].into_iter().collect();
    assert_eq!(m.len(), 1);
    assert_eq!(m.get("a"), Some(&2));
}

#[test]
fn deque_can_push_after_full_make_contiguous() {
    let mut q = Deque::<_, 4>::new();
    q.extend([0, 1, 2, 3]);
    assert_eq!(q.pop_front(), Some(0));
    assert_eq!(q.pop_front(), Some(1));
    q.extend([4, 5]);
    assert_eq!(q.make_contiguous(), &[2, 3, 4, 5]);
    assert_eq!(q.pop_front(), Some(2));
    q.push_back(6).unwrap();
    assert_eq!(q.iter().copied().collect::<std::vec::Vec<_>>(), [3, 4, 5, 6]);
}

#[test]
fn sorted_list_pop_mutated_entry() {
    let mut q = SortedLinkedList::<_, Max, 2, u8>::new_u8();
    q.push(1).unwrap();
    let mut found = q.find_mut(|_| true).unwrap();
    *found = 2;
    assert_eq!(found.pop(), 2);
    assert_eq!(q.pop(), None);
}

#[test]
fn sorted_list_drops_owned_elements() {
    let dropped = std::rc::Rc::new(std::cell::Cell::new(0));
    #[derive(Eq, PartialEq, Ord, PartialOrd)]
    struct Item(std::rc::Rc<std::cell::Cell<usize>>);
    impl Drop for Item { fn drop(&mut self) { self.0.set(self.0.get() + 1); } }
    let mut q = SortedLinkedList::<_, Max, 2, u8>::new_u8();
    assert!(q.push(Item(dropped.clone())).is_ok());
    drop(q);
    assert_eq!(dropped.get(), 1);
}

#[test]
fn index_map_truncate_keeps_earlier_keys_findable() {
    use heapless::index_map::FnvIndexMap;
    for a in 0..8 {
        for b in 0..8 {
            for c in 0..8 {
                if a == b || b == c || a == c { continue; }
                let mut m = FnvIndexMap::<_, _, 4>::new();
                m.insert(a, 10).unwrap();
                m.insert(b, 20).unwrap();
                m.insert(c, 30).unwrap();
                m.truncate(2);
                assert_eq!(m.get(&a), Some(&10), "keys {a}, {b}, {c}");
                assert_eq!(m.get(&b), Some(&20), "keys {a}, {b}, {c}");
                assert_eq!(m.get(&c), None, "keys {a}, {b}, {c}");
            }
        }
    }
}
