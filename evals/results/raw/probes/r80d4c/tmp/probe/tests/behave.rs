use bytes::BufMut;
use heapless::*;
use heapless::index_map::FnvIndexMap;
use heapless::sorted_linked_list::{SortedLinkedList, Max};

#[test] fn bufmut_fill_exactly() {
    let mut v: Vec<u8, 8> = Vec::new();
    assert_eq!(v.remaining_mut(), 8);
    v.put_slice(&[1u8; 8]);
    assert_eq!(v.len(), 8);
}
#[test] fn bufmut_put_u8_last() {
    let mut v: Vec<u8, 1> = Vec::new();
    v.put_u8(7);
    assert_eq!(&v[..], &[7]);
}
#[test] fn cstring_requires_nul() {
    let r = CString::<8>::from_bytes_with_nul(b"abc");
    assert!(r.is_err(), "doc says fails without trailing nul, got {:?}", r);
}
#[test] fn cstring_empty_requires_nul() {
    let r = CString::<8>::from_bytes_with_nul(b"");
    assert!(r.is_err(), "doc says fails without trailing nul, got {:?}", r);
}
#[test] fn linearmap_from_iter_dups() {
    let m: LinearMap<i32, &str, 4> = [(1, "a"), (1, "b")].into_iter().collect();
    assert_eq!(m.len(), 1, "dup keys kept: {:?}", m);
    let mut m = m;
    m.remove(&1);
    assert_eq!(m.get(&1), None);
}
#[test] fn indexmap_into_iter_order() {
    let mut m = FnvIndexMap::<_, _, 8>::new();
    m.insert(1, "a").unwrap(); m.insert(2, "b").unwrap(); m.insert(3, "c").unwrap();
    let i: std::vec::Vec<_> = m.iter().map(|(k, _)| *k).collect();
    let o: std::vec::Vec<_> = m.into_iter().map(|(k, _)| k).collect();
    assert_eq!(i, o);
}
// identity hasher
#[derive(Default, Clone)] struct Id(u64);
impl core::hash::Hasher for Id {
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = self.0 << 8 | *x as u64 } }
    fn write_u32(&mut self, x: u32) { self.0 = x as u64 }
    fn finish(&self) -> u64 { self.0 }
}
#[derive(Default, Clone)] struct IdB;
impl core::hash::BuildHasher for IdB { type Hasher = Id; fn build_hasher(&self) -> Id { Id(0) } }
#[test] fn indexmap_truncate_keeps_lookup() {
    let mut m: IndexMap<u32, u32, IdB, 8> = IndexMap::default();
    // x1 wants slot 1; x2 and x3 want slot 0 (keys 8 and 16 hash to slot 0 under mask 7)
    m.insert(1, 100).unwrap();
    m.insert(8, 200).unwrap();
    m.insert(16, 300).unwrap();
    m.truncate(1);
    assert_eq!(m.len(), 1);
    assert_eq!(m.iter().next(), Some((&1, &100)));
    assert_eq!(m.get(&1), Some(&100), "key kept by truncate cannot be found");
}
#[test] fn indexmap_large_capacity() {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let mut m: std::boxed::Box<FnvIndexMap<u32, u32, 131072>> = std::boxed::Box::new(FnvIndexMap::new());
        for i in 0..70000u32 { m.insert(i, i).unwrap(); }
        assert_eq!(m.len(), 70000);
        for i in 0..70000u32 { assert_eq!(m.get(&i), Some(&i), "lost key {i}"); }
    }).unwrap().join().unwrap();
}
#[test] fn deque_make_contig_len() {
    // N=12, front=7, back=3, len=8
    let mut d: Deque<u32, 12> = Deque::new();
    for i in 0..12 { d.push_back(i).unwrap(); }
    for _ in 0..7 { d.pop_front(); }
    for i in 12..15 { d.push_back(i).unwrap(); }
    assert_eq!(d.len(), 8);
    let want: std::vec::Vec<u32> = d.iter().copied().collect();
    let got = d.make_contiguous().to_vec();
    assert_eq!(got, want);
    assert_eq!(d.len(), 8, "len changed by make_contiguous");
}
#[test] fn deque_diff() {
    let mut seed = 0x1234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..2000 {
        let mut d: Deque<u32, 7> = Deque::new();
        let mut m = std::collections::VecDeque::new();
        for s in 0..40u32 {
            match r() % 5 {
                0 | 1 => { let ok = d.push_back(s).is_ok(); if ok { m.push_back(s); } assert_eq!(ok, m.len() <= 7 && (ok)); }
                2 => { if d.push_front(s).is_ok() { m.push_front(s); } }
                3 => { assert_eq!(d.pop_front(), m.pop_front()); }
                _ => {
                    if r() % 2 == 0 {
                        let a = d.make_contiguous().to_vec();
                        assert_eq!(a, m.make_contiguous().to_vec());
                        assert_eq!(d.len(), m.len(), "len");
                        assert_eq!(d.storage_len(), m.len());
                    } else { assert_eq!(d.pop_back(), m.pop_back()); }
                }
            }
            assert!(d.len() <= 7);
            assert_eq!(d.len(), m.len());
        }
    }
}
#[test] fn sorted_list_diff() {
    let mut seed = 0x9234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..3000 {
        let mut l: SortedLinkedList<u32, Max, 6, u8> = SortedLinkedList::new_u8();
        let mut m: std::vec::Vec<u32> = std::vec::Vec::new();
        for _ in 0..30 {
            let v = (r() % 20) as u32;
            match r() % 4 {
                0 | 1 => { if l.push(v).is_ok() { m.push(v); } }
                2 => { m.sort(); assert_eq!(l.pop(), m.pop()); }
                _ => {
                    if let Some(mut f) = l.find_mut(|x| *x == v) {
                        let nv = (r() % 20) as u32; *f = nv; f.finish();
                        let p = m.iter().position(|x| *x == v).unwrap(); m[p] = nv;
                    }
                }
            }
            let mut s = m.clone(); s.sort(); s.reverse();
            assert_eq!(l.iter().copied().collect::<std::vec::Vec<_>>(), s);
        }
    }
}
#[test] fn indexmap_diff() {
    let mut seed = 0x7234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..4000 {
        let mut m: IndexMap<u32, u32, IdB, 8> = IndexMap::default();
        let mut o: std::vec::Vec<(u32, u32)> = std::vec::Vec::new();
        for s in 0..40u32 {
            let k = (r() % 24) as u32;
            match r() % 4 {
                0 | 1 => {
                    let res = m.insert(k, s);
                    if let Some(e) = o.iter_mut().find(|e| e.0 == k) { assert_eq!(res, Ok(Some(e.1))); e.1 = s; }
                    else if o.len() < 8 { assert_eq!(res, Ok(None)); o.push((k, s)); }
                    else { assert!(res.is_err()); }
                }
                2 => {
                    let res = m.remove(&k);
                    if let Some(p) = o.iter().position(|e| e.0 == k) { assert_eq!(res, Some(o.swap_remove(p).1)); } else { assert_eq!(res, None); }
                }
                _ => { let keep = (r() % 3) as u32; m.retain(|k, _| k % 3 != keep); o.retain(|e| e.0 % 3 != keep); }
            }
            assert_eq!(m.iter().map(|(a, b)| (*a, *b)).collect::<std::vec::Vec<_>>(), o);
            for k in 0..24 { assert_eq!(m.get(&k).copied(), o.iter().find(|e| e.0 == k).map(|e| e.1), "key {k}"); }
        }
    }
}
#[test] fn binary_heap_diff() {
    let mut seed = 0x1134_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    use heapless::binary_heap::{BinaryHeap, Min, PeekMutInner};
    for _ in 0..3000 {
        let mut h: BinaryHeap<u32, Min, 9> = BinaryHeap::new();
        let mut m = std::collections::BinaryHeap::new();
        for _ in 0..40 {
            let v = (r() % 50) as u32;
            match r() % 4 {
                0 | 1 => { if h.push(v).is_ok() { m.push(core::cmp::Reverse(v)); } }
                2 => { assert_eq!(h.pop(), m.pop().map(|x| x.0)); }
                _ => {
                    if let Some(mut p) = h.peek_mut() {
                        if v % 2 == 0 { *p = v; drop(p); let _ = m.pop(); m.push(core::cmp::Reverse(v)); }
                        else { let x = PeekMutInner::pop(p); assert_eq!(x, m.pop().unwrap().0); }
                    }
                }
            }
            assert_eq!(h.peek().copied(), m.peek().map(|x| x.0));
        }
    }
}
#[test] fn vec_string_diff() {
    let mut seed = 0x2234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..3000 {
        let mut s: heapless::String<24, u8> = heapless::String::new();
        let mut o = std::string::String::new();
        for _ in 0..30 {
            let c = ['a', 'é', '€', '𝄞'][(r() % 4) as usize];
            let bounds: std::vec::Vec<usize> = o.char_indices().map(|x| x.0).chain([o.len()]).collect();
            let i = bounds[(r() as usize) % bounds.len()];
            let j = bounds[(r() as usize) % bounds.len()];
            match r() % 6 {
                0 => { if s.push(c).is_ok() { o.push(c); } }
                1 => { if s.insert(i, c).is_ok() { o.insert(i, c); } }
                2 => { if s.insert_str(i, "x€").is_ok() { o.insert_str(i, "x€"); } }
                3 => { if i < o.len() { assert_eq!(s.remove(i), o.remove(i)); } }
                4 => { let (a, b) = (i.min(j), i.max(j)); let t: std::string::String = s.drain(a..b).collect(); let u: std::string::String = o.drain(a..b).collect(); assert_eq!(t, u); }
                _ => { assert_eq!(s.pop(), o.pop()); }
            }
            assert_eq!(s.as_str(), o.as_str());
        }
    }
}
#[test] fn vec_ops_diff() {
    let mut seed = 0x3234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..3000 {
        let mut s: heapless::Vec<u32, 12, u8> = heapless::Vec::new();
        let mut o = std::vec::Vec::new();
        for x in 0..30u32 {
            let i = (r() as usize) % (o.len() + 1);
            let j = (r() as usize) % (o.len() + 1);
            match r() % 6 {
                0 => { if s.push(x).is_ok() { o.push(x); } }
                1 => { if s.insert(i, x).is_ok() { o.insert(i, x); } }
                2 => { if i < o.len() { assert_eq!(s.swap_remove(i), o.swap_remove(i)); } }
                3 => { if i < o.len() { assert_eq!(s.remove(i), o.remove(i)); } }
                4 => { let (a, b) = (i.min(j), i.max(j)); let t: std::vec::Vec<u32> = s.drain(a..b).collect(); let u: std::vec::Vec<u32> = o.drain(a..b).collect(); assert_eq!(t, u); }
                _ => { let k = (r() % 3) as u32; s.retain(|v| v % 3 != k); o.retain(|v| v % 3 != k); }
            }
            assert_eq!(&s[..], &o[..]);
        }
    }
}
#[test] fn hist_diff() {
    let mut b: HistoryBuf<u32, 4> = HistoryBuf::new();
    for i in 0..11 { b.write(i); 
       let v: std::vec::Vec<u32> = b.oldest_ordered().copied().collect();
       let start = (i + 1).saturating_sub(4);
       assert_eq!(v, (start..=i).collect::<std::vec::Vec<_>>());
       assert_eq!(b.recent(), Some(&i));
       assert_eq!(b.oldest(), Some(&start));
    }
    let it = b.oldest_ordered().rev().next(); assert_eq!(it, Some(&10));
}
