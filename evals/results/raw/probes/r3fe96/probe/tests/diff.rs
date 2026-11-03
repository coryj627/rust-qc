use heapless::*;
use std::collections::{HashMap, VecDeque};
struct R(u64);
impl R { fn n(&mut self, m: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 % m } }
const ITERS: u64 = 300_000;

#[test]
fn deque_diff_no_mc() { deque_diff(false) }
#[test]
fn deque_diff_mc() { deque_diff(true) }
fn deque_diff(mc: bool) {
    let mut r = R(0x1234567);
    let mut d = Deque::<u32, 7>::new();
    let mut m: VecDeque<u32> = VecDeque::new();
    let mut c = 0;
    for _ in 0..ITERS {
        c += 1;
        match r.n(if mc { 11 } else { 10 }) {
            0 | 1 => { let e = d.push_back(c); if m.len() < 7 { m.push_back(c); assert!(e.is_ok()) } else { assert!(e.is_err()) } }
            2 | 3 => { let e = d.push_front(c); if m.len() < 7 { m.push_front(c); assert!(e.is_ok()) } else { assert!(e.is_err()) } }
            4 => assert_eq!(d.pop_front(), m.pop_front()),
            5 => assert_eq!(d.pop_back(), m.pop_back()),
            6 => { let i = r.n(9) as usize; assert_eq!(d.swap_remove_back(i), m.swap_remove_back(i)); }
            7 => { let i = r.n(9) as usize; assert_eq!(d.swap_remove_front(i), m.swap_remove_front(i)); }
            8 => { let i = r.n(9) as usize; assert_eq!(d.get(i), m.get(i)); }
            9 => { if !m.is_empty() { let (i,j)=(r.n(m.len() as u64) as usize, r.n(m.len() as u64) as usize); d.swap(i,j); m.swap(i,j);} }
            _ => {
                let a: std::vec::Vec<u32> = d.make_contiguous().to_vec();
                assert_eq!(a, m.iter().copied().collect::<std::vec::Vec<_>>());
                assert_eq!(d.len(), m.len(), "len after make_contiguous");
                if d.len() == 7 { assert!(d.is_full()); d.pop_front(); m.pop_front(); assert!(d.push_back(0).is_ok()); m.push_back(0); }
            }
        }
        assert_eq!(d.len(), m.len());
        assert!(d.iter().eq(m.iter()));
        assert!(d.iter().rev().eq(m.iter().rev()));
        assert_eq!(d.iter().len(), m.len());
        assert_eq!(d.front(), m.front()); assert_eq!(d.back(), m.back());
    }
}
#[test]
fn vec_diff() {
    let mut r = R(0x7654321);
    let mut d = Vec::<u32, 9, u8>::new();
    let mut m: std::vec::Vec<u32> = vec![];
    let mut c = 0;
    for _ in 0..ITERS {
        c += 1;
        match r.n(9) {
            0 | 1 | 2 => { let e = d.push(c); if m.len() < 9 { m.push(c); assert!(e.is_ok()) } }
            3 => assert_eq!(d.pop(), m.pop()),
            4 => { if !m.is_empty() { let i = r.n(m.len() as u64) as usize; assert_eq!(d.swap_remove(i), m.swap_remove(i)); } }
            5 => { if !m.is_empty() { let i = r.n(m.len() as u64) as usize; assert_eq!(d.remove(i), m.remove(i)); } }
            6 => { let i = r.n(m.len() as u64 + 1) as usize; if m.len() < 9 { d.insert(i, c).unwrap(); m.insert(i, c);} }
            7 => { let a = r.n(m.len() as u64 + 1) as usize; let b = a + r.n((m.len() - a) as u64 + 1) as usize;
                   let x: std::vec::Vec<_> = d.drain(a..b).collect(); let y: std::vec::Vec<_> = m.drain(a..b).collect(); assert_eq!(x,y); }
            _ => { let k = r.n(3) as u32; d.retain(|v| v % 3 != k); m.retain(|v| v % 3 != k); }
        }
        assert_eq!(&d[..], &m[..]);
    }
}
#[test]
fn string_diff() {
    let mut r = R(0xabcdef);
    let mut d = String::<24, u8>::new();
    let mut m = std::string::String::new();
    let chars = ['a', 'é', '€', '𝄞', 'z'];
    let bnd = |m: &std::string::String, r: &mut R| { let v: std::vec::Vec<usize> = m.char_indices().map(|x|x.0).chain([m.len()]).collect(); v[r.n(v.len() as u64) as usize] };
    for _ in 0..ITERS {
        let ch = chars[r.n(5) as usize];
        match r.n(7) {
            0 | 1 => { let e = d.push(ch); if m.len() + ch.len_utf8() <= 24 { m.push(ch); assert!(e.is_ok()) } else { assert!(e.is_err()) } }
            2 => assert_eq!(d.pop(), m.pop()),
            3 => { let i = bnd(&m,&mut r); if i < m.len() { assert_eq!(d.remove(i), m.remove(i)); } }
            4 => { let i = bnd(&m,&mut r); let e = d.insert(i, ch); if m.len() + ch.len_utf8() <= 24 { m.insert(i, ch); assert!(e.is_ok()) } else { assert!(e.is_err()) } }
            5 => { let i = bnd(&m,&mut r); let e = d.insert_str(i, "xé"); if m.len() + 3 <= 24 { m.insert_str(i, "xé"); assert!(e.is_ok()) } else { assert!(e.is_err()) } }
            _ => { let a = bnd(&m,&mut r); let b = bnd(&m,&mut r); let (a,b)=(a.min(b),a.max(b));
                   let x: std::string::String = d.drain(a..b).rev().collect(); let y: std::string::String = m.drain(a..b).rev().collect(); assert_eq!(x,y); }
        }
        assert_eq!(d.as_str(), m.as_str());
    }
}
#[derive(Default)] struct H(u64);
impl core::hash::Hasher for H { fn finish(&self)->u64{self.0} fn write(&mut self,_:&[u8]){} fn write_u32(&mut self,x:u32){self.0=(x%5) as u64;} }
#[derive(Default)] struct B;
impl core::hash::BuildHasher for B { type Hasher=H; fn build_hasher(&self)->H{H(0)} }
#[test]
fn indexmap_diff() {
    let mut r = R(0x55555);
    let mut d: IndexMap<u32,u32,B,8> = IndexMap::default();
    let mut m: HashMap<u32,u32> = HashMap::new();
    let mut order: std::vec::Vec<u32> = vec![];
    for it in 0..ITERS {
        let k = r.n(14) as u32;
        match r.n(7) {
            0|1|2 => { let e = d.insert(k, it as u32);
                if m.contains_key(&k) || m.len()<8 { let o = m.insert(k, it as u32); assert_eq!(e.unwrap(), o); if o.is_none(){order.push(k);} } else { assert!(e.is_err()); } }
            3 => { let o = m.remove(&k); assert_eq!(d.remove(&k), o);
                   if o.is_some() { let p = order.iter().position(|x|*x==k).unwrap(); order.swap_remove(p);} }
            4 => { assert_eq!(d.get(&k), m.get(&k)); assert_eq!(d.get_index_of(&k), order.iter().position(|x|*x==k)); }
            5 => { let z = r.n(3) as u32; d.retain(|k,_| k%3!=z); m.retain(|k,_| k%3!=z); order.retain(|k| k%3!=z); }
            _ => { match d.entry(k) { index_map::Entry::Occupied(o) => { assert!(m.contains_key(&k)); if it%2==0 { assert_eq!(o.remove(), m.remove(&k).unwrap()); let p = order.iter().position(|x|*x==k).unwrap(); order.swap_remove(p);} }
                   index_map::Entry::Vacant(v) => { assert!(!m.contains_key(&k)); let e=v.insert(1); if m.len()<8 { assert!(e.is_ok()); m.insert(k,1); order.push(k);} else {assert!(e.is_err());} } } }
        }
        assert_eq!(d.len(), m.len());
        assert!(d.keys().eq(order.iter()));
        for k in 0..14 { assert_eq!(d.get(&k), m.get(&k), "k={k} it={it}"); }
    }
}
#[test]
fn linearmap_diff() {
    let mut r = R(0x9999);
    let mut d: LinearMap<u32,u32,6> = LinearMap::new();
    let mut m: HashMap<u32,u32> = HashMap::new();
    for it in 0..ITERS {
        let k = r.n(10) as u32;
        match r.n(3) {
            0|1 => { let e=d.insert(k,it as u32); if m.contains_key(&k)||m.len()<6 { assert_eq!(e.unwrap(), m.insert(k,it as u32)); } else { assert!(e.is_err()); } }
            _ => assert_eq!(d.remove(&k), m.remove(&k)),
        }
        for k in 0..10 { assert_eq!(d.get(&k), m.get(&k)); }
    }
}
#[test]
fn histbuf_diff() {
    let mut r = R(0x3333);
    let mut d = HistoryBuf::<u32,5>::new();
    let mut m: VecDeque<u32> = VecDeque::new();
    for it in 0..ITERS {
        if r.n(50)==0 { d.clear(); m.clear(); }
        d.write(it as u32); m.push_back(it as u32); if m.len()>5 { m.pop_front(); }
        assert!(d.oldest_ordered().eq(m.iter()));
        assert!(d.oldest_ordered().rev().eq(m.iter().rev()));
        assert_eq!(d.recent(), m.back()); assert_eq!(d.oldest(), m.front()); assert_eq!(d.len(), m.len());
        let c = d.clone(); assert!(c == d);
    }
}
#[test]
fn heap_diff() {
    use heapless::binary_heap::{BinaryHeap as BH, Max, Min};
    let mut r = R(0x4444);
    let mut a: BH<u32,Max,16> = BH::new(); let mut b: BH<u32,Min,16> = BH::new();
    let mut m = std::vec::Vec::new();
    for _ in 0..ITERS {
        match r.n(5) {
            0|1 => { let v=r.n(50) as u32; let e=a.push(v); let f=b.push(v); if m.len()<16 {m.push(v); e.unwrap(); f.unwrap();} }
            2 => { m.sort(); assert_eq!(a.pop(), m.last().copied());
                   if !m.is_empty() { m.pop(); let mn=m.iter().min().copied(); let _ = mn; }
                   // rebuild min heap to mirror
                   b.clear(); for v in &m { b.push(*v).unwrap(); } }
            3 => { m.sort(); if !m.is_empty() { assert_eq!(b.pop(), m.first().copied()); m.remove(0); a.clear(); for v in &m { a.push(*v).unwrap(); } } }
            _ => { if let Some(mut p)=a.peek_mut() { let nv=r.n(50) as u32; let old=*p; *p=nv; drop(p); m.sort(); let last=m.len()-1; assert_eq!(m[last],old); m[last]=nv; b.clear(); for v in &m { b.push(*v).unwrap(); } } }
        }
        assert_eq!(a.peek(), m.iter().max()); assert_eq!(b.peek(), m.iter().min()); assert_eq!(a.len(), m.len());
    }
}
#[test]
fn sll_diff() {
    use heapless::sorted_linked_list::{Max, SortedLinkedList};
    let mut r = R(0x8888);
    let mut d: SortedLinkedList<u32,Max,8,u8> = SortedLinkedList::new_u8();
    let mut m: std::vec::Vec<u32> = vec![];
    for _ in 0..ITERS {
        let v = r.n(20) as u32;
        match r.n(5) {
            0|1 => { let e=d.push(v); if m.len()<8 { m.push(v); e.unwrap(); } else { assert!(e.is_err()); } }
            2 => { m.sort(); assert_eq!(d.pop(), m.pop()); }
            3 => { if let Some(f)=d.find_mut(|x| *x==v) { assert_eq!(f.pop(), v); let p=m.iter().position(|x|*x==v).unwrap(); m.swap_remove(p);} else { assert!(!m.contains(&v)); } }
            _ => { let nv=r.n(20) as u32; if let Some(mut f)=d.find_mut(|x| *x==v) { *f=nv; f.finish(); let p=m.iter().position(|x|*x==v).unwrap(); m[p]=nv; } }
        }
        m.sort(); m.reverse();
        assert!(d.iter().eq(m.iter()));
        assert_eq!(d.is_full(), m.len()==8);
    }
}
#[test]
fn spsc_diff() {
    let mut r = R(0x2222);
    let mut d = spsc::Queue::<u32,5>::new();
    let mut m: VecDeque<u32> = VecDeque::new();
    for it in 0..ITERS {
        match r.n(3) {
            0|1 => { let e=d.enqueue(it as u32); if m.len()<4 { m.push_back(it as u32); e.unwrap(); } else { assert!(e.is_err()); } }
            _ => assert_eq!(d.dequeue(), m.pop_front()),
        }
        assert_eq!(d.len(), m.len()); assert!(d.iter().eq(m.iter())); assert!(d.iter().rev().eq(m.iter().rev()));
        assert_eq!(d.peek(), m.front()); assert_eq!(d.is_full(), m.len()==4);
        let c=d.clone(); assert!(c==d);
    }
}
#[test]
fn mpmc_diff() {
    let mut r = R(0x1111);
    let d = mpmc::Queue::<u32,8>::new();
    let mut m: VecDeque<u32> = VecDeque::new();
    for it in 0..ITERS*3 {
        match r.n(2) {
            0 => { let e=d.enqueue(it as u32); if m.len()<8 { m.push_back(it as u32); e.unwrap(); } else { assert!(e.is_err()); } }
            _ => assert_eq!(d.dequeue(), m.pop_front()),
        }
    }
}
#[test]
fn cstring_diff() {
    let mut c = CString::<6>::new();
    c.extend_from_bytes(b"ab\0").unwrap();
    assert_eq!(c.as_bytes(), b"ab");
    assert!(c.extend_from_bytes(b"cde\0").is_ok());
    assert_eq!(c.as_bytes_with_nul(), b"abcde\0");
    assert!(c.extend_from_bytes(b"f").is_err());
    assert_eq!(c.as_bytes_with_nul(), b"abcde\0");
    assert!(c.extend_from_bytes(b"\0").is_ok());
    assert!(CString::<4>::from_bytes_with_nul(b"a\0b\0").is_err());
}
