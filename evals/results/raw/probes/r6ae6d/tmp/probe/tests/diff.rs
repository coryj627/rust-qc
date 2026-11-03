use core::hash::{BuildHasherDefault, Hasher};
use std::collections::{HashMap, VecDeque};
use heapless::{binary_heap::{BinaryHeap, Max, Min, PeekMut}, sorted_linked_list::{SortedLinkedList, Max as SMax}, Deque, HistoryBuf, IndexMap, LinearMap};
struct R(u64);
impl R { fn n(&mut self, m: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; (self.0 >> 20) % m } }
#[derive(Default)] struct Low(u64);
impl Hasher for Low { fn finish(&self) -> u64 { self.0 } fn write(&mut self, _: &[u8]) {} fn write_u32(&mut self, i: u32) { self.0 = (i % 5) as u64 } }

#[test] fn indexmap() { let mut r = R(0x1234567);
  for _ in 0..300 { let mut m: IndexMap<u32, u32, BuildHasherDefault<Low>, 16> = IndexMap::default(); let mut o: std::vec::Vec<(u32,u32)> = vec![];
    for s in 0..400u32 { let k = r.n(30) as u32; match r.n(10) {
      0..=4 => { let e = o.iter().position(|x| x.0==k); let res = m.insert(k,s); match e { Some(i) => { assert_eq!(res.unwrap(), Some(o[i].1)); o[i].1=s; } None => if o.len()==16 { assert!(res.is_err()) } else { assert_eq!(res.unwrap(),None); o.push((k,s)); } } }
      5..=7 => { let e = o.iter().position(|x| x.0==k); assert_eq!(m.remove(&k), e.map(|i| o.swap_remove(i).1)); }
      8 => { let t = r.n(3) as u32; m.retain(|k,_| k % 3 != t); o.retain(|x| x.0 % 3 != t); }
      _ => { use heapless::index_map::Entry; match m.entry(k) { Entry::Occupied(e) => { let i=o.iter().position(|x| x.0==k).unwrap(); assert_eq!(e.remove(), o.swap_remove(i).1); } Entry::Vacant(v) => { if o.len()<16 { v.insert(s).ok().unwrap(); o.push((k,s)); } else { assert!(v.insert(s).is_err()); } } } }
    }
    assert_eq!(m.iter().map(|(a,b)|(*a,*b)).collect::<std::vec::Vec<_>>(), o);
    for q in 0..30 { assert_eq!(m.get(&q).copied(), o.iter().find(|x|x.0==q).map(|x|x.1)); } } } }

#[test] fn deque() { let mut r = R(77);
  for _ in 0..300 { let mut d: Deque<u32,7> = Deque::new(); let mut o = VecDeque::new();
    for s in 0..300u32 { match r.n(8) {
      0|1 => { let x=d.push_back(s); if o.len()<7 {x.unwrap();o.push_back(s);} else {assert!(x.is_err())} }
      2|3 => { let x=d.push_front(s); if o.len()<7 {x.unwrap();o.push_front(s);} else {assert!(x.is_err())} }
      4 => assert_eq!(d.pop_front(),o.pop_front()), 5 => assert_eq!(d.pop_back(),o.pop_back()),
      6 => { let i=r.n(8) as usize; assert_eq!(d.swap_remove_back(i),o.swap_remove_back(i)); }
      _ => { let i=r.n(8) as usize; assert_eq!(d.swap_remove_front(i),o.swap_remove_front(i)); } }
      assert_eq!(d.iter().copied().collect::<std::vec::Vec<_>>(), o.iter().copied().collect::<std::vec::Vec<_>>());
      assert_eq!(d.len(),o.len()); assert_eq!(d.back(),o.back()); assert_eq!(d.front(),o.front());
      let i=r.n(8) as usize; assert_eq!(d.get(i),o.get(i)); } } }

#[test] fn vec_string() { let mut r = R(99);
  for _ in 0..300 { let mut v: heapless::Vec<u32,16> = heapless::Vec::new(); let mut o: std::vec::Vec<u32> = vec![];
    let mut s: heapless::String<32> = heapless::String::new(); let mut t = std::string::String::new();
    for x in 0..200u32 { match r.n(7) {
      0|1 => if o.len()<16 { let i=r.n(o.len() as u64+1) as usize; v.insert(i,x).unwrap(); o.insert(i,x);} 
      2 => if !o.is_empty() { let i=r.n(o.len() as u64) as usize; assert_eq!(v.remove(i),o.remove(i)); }
      3 => { let a=r.n(o.len() as u64+1) as usize; let b=a+r.n((o.len()-a) as u64+1) as usize; let p:std::vec::Vec<_>=v.drain(a..b).collect(); let q:std::vec::Vec<_>=o.drain(a..b).collect(); assert_eq!(p,q); }
      4 => { let m=r.n(3) as u32+2; v.retain(|e|e%m!=0); o.retain(|e|e%m!=0); }
      5 => if !o.is_empty() { let i=r.n(o.len() as u64) as usize; assert_eq!(v.swap_remove(i),o.swap_remove(i)); }
      _ => { let n=r.n(o.len() as u64+2) as usize; v.truncate(n); o.truncate(n);} }
      assert_eq!(&v[..],&o[..]);
      let c=['a','é','€','𝄞'][r.n(4) as usize];
      let bs:std::vec::Vec<usize>=t.char_indices().map(|x|x.0).chain([t.len()]).collect();
      let i=bs[r.n(bs.len() as u64) as usize]; let j=bs[r.n(bs.len() as u64) as usize]; let (a,b)=(i.min(j),i.max(j));
      match r.n(5) { 0 => if t.len()+c.len_utf8()<=32 {s.insert(i,c).unwrap();t.insert(i,c);} else {assert!(s.insert(i,c).is_err())},
        1 => if i<t.len() {assert_eq!(s.remove(i),t.remove(i));}, 2 => assert_eq!(s.pop(),t.pop()),
        3 => { let p:std::string::String=s.drain(a..b).collect(); let q:std::string::String=t.drain(a..b).collect(); assert_eq!(p,q); }
        _ => if t.len()+4<=32 { s.insert_str(i,"x€").unwrap(); t.insert_str(i,"x€"); } }
      assert_eq!(s.as_str(),t.as_str()); } } }

