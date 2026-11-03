use heapless::*;
use heapless::index_map::{FnvIndexMap, Entry};
use heapless::sorted_linked_list::{SortedLinkedList, Min};
use serde::de::value::{SeqDeserializer, MapDeserializer, Error as VE, StrDeserializer, BytesDeserializer};
use serde::Deserialize;
use std::string::{String as S, ToString};

#[test] fn zst() {
    let mut v: heapless::Vec<(), 6, u8> = heapless::Vec::new();
    for _ in 0..6 { v.push(()).unwrap(); }
    assert!(v.push(()).is_err());
    { let mut d = v.drain(1..3); d.next(); }
    assert_eq!(v.len(), 4);
    v.retain(|_| false);
    assert_eq!(v.len(), 0);
    let mut d: Deque<(), 3> = Deque::new();
    d.push_back(()).unwrap(); d.push_front(()).unwrap(); d.make_contiguous();
    assert_eq!(d.len(), 2);
    let mut h: HistoryBuf<(), 2> = HistoryBuf::new(); h.write(()); h.write(()); h.write(()); assert_eq!(h.len(), 2);
    let mut q: spsc::Queue<(), 3> = spsc::Queue::new(); q.enqueue(()).unwrap(); q.dequeue().unwrap();
    let m: mpmc::Queue<(), 2> = mpmc::Queue::new(); m.enqueue(()).unwrap(); m.dequeue().unwrap();
}
#[test] fn strings_everywhere() {
    let s = |i: u32| i.to_string();
    // Vec
    let mut v: heapless::Vec<S, 6> = heapless::Vec::from_array([s(0), s(1), s(2)]);
    v.insert(1, s(9)).unwrap();
    assert_eq!(v.remove(0), "0");
    assert_eq!(v.swap_remove(0), "9");
    v.extend_from_slice(&[s(5), s(6)]).unwrap();
    { let mut d = v.drain(1..3); assert_eq!(d.next_back().unwrap(), "5"); }
    let view: &mut VecView<S> = &mut v; view.push(s(7)).unwrap(); view.truncate(2);
    let cl = v.clone();
    let mut it = cl.into_iter(); it.next(); let it2 = it.clone(); drop(it); assert_eq!(it2.count(), 1);
    let arr: [S; 2] = v.clone().into_array().unwrap(); drop(arr);
    let w: heapless::Vec<S, 6, u8> = v.clone().cast_len_type(); drop(w);
    let full: heapless::Vec<S, 2> = heapless::Vec::from_array([s(1), s(2)]);
    let av: std::vec::Vec<S> = full.try_into().unwrap();
    let back: heapless::Vec<S, 2> = av.try_into().unwrap(); drop(back);
    let vv: Result<heapless::Vec<S, 1>, _> = std::vec![s(1), s(2)].try_into(); assert!(vv.is_err());
    // spare capacity
    let mut sp: heapless::Vec<S, 3> = heapless::Vec::new();
    sp.spare_capacity_mut()[0].write(s(1)); unsafe { sp.set_len(1) };
    // Deque all branches
    for cap_shift in 0..7 {
        for count in 0..=7usize {
            let mut d: Deque<S, 7> = Deque::new();
            for _ in 0..cap_shift { d.push_back(s(0)).unwrap(); d.pop_front(); }
            for i in 0..count { d.push_back(s(i as u32)).unwrap(); }
            let b = d.clone();
            let _ = b.iter().count();
            assert!(b == d);
            let (a, c) = d.as_mut_slices(); for x in a.iter_mut().chain(c) { x.push('!'); }
            let t: std::vec::Vec<S> = d.into_iter().collect(); assert_eq!(t.len(), count);
        }
    }
    let dv: &mut deque::DequeView<S> = &mut Deque::<S, 3>::new();
    dv.push_back(s(1)).unwrap(); dv.push_front(s(2)).unwrap(); dv.clear();
    // History
    let mut h: HistoryBuf<S, 3> = HistoryBuf::new();
    for i in 0..7 { h.write(s(i)); }
    let c = h.clone(); assert!(c == h);
    let hv: &mut history_buf::HistoryBufView<S> = &mut h; hv.write(s(9)); hv.clear(); hv.write(s(1));
    // Maps
    let mut m = FnvIndexMap::<S, S, 4>::new();
    for i in 0..4 { m.insert(s(i), s(i)).unwrap(); }
    assert!(m.insert(s(10), s(10)).is_err());
    if let Entry::Occupied(o) = m.entry(s(1)) { let (k, _) = o.remove_entry(); assert_eq!(k, "1"); }
    if let Entry::Vacant(o) = m.entry(s(11)) { *o.insert(s(0)).unwrap() = s(3); }
    m.retain(|k, _| k != "2");
    let mc = m.clone();
    assert!(mc == m);
    let _ = mc.into_iter().next();
    let set: index_set::FnvIndexSet<S, 4> = [s(1), s(2), s(1)].into_iter().collect(); assert_eq!(set.len(), 2);
    let mut l: LinearMap<S, S, 3> = LinearMap::new();
    l.insert(s(1), s(1)).unwrap(); l.insert(s(2), s(2)).unwrap(); l.insert(s(2), s(3)).unwrap();
    assert_eq!(l.remove("1").unwrap(), "1");
    let c = l.clone(); let _ = c.into_iter().next();
    // sorted list
    let mut ll: SortedLinkedList<S, Min, 4, u8> = SortedLinkedList::new_u8();
    for i in [3, 1, 2] { ll.push(s(i)).unwrap(); }
    let v = ll.find_mut(|x| x == "2").unwrap().pop(); assert_eq!(v, "2");
    { let mut f = ll.find_mut(|x| x == "1").unwrap(); f.push('z'); }
    drop(ll);
    // heap
    let mut b: binary_heap::BinaryHeap<S, binary_heap::Max, 4> = binary_heap::BinaryHeap::new();
    for i in [3, 1, 2] { b.push(s(i)).unwrap(); }
    b.peek_mut().unwrap().push('0');
    let _ = b.clone().into_vec();
    // spsc mpmc
    let mut q: spsc::Queue<S, 3> = spsc::Queue::new();
    q.enqueue(s(1)).unwrap(); q.enqueue(s(2)).unwrap(); q.dequeue(); q.enqueue(s(3)).unwrap();
    for x in q.iter_mut() { x.push('a'); }
    let c = q.clone(); assert!(c == q);
    assert_eq!(q.iter().rev().next().unwrap(), "3a");
    let mq: mpmc::Queue<S, 2> = mpmc::Queue::new(); mq.enqueue(s(1)).unwrap();
    // string
    let mut st: heapless::String<16> = heapless::String::try_from("héllo").unwrap();
    st.insert(1, '€').unwrap(); st.insert_str(0, "ab").unwrap(); st.truncate(3);
    let c = st.clone(); assert!(c == st);
    let _ = heapless::String::<4>::try_from(-1234i16);
}
#[test] fn serde_limits() {
    type Sq<I> = SeqDeserializer<I, VE>;
    assert_eq!(heapless::Vec::<u8, 2>::deserialize(Sq::new([1u8, 2].into_iter())).unwrap(), [1, 2]);
    assert!(heapless::Vec::<u8, 2>::deserialize(Sq::new([1u8, 2, 3].into_iter())).is_err());
    assert!(Deque::<u8, 2>::deserialize(Sq::new([1u8, 2, 3].into_iter())).is_err());
    assert!(binary_heap::BinaryHeap::<u8, binary_heap::Min, 2>::deserialize(Sq::new([1u8, 2, 3].into_iter())).is_err());
    assert!(index_set::FnvIndexSet::<u8, 2>::deserialize(Sq::new([1u8, 2, 3].into_iter())).is_err());
    assert_eq!(index_set::FnvIndexSet::<u8, 2>::deserialize(Sq::new([1u8, 1, 1].into_iter())).unwrap().len(), 1);
    let h = HistoryBuf::<u8, 2>::deserialize(Sq::new([1u8, 2, 3].into_iter())).unwrap(); println!("hist from 3 into cap 2: {:?}", h.oldest_ordered().collect::<std::vec::Vec<_>>());
    let m = LinearMap::<u8, u8, 2>::deserialize(MapDeserializer::<_, VE>::new([(1u8, 1u8), (1, 2)].into_iter())).unwrap(); assert_eq!(m.len(), 1);
    assert!(index_map::FnvIndexMap::<u8, u8, 2>::deserialize(MapDeserializer::<_, VE>::new([(1u8, 1u8), (2, 2), (3, 3)].into_iter())).is_err());
    assert!(heapless::String::<2>::deserialize(StrDeserializer::<VE>::new("abc")).is_err());
    assert_eq!(heapless::String::<3>::deserialize(StrDeserializer::<VE>::new("abc")).unwrap(), "abc");
    println!("string from bytes hint: {:?}", heapless::String::<3>::deserialize(BytesDeserializer::<VE>::new(b"ab")).map(|s| s.len()));
    println!("string from bad bytes: {:?}", heapless::String::<3>::deserialize(BytesDeserializer::<VE>::new(&[0xff])).is_err());
}
