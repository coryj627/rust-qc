use heapless::*;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};

// ---- deque
fn wrapped_12() -> Deque<u32, 12> {
    let mut d = Deque::<u32, 12>::new();
    for _ in 0..7 { d.push_back(0).unwrap(); d.pop_front().unwrap(); }
    for i in 1..=8 { d.push_back(i).unwrap(); }
    d
}
#[test]
fn deque_mc_len_wrong() {
    let mut d = wrapped_12();
    assert_eq!(d.len(), 8);
    let s: std::vec::Vec<u32> = d.make_contiguous().to_vec();
    assert_eq!(s, [1, 2, 3, 4, 5, 6, 7, 8]);
    println!("len after make_contiguous = {}", d.len());
    assert_eq!(d.len(), 8);
}
#[test]
fn deque_mc_uninit_read() {
    let mut d = wrapped_12();
    d.make_contiguous();
    let v = d.pop_back();
    println!("pop_back = {:?}", v);
    assert_eq!(v, Some(8));
}
#[test]
fn deque_mc_full_oob() {
    let mut d = Deque::<u32, 4>::new();
    for i in 0..4 { d.push_back(i).unwrap(); }
    d.pop_front(); d.pop_front();
    d.push_back(4).unwrap(); d.push_back(5).unwrap();
    assert!(d.is_full());
    assert_eq!(d.make_contiguous(), &[2, 3, 4, 5]);
    d.pop_front();
    println!("pushing into slot past the buffer");
    d.push_back(6).unwrap();
    println!("{:?}", d);
}

// ---- panic safety
static PANICKED: AtomicBool = AtomicBool::new(false);
struct P(Box<u32>);
impl Drop for P {
    fn drop(&mut self) {
        if *self.0 == 1 && !PANICKED.swap(true, Ordering::SeqCst) { panic!("boom"); }
    }
}
#[test]
fn deque_clear_panic() {
    let mut d = Deque::<P, 4>::new();
    for i in 0..3 { let _ = d.push_back(P(Box::new(i))); }
    let _ = catch_unwind(AssertUnwindSafe(|| d.clear()));
    println!("len after clear panic {}", d.storage_len());
    drop(d);
}
#[test]
fn histbuf_clear_panic() {
    let mut d = HistoryBuf::<P, 4>::new();
    for i in 0..3 { d.write(P(Box::new(i))); }
    let _ = catch_unwind(AssertUnwindSafe(|| d.clear()));
    drop(d);
}
#[test]
fn histbuf_write_panic() {
    let mut d = HistoryBuf::<P, 2>::new();
    for i in 1..3 { d.write(P(Box::new(i))); }
    let _ = catch_unwind(AssertUnwindSafe(|| d.write(P(Box::new(7)))));
    drop(d);
}
#[test]
fn vec_intoiter_panic() {
    let mut v = Vec::<P, 4>::new();
    for i in 0..3 { let _ = v.push(P(Box::new(i))); }
    let mut it = v.into_iter();
    let first = it.next();
    let _ = catch_unwind(AssertUnwindSafe(|| drop(it)));
    drop(first);
}

// ---- index map
#[derive(Default)]
struct H(u64);
impl core::hash::Hasher for H {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = (self.0 << 8) | *x as u64; } }
    fn write_u32(&mut self, x: u32) { self.0 = (x >> 8) as u64; }
}
#[derive(Default, Clone)]
struct B;
impl core::hash::BuildHasher for B { type Hasher = H; fn build_hasher(&self) -> H { H(0) } }