#[test] fn heap_sll_hist_linear() { let mut r = R(4242);
  for _ in 0..300 {
    let mut h: BinaryHeap<u32,Max,9> = BinaryHeap::new(); let mut l: BinaryHeap<u32,Min,9> = BinaryHeap::new(); let mut o: std::vec::Vec<u32> = vec![]; let mut lo: std::vec::Vec<u32> = vec![];
    let mut sl: SortedLinkedList<u32,SMax,8,u8> = SortedLinkedList::new_u8(); let mut so: std::vec::Vec<u32> = vec![];
    let mut hb: HistoryBuf<u32,5> = HistoryBuf::new(); let mut ho: VecDeque<u32> = VecDeque::new();
    let mut lm: LinearMap<u32,u32,6> = LinearMap::new(); let mut lmo: HashMap<u32,u32> = HashMap::new();
    for x in 0..200u32 { let e=r.n(40) as u32;
      match r.n(4) { 0|1 => if o.len()<9 { h.push(e).unwrap(); o.push(e); } else {assert!(h.push(e).is_err())}, 2 => { o.sort(); assert_eq!(h.pop(),o.pop()); }
        _ => if !o.is_empty() { o.sort(); let mut p=h.peek_mut().unwrap(); *p=e; drop(p); *o.last_mut().unwrap()=e; o.sort(); assert_eq!(h.peek(),o.last()); if r.n(3)==0 { assert_eq!(PeekMut::pop(h.peek_mut().unwrap()),o.pop().unwrap()); } } }
      match r.n(3) { 0|1 => if lo.len()<9 { l.push(e).unwrap(); lo.push(e);} , _ => { lo.sort_by(|a,b|b.cmp(a)); assert_eq!(l.pop(),lo.pop()); } }
      match r.n(4) { 0|1 => if so.len()<8 { sl.push(e).unwrap(); so.push(e);} else {assert!(sl.push(e).is_err())}, 2 => { so.sort(); assert_eq!(sl.pop(),so.pop()); }
        _ => { let t=e; if let Some(mut f)=sl.find_mut(|v| *v>=t) { let old=*f; *f=e+1; f.finish(); let i=so.iter().position(|v|*v==old).unwrap(); so[i]=e+1; } } }
      so.sort(); assert_eq!(sl.iter().copied().collect::<std::vec::Vec<_>>(), so.iter().rev().copied().collect::<std::vec::Vec<_>>());
      hb.write(x); ho.push_back(x); if ho.len()>5 {ho.pop_front();}
      assert_eq!(hb.oldest_ordered().copied().collect::<std::vec::Vec<_>>(), ho.iter().copied().collect::<std::vec::Vec<_>>()); assert_eq!(hb.recent(),ho.back()); assert_eq!(hb.oldest(),ho.front());
      let k=e%10; if r.n(2)==0 { let rs=lm.insert(k,x); if lmo.contains_key(&k)||lmo.len()<6 { assert_eq!(rs.unwrap(),lmo.insert(k,x)); } else {assert!(rs.is_err())} } else { assert_eq!(lm.remove(&k),lmo.remove(&k)); }
      assert_eq!(lm.len(),lmo.len()); } } }

#[test] fn spsc_wrap() { let mut r=R(5150); let mut q: heapless::spsc::Queue<u32,5> = heapless::spsc::Queue::new(); let mut o=VecDeque::new();
  for x in 0..20000u32 { if r.n(2)==0 { let a=q.enqueue(x); if o.len()<4 {a.unwrap();o.push_back(x);} else {assert!(a.is_err())} } else { assert_eq!(q.dequeue(),o.pop_front()); }
    assert_eq!(q.len(),o.len()); assert_eq!(q.iter().copied().collect::<std::vec::Vec<_>>(),o.iter().copied().collect::<std::vec::Vec<_>>()); assert_eq!(q.iter().rev().count(),o.len()); } }
