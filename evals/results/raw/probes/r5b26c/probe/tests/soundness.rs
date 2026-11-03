use heapless::{Deque, HistoryBuf, Vec, IndexMap, LinearMap};
use heapless::sorted_linked_list::{SortedLinkedList, Max};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::hash::{BuildHasherDefault, Hasher};

// ---------- deque make_contiguous
fn setup(n_front_rot: usize) -> Deque<Box<u32>, 12> {
    let mut d = Deque::new();
    for _ in 0..n_front_rot { d.push_back(Box::new(0)).unwrap(); d.pop_front().unwrap(); }
    d
}
#[test]
fn deque_contig_middle_branch_len() {
    // FGH....ABCDE: front=7, back=3, len=8, free=4, front_len=5, back_len=3
    let mut d = setup(7);
    for i in 0..8 { d.push_back(Box::new(i)).unwrap(); }
    assert_eq!(d.len(), 8);
    {
        let s = d.make_contiguous();
        assert_eq!(s.len(), 8);
        let v: std::vec::Vec<u32> = s.iter().map(|b| **b).collect();
        assert_eq!(v, (0..8).collect::<std::vec::Vec<_>>());
    }
    eprintln!("len after make_contiguous = {}", d.len());
    assert_eq!(d.len(), 8, "len changed by make_contiguous");
    let v: std::vec::Vec<u32> = d.iter().map(|b| **b).collect();
    assert_eq!(v, (0..8).collect::<std::vec::Vec<_>>());
}
#[test]
fn deque_contig_full_then_push_oob() {
    let mut d: Deque<Box<u32>, 4> = Deque::new();
    for _ in 0..2 { d.push_back(Box::new(0)).unwrap(); d.pop_front().unwrap(); }
    for i in 0..4 { d.push_back(Box::new(i)).unwrap(); }
    assert!(d.is_full());
    let _ = d.make_contiguous();
    assert_eq!(d.pop_front().map(|b| *b), Some(0));
    d.push_back(Box::new(99)).unwrap();
    let v: std::vec::Vec<u32> = d.iter().map(|b| **b).collect();
    assert_eq!(v, [1,2,3,99]);
}

// ---------- panicking drops
static BOOM: AtomicUsize = AtomicUsize::new(0);
struct P(#[allow(dead_code)] Box<u32>, bool);
impl Drop for P { fn drop(&mut self) { BOOM.fetch_add(1, SeqCst); if self.1 { self.1 = false; panic!("boom"); } } }
fn p(b: bool) -> P { P(Box::new(1), b) }

#[test]
fn deque_clear_panic() {
    let mut d: Deque<P, 4> = Deque::new();
    d.push_back(p(true)).ok().unwrap();
    d.push_back(p(false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| d.clear()));
    drop(d);
}
#[test]
fn vec_into_iter_panic() {
    let mut v: Vec<P, 4> = Vec::new();
    v.push(p(true)).ok().unwrap();
    v.push(p(false)).ok().unwrap();
    let it = v.into_iter();
    let _ = catch_unwind(AssertUnwindSafe(|| drop(it)));
}
#[test]
fn history_write_panic() {
    let mut h: HistoryBuf<P, 1> = HistoryBuf::new();
    h.write(p(true));
    let _ = catch_unwind(AssertUnwindSafe(|| h.write(p(false))));
    drop(h);
}
#[test]
fn history_clear_panic() {
    let mut h: HistoryBuf<P, 2> = HistoryBuf::new();
    h.write(p(true));
    h.write(p(false));
    let _ = catch_unwind(AssertUnwindSafe(|| h.clear()));
    drop(h);
}

// ---------- IndexMap with identity hash
#[derive(Default)]
struct Id(u64);
impl Hasher for Id {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = (self.0 << 8) | *x as u64; } }
    fn write_u32(&mut self, v: u32) { self.0 = v as u64; }
}
type M<V, const N: usize> = IndexMap<u32, V, BuildHasherDefault<Id>, N>;

