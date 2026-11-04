use heapless::{Vec, Deque, HistoryBuf, CString, LinearMap};
use heapless::index_map::{FnvIndexMap, Entry};
use std::{cell::Cell, rc::Rc, panic::{catch_unwind, AssertUnwindSafe}};

#[test]
fn deque_full_contiguous_drain() {
    let mut q = Deque::<u8,4>::new();
    for i in 1..=4 { q.push_back(i).unwrap(); }
    assert_eq!(q.pop_front(),Some(1)); assert_eq!(q.pop_front(),Some(2));
    q.push_back(5).unwrap(); q.push_back(6).unwrap();
    assert_eq!(q.make_contiguous(), &[3,4,5,6]);
    for i in 3..=6 { assert_eq!(q.pop_front(),Some(i)); }
    assert_eq!((q.len(),q.is_empty()), (0,true));
}
#[test]
fn sorted_modified_pop() {
    use heapless::sorted_linked_list::{SortedLinkedList, Max};
    let mut q: SortedLinkedList<i32,Max,3,u8> = SortedLinkedList::new_u8();
    for i in [3,2,1] {q.push(i).unwrap();}
    let mut entry = q.find_mut(|x| *x==2).unwrap();
    *entry=20;
    assert_eq!(entry.pop(),20);
    assert_eq!(q.iter().copied().collect::<std::vec::Vec<_>>(), [3,1]);
}
#[test]
fn bytes_full_owned() {
    use bytes::BufMut;
    let mut v = Vec::<u8,4>::new();
    assert_eq!(v.remaining_mut(),4);
    v.put_slice(b"abcd");
    assert_eq!(&v[..], b"abcd");
}
#[test]
fn bytes_full_view() {
    use bytes::BufMut;
    let mut v = Vec::<u8,1>::new();
    v.as_mut_view().put_u8(7);
    assert_eq!(&v[..], &[7]);
}
#[test]
fn index_truncate_keeps_keys() {
    let mut m=FnvIndexMap::<u32,u32,4>::new();
    for k in [0,1,5] {m.insert(k,k).unwrap();}
    m.truncate(2);
    assert_eq!(m.iter().map(|(k,_)| *k).collect::<std::vec::Vec<_>>(), [0,1]);
    assert_eq!(m.get(&0),Some(&0));
    assert_eq!(m.get(&1),Some(&1));
}
#[test]
fn linear_collect_unique() {
    let m: LinearMap<u8,u8,2> = [(1,10),(1,20)].into_iter().collect();
    assert_eq!((m.len(),m.get(&1)),(1,Some(&20)));
}
#[test]
fn linear_collect_duplicate_capacity() {
    let m: LinearMap<u8,u8,1> = [(1,10),(1,20)].into_iter().collect();
    assert_eq!(m.get(&1),Some(&20));
}
#[test]
fn resize_defaults_independent() {
    let mut v=Vec::<Rc<Cell<u8>>,3>::new();
    v.resize_default(3).unwrap();
    v[0].set(9);
    assert_eq!(v[1].get(),0);
}
#[test]
fn resize_shrink_does_not_default() {
    #[derive(Clone)] struct BadDefault;
    impl Default for BadDefault {fn default()->Self {panic!("default called");}}
    let mut v=Vec::<BadDefault,2>::new();
    v.push(BadDefault).ok();
    v.resize_default(0).unwrap();
    assert_eq!(v.len(),0);
}
#[test]
fn cstring_requires_nul() {
    assert!(CString::<8>::from_bytes_with_nul(b"abc").is_err());
}
#[test]
fn cstring_requires_nul_empty() {
    assert!(CString::<8>::from_bytes_with_nul(b"").is_err());
}
#[test]
fn owning_index_order() {
    let mut m=FnvIndexMap::<u8,u8,4>::new();
    for i in 1..=3 {m.insert(i,i).unwrap();}
    assert_eq!(m.into_iter().map(|(k,_)| k).collect::<std::vec::Vec<_>>(),[1,2,3]);
}
#[test]
fn retain_unwind_indices() {
    let mut m=FnvIndexMap::<u32,u32,4>::new();
    for i in 0..3 {m.insert(i,i).unwrap();}
    assert!(catch_unwind(AssertUnwindSafe(|| m.retain(|k,_| match *k {0=>false,1=>panic!("predicate"),_=>true}))).is_err());
    assert_eq!(m.iter().map(|(k,_)| *k).collect::<std::vec::Vec<_>>(), [1,2]);
    assert_eq!(m.get(&1),Some(&1));
    assert_eq!(m.get(&2),Some(&2));
}
#[test]
fn linear_eq_is_reflexive() {
    fn requires_eq<T: Eq>(_: &T) {}
    let mut m=LinearMap::<u8,f32,1>::new();
    m.insert(0,f32::NAN).unwrap();
    requires_eq(&m);
    assert!(m==m);
}
#[test]
fn occupied_stored_key() {
    #[derive(Debug)] struct Key(u8,&'static str);
    impl PartialEq for Key {fn eq(&self,other:&Self)->bool {self.0==other.0}}
    impl Eq for Key {}
    impl std::hash::Hash for Key {fn hash<H:std::hash::Hasher>(&self,h:&mut H) {self.0.hash(h)}}
    let mut m=FnvIndexMap::<Key,u8,4>::new();
    m.insert(Key(1,"stored"),42).unwrap();
    match m.entry(Key(1,"query")) {
        Entry::Occupied(e)=> assert_eq!(e.key().1,"stored"),
        _=> panic!("missing entry"),
    }
}
#[test]
fn history_zero_constructor() {
    let h=HistoryBuf::<u8,0>::new_with(42);
    assert_eq!(h.oldest(),None);
}
#[test]
fn history_recent_wrap_control() {
    let mut h=HistoryBuf::<u8,4>::new();
    assert_eq!(h.recent(),None);
    for i in 1..=12 {h.write(i); assert_eq!(h.recent(),Some(&i));}
}
#[test]
fn cstring_exact_append_control() {
    let mut c=CString::<5>::from_bytes_with_nul(b"ab\0").unwrap();
    c.extend_from_bytes(b"cd\0").unwrap();
    assert_eq!(c.as_c_str().to_bytes_with_nul(),b"abcd\0");
}
#[test]
fn heap_mutation_control() {
    use heapless::binary_heap::{BinaryHeap,Max};
    let mut h=BinaryHeap::<u8,Max,4>::new();
    for i in [9,7,8,1] {h.push(i).unwrap();}
    {*h.peek_mut().unwrap()=0;}
    assert_eq!(h.peek(),Some(&8));
    assert_eq!(std::iter::from_fn(||h.pop()).collect::<std::vec::Vec<_>>(),[8,7,1,0]);
}
#[test]
fn string_long_truncate_control() {
    let mut s=heapless::String::<8>::try_from("aéz").unwrap();
    s.truncate(5);
    assert_eq!(&s[..],"aéz");
}
#[test]
fn drain_partial_control() {
    let mut v=Vec::<_,5>::from_array([10,20,30,40,50]);
    {let mut d=v.drain(1..4); assert_eq!(d.next(),Some(20));}
    assert_eq!(&v[..], &[10,50]);
}