#[test]
fn indexmap_truncate_breaks_chain() {
    // key >> 8 is the hash
    let (x, y, z, w) = (0x100u32 | 1, 0x100 | 2, 0, 1);
    let mut m: IndexMap<u32, u32, B, 8> = IndexMap::default();
    m.insert(x, 10).unwrap();
    m.insert(y, 11).unwrap();
    m.insert(z, 12).unwrap();
    m.insert(w, 13).unwrap();
    assert_eq!(m.get(&x), Some(&10));
    m.truncate(2);
    assert_eq!(m.len(), 2);
    println!("get(x)={:?} get(y)={:?}", m.get(&x), m.get(&y));
    println!("insert(x) again -> {:?}", m.insert(x, 99));
    println!("keys: {:?} len {}", m.keys().collect::<std::vec::Vec<_>>(), m.len());
    assert_eq!(m.len(), 2);
}
#[derive(Hash, PartialEq, Eq, Debug)]
struct K(Box<u32>);
#[test]
fn indexmap_retain_panic_stale() {
    let mut m: index_map::FnvIndexMap<K, u32, 8> = index_map::FnvIndexMap::new();
    for i in 0..6 { m.insert(K(Box::new(i)), i).unwrap(); }
    let _ = catch_unwind(AssertUnwindSafe(|| m.retain(|k, _| {
        if *k.0 == 3 { panic!("pred") }
        *k.0 % 2 == 0
    })));
    println!("len {}", m.len());
    // entry for key 1 was deleted; its index slot still points past / at a moved entry
    for i in 0..6 {
        println!("get({i}) = {:?}", m.get(&K(Box::new(i))));
    }
}
#[test]
fn indexmap_into_iter_order() {
    let mut m = index_map::FnvIndexMap::<_, _, 8>::new();
    for i in 0..4 { m.insert(i, i).unwrap(); }
    let a: std::vec::Vec<_> = m.iter().map(|(k, _)| *k).collect();
    let b: std::vec::Vec<_> = m.into_iter().map(|(k, _)| k).collect();
    println!("iter {:?} into_iter {:?}", a, b);
    assert_eq!(a, b);
}
#[derive(Default)]
struct H2(u64);
impl core::hash::Hasher for H2 {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, _: &[u8]) {}
    fn write_u32(&mut self, x: u32) { self.0 = x as u64; }
}
#[derive(Default)]
struct B2;
impl core::hash::BuildHasher for B2 { type Hasher = H2; fn build_hasher(&self) -> H2 { H2(0) } }
fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap();
}
#[test]
fn indexmap_65536_niche() {
    big(|| {
        let mut m: Box<IndexMap<u32, u32, B2, 65536>> = Box::new(IndexMap::default());
        for k in 0..=65535u32 { m.insert(k, k).unwrap(); }
        println!("len {}", m.len());
        println!("get(65535) = {:?}", m.get(&65535));
        assert_eq!(m.get(&65535), Some(&65535));
    });
}
#[test]
fn indexmap_131072() {
    big(|| {
        let mut m: Box<IndexMap<u32, u32, B2, 131072>> = Box::new(IndexMap::default());
        for k in 0..70000u32 { m.insert(k, k).unwrap(); }
        println!("len {}", m.len());
        let missing = (0..70000u32).filter(|k| m.get(k) != Some(k)).count();
        println!("lookups not returning inserted value: {missing}");
        assert_eq!(missing, 0);
    });
}

// ---- linear map
#[test]
fn linearmap_from_iter_dupes() {
    let m: LinearMap<u32, u32, 4> = [(1, 10), (1, 20)].into_iter().collect();
    println!("len {} get {:?}", m.len(), m.get(&1));
    assert_eq!(m.len(), 1);
    assert_eq!(m.get(&1), Some(&20));
}
#[test]
fn linearmap_eq_bound() {
    fn is_eq<T: Eq>(_: &T) {}
    let mut m: LinearMap<u32, f32, 4> = LinearMap::new();
    m.insert(1, f32::NAN).unwrap();
    is_eq(&m);
    assert!(m != m, "x");
}

// ---- bytes
#[test]
fn bufmut_fill_to_capacity() {
    use bytes::BufMut;
    let mut v: Vec<u8, 4> = Vec::new();
    v.put_slice(&[1, 2, 3, 4]);
    assert_eq!(v, [1, 2, 3, 4]);
}
#[test]
fn bufmut_put_u8_last() {
    use bytes::BufMut;
    let mut v: Vec<u8, 2> = Vec::new();
    v.put_u8(1);
    assert_eq!(v.remaining_mut(), 1);
    v.put_u8(2);
    assert_eq!(v, [1, 2]);
}

// ---- cstring
#[test]
fn cstring_with_nul_no_nul() {
    let r = CString::<8>::from_bytes_with_nul(b"abc");
    println!("{:?}", r);
    assert!(r.is_err());
}
#[test]
fn cstring_with_nul_empty() {
    let r = CString::<8>::from_bytes_with_nul(b"");
    println!("{:?}", r);
    assert!(r.is_err());
}