#[test]
fn indexmap_retain_panic_stale() {
    let mut m: M<Box<u32>, 8> = Default::default();
    for k in 0..4u32 { m.insert(k, Box::new(k)).unwrap(); }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let mut i = 0;
        m.retain(|_, _| { i += 1; if i == 1 { false } else if i == 2 { panic!("x") } else { true } });
    }));
    eprintln!("len={} ", m.len());
    // key 3 was at entry index 3, now out of len
    let v = m.remove(&3);
    eprintln!("removed {:?}", v);
    drop(v);
    drop(m);
}
#[test]
fn indexmap_truncate_loses_keys() {
    // X desired 1, Y desired 0, Z desired 0 => Z steals X's slot
    let mut m: M<u32, 8> = Default::default();
    m.insert(1, 100).unwrap(); // X, entry 0, slot 1
    m.insert(8, 200).unwrap(); // Y, entry 1, slot 0
    m.insert(16, 300).unwrap(); // Z, entry 2, slot 1, X moves to 2
    assert_eq!(m.get(&1), Some(&100));
    m.truncate(2);
    assert_eq!(m.len(), 2);
    eprintln!("after truncate get(1) = {:?}", m.get(&1));
    let in_iter: std::vec::Vec<u32> = m.keys().copied().collect();
    assert!(in_iter.contains(&1));
    assert_eq!(m.get(&1), Some(&100), "key in keys() but get() is None");
}
#[test]
fn indexmap_into_iter_reversed() {
    let mut m: M<u32, 8> = Default::default();
    for k in [1u32,2,3] { m.insert(k, k).unwrap(); }
    let a: std::vec::Vec<u32> = m.iter().map(|(k,_)| *k).collect();
    let b: std::vec::Vec<u32> = m.into_iter().map(|(k,_)| k).collect();
    assert_eq!(a, b, "iter and into_iter order differ");
}
#[test]
fn linear_map_from_iter_dupes() {
    let m: LinearMap<u32, char, 4> = [(1,'a'),(1,'b')].into_iter().collect();
    eprintln!("len={} get={:?}", m.len(), m.get(&1));
    assert_eq!(m.len(), 1);
    assert_eq!(m.get(&1), Some(&'b'));
}
#[test]
fn bufmut_fill_exactly() {
    use bytes::BufMut;
    let mut v: Vec<u8, 8> = Vec::new();
    assert_eq!(v.remaining_mut(), 8);
    v.put_slice(&[1,2,3,4,5,6,7,8]);
    assert_eq!(v.len(), 8);
}
#[test]
fn sorted_list_find_mut_modify_pop() {
    let mut ll: SortedLinkedList<Box<u32>, Max, 4> = SortedLinkedList::new_usize();
    ll.push(Box::new(3)).unwrap();
    ll.push(Box::new(2)).unwrap();
    ll.push(Box::new(1)).unwrap();
    let mut f = ll.find_mut(|x| **x == 2).unwrap();
    **f = 20;
    let v = f.pop();
    assert_eq!(*v, 20);
    let rest: std::vec::Vec<u32> = ll.iter().take(10).map(|b| **b).collect();
    eprintln!("rest = {rest:?}");
    assert_eq!(rest, [3, 1]);
}
#[test]
fn cstring_doc() {
    let r = heapless::CString::<8>::from_bytes_with_nul(b"abc");
    eprintln!("{:?}", r);
    assert!(r.is_err(), "doc says Fails if not nul-terminated");
}
#[test]
fn history_new_with_zero() {
    let h: HistoryBuf<u8, 0> = HistoryBuf::new_with(1);
    eprintln!("{:?} {:?}", h.len(), h.is_full());
    let _ = h.recent();
}
#[test]
fn string_drain_miri() {
    let mut s = heapless::String::<16>::try_from("héllo wörld").unwrap();
    let d: std::string::String = s.drain(1..3).collect();
    assert_eq!(d, "é");
    assert_eq!(s, "hllo wörld");
    s.insert(1, 'é').unwrap();
    s.insert_str(0, "x").unwrap();
    assert_eq!(s, "xhéllo wörld");
    assert_eq!(s.remove(2), 'é');
}
#[test]
fn deque_overflow_canary() {
    #[repr(C)] struct S { d: Deque<u64, 4>, canary: [u64; 2] }
    let mut s = S { d: Deque::new(), canary: [0x1111, 0x1111] };
    for _ in 0..2 { s.d.push_back(0).unwrap(); s.d.pop_front().unwrap(); }
    for i in 0..4 { s.d.push_back(i).unwrap(); }
    let _ = s.d.make_contiguous();
    s.d.pop_front();
    s.d.push_back(0xDEAD_BEEF).unwrap();
    let c = unsafe { std::ptr::read_volatile(&s.canary) };
    eprintln!("canary = {:#x?}", c);
    assert_eq!(c, [0x1111, 0x1111]);
}
#[test]
fn indexmap_clear_panic_stale() {
    let mut m: M<P, 8> = Default::default();
    m.insert(0, p(true)).ok().unwrap();
    m.insert(1, p(false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| m.clear()));
    eprintln!("len={}", m.len());
    assert!(m.get(&1).is_none());
    drop(m);
}
#[test]
fn indexmap_truncate_panic_stale() {
    let mut m: M<P, 8> = Default::default();
    m.insert(0, p(false)).ok().unwrap();
    m.insert(1, p(true)).ok().unwrap();
    m.insert(2, p(false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| m.truncate(1)));
    eprintln!("len={}", m.len());
    assert!(m.get(&2).is_none());
}