// ---- sorted linked list
use heapless::sorted_linked_list::{Max, SortedLinkedList};
#[test]
fn sll_mut_then_pop_nonhead() {
    let mut ll: SortedLinkedList<Box<u32>, Max, 4, u8> = SortedLinkedList::new_u8();
    for i in 1..=3 { ll.push(Box::new(i)).unwrap(); }
    let mut f = ll.find_mut(|v| **v == 2).unwrap();
    **f = 2;
    let popped = f.pop();
    assert_eq!(*popped, 2);
    println!("list after pop {:?}", ll);
    drop(popped);
    assert_eq!(ll.iter().map(|b| **b).collect::<std::vec::Vec<_>>(), [3, 1]);
}
#[test]
fn sll_mut_then_pop_head_single() {
    let mut ll: SortedLinkedList<Box<u32>, Max, 4, u8> = SortedLinkedList::new_u8();
    ll.push(Box::new(1)).unwrap();
    let mut f = ll.find_mut(|_| true).unwrap();
    **f = 5;
    let popped = f.pop();
    assert_eq!(*popped, 5);
    println!("empty? {}", ll.is_empty());
}
#[test]
fn sll_mut_then_pop_head_multi() {
    let mut ll: SortedLinkedList<u32, Max, 4, u8> = SortedLinkedList::new_u8();
    for i in 1..=3 { ll.push(i).unwrap(); }
    let mut f = ll.find_mut(|v| *v == 3).unwrap();
    *f = 9;
    assert_eq!(f.pop(), 9);
    println!("{:?}", ll);
}

// ---- spsc peek race
#[test]
fn spsc_peek_race() {
    let mut q: spsc::Queue<u64, 4> = spsc::Queue::new();
    let (mut p, c) = q.split();
    std::thread::scope(|s| {
        s.spawn(move || { p.enqueue(42).unwrap(); });
        s.spawn(move || {
            loop {
                if let Some(v) = c.peek() { assert_eq!(*v, 42); break; }
                std::thread::yield_now();
            }
        });
    });
}

// ---- history buf
#[test]
fn histbuf_new_with_zero() {
    let x = HistoryBuf::<u8, 0>::new_with(1);
    println!("len {}", x.len());
    let r = catch_unwind(AssertUnwindSafe(|| x.recent().copied()));
    println!("recent -> {:?}", r.is_err());
    assert!(r.is_ok());
}

#[test]
fn indexmap_retain_uaf() {
    let mut m: index_map::FnvIndexMap<K, u32, 8> = index_map::FnvIndexMap::new();
    for i in 0..6 { m.insert(K(Box::new(i)), i).unwrap(); }
    let _ = catch_unwind(AssertUnwindSafe(|| m.retain(|k, _| {
        if *k.0 == 5 { panic!("pred") }
        *k.0 < 3
    })));
    println!("len {}", m.len());
    // key 4 was dropped; indices still map its hash to entries[4]
    println!("get(4) = {:?}", m.get(&K(Box::new(4))));
}

#[test]
fn deque_mc_box_double_free() {
    let mut d = Deque::<Box<u32>, 12>::new();
    for _ in 0..7 { d.push_back(Box::new(0)).unwrap(); d.pop_front().unwrap(); }
    for i in 1..=8 { d.push_back(Box::new(i)).unwrap(); }
    d.make_contiguous();
    println!("len {}", d.storage_len());
    drop(d);
}
#[test]
fn deque_mc_uninit_slot() {
    // free > back_len where the stale slot was never written
    let mut d = Deque::<u64, 16>::new();
    for _ in 0..9 { d.push_back(0).unwrap(); d.pop_front().unwrap(); }
    for i in 1..=9 { d.push_back(i).unwrap(); }
    println!("before {:?}", d.as_slices());
    d.make_contiguous();
    println!("after {:?}", d.as_slices());
}
#[test]
fn vec_retain_panic_sb() {
    let mut v = Vec::<Box<u32>, 4>::new();
    for i in 0..4 { let _ = v.push(Box::new(i)); }
    let _ = catch_unwind(AssertUnwindSafe(|| v.retain(|x| {
        if **x == 2 { panic!("pred") }
        **x != 1
    })));
    assert_eq!(v.len(), 3);
}
