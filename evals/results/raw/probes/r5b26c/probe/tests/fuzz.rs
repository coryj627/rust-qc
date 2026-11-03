use heapless::{Deque, HistoryBuf, Vec as HV, String as HS, LinearMap, BinaryHeap, CString};
use heapless::index_map::FnvIndexMap;
use heapless::binary_heap::{Max, Min, PeekMut};
use heapless::sorted_linked_list::{self as sll, SortedLinkedList};
use heapless::spsc::Queue;
use std::collections::VecDeque;

struct R(u64);
impl R { fn n(&mut self, m: usize) -> usize { let mut x=self.0; x^=x<<13; x^=x>>7; x^=x<<17; self.0=x; (x % m as u64) as usize } }

#[test] fn deque() {
  for seed in 1..(if cfg!(miri) {4u64} else {200u64}) { let mut r=R(seed*7919+1);
    let mut d:Deque<u32,5>=Deque::new(); let mut m:VecDeque<u32>=VecDeque::new();
    for i in 0..300u32 { match r.n(10) {
      0|1=>{ let a=d.push_back(i).is_ok(); assert_eq!(a,m.len()<5); if a {m.push_back(i);} }
      2|3=>{ let a=d.push_front(i).is_ok(); assert_eq!(a,m.len()<5); if a {m.push_front(i);} }
      4=>assert_eq!(d.pop_front(),m.pop_front()),
      5=>assert_eq!(d.pop_back(),m.pop_back()),
      6=>{ let x=r.n(6); assert_eq!(d.swap_remove_back(x),m.swap_remove_back(x)); }
      7=>{ let x=r.n(6); assert_eq!(d.swap_remove_front(x),m.swap_remove_front(x)); }
      8=>{ let x=r.n(6); assert_eq!(d.get(x),m.get(x)); }
      _=>{ if m.len()>1 { let a=r.n(m.len()); let b=r.n(m.len()); d.swap(a,b); m.swap(a,b);} }
    }
    assert_eq!(d.len(),m.len()); assert_eq!(d.iter().copied().collect::<Vec<_>>(), m.iter().copied().collect::<Vec<_>>());
    assert_eq!(d.iter().rev().copied().collect::<Vec<_>>(), m.iter().rev().copied().collect::<Vec<_>>());
    assert_eq!(d.front(),m.front()); assert_eq!(d.back(),m.back());
    assert_eq!(d.is_full(),m.len()==5); assert_eq!(d.iter().len(),m.len());
    let (a,b)=d.as_slices(); assert_eq!(a.len()+b.len(),m.len());
    let c=d.clone(); assert!(c==d);
    }
  }
}
#[test] fn deque_contig() {
  // every front offset and length: record what fails
  let mut bad=vec![];
  for f in 0..8usize { for l in 0..=8usize {
    let mut d:Deque<u32,8>=Deque::new();
    for _ in 0..f { d.push_back(0).unwrap(); d.pop_front(); }
    for i in 0..l as u32 { d.push_back(i).unwrap(); }
    let s:Vec<u32>=d.make_contiguous().to_vec();
    let exp:Vec<u32>=(0..l as u32).collect();
    let mut ok = s==exp && d.len()==l;
    if ok && l<8 { ok = d.push_back(100).is_ok() && d.pop_back()==Some(100); }
    if !ok { bad.push((f,l,d.len())); }
  }}
  eprintln!("make_contiguous wrong (front,len,len_after): {bad:?}");
  assert!(bad.is_empty());
}
#[test] fn index_map() {
  for seed in 1..(if cfg!(miri) {4u64} else {200u64}) { let mut r=R(seed*104729+3);
    let mut h:FnvIndexMap<u32,u32,8>=FnvIndexMap::new(); let mut m:Vec<(u32,u32)>=vec![];
    for i in 0..300u32 { let k=r.n(14) as u32*3; match r.n(8) {
      0|1|2=>{ let e=m.iter().position(|x|x.0==k); let a=h.insert(k,i);
        match e { Some(p)=>{assert_eq!(a,Ok(Some(m[p].1))); m[p].1=i;}
         None=> if m.len()==8 {assert!(a.is_err());} else {assert_eq!(a,Ok(None)); m.push((k,i));} } }
      3|4=>{ let e=m.iter().position(|x|x.0==k); let a=h.swap_remove(&k);
        match e { Some(p)=>{assert_eq!(a,Some(m.swap_remove(p).1));} None=>assert_eq!(a,None) } }
      5=>{ let t=r.n(3) as u32; h.retain(|k,_| k%3!=t||*k%2==0); m.retain(|x| x.0%3!=t||x.0%2==0); }
      6=>{ match h.entry(k) { heapless::index_map::Entry::Occupied(o)=>{ let v=o.remove(); let p=m.iter().position(|x|x.0==k).unwrap(); assert_eq!(v,m.swap_remove(p).1);} heapless::index_map::Entry::Vacant(v)=>{ let f=m.len()==8; let a=v.insert(i); assert_eq!(a.is_err(),f); if !f {m.push((k,i));} } } }
      _=>{ assert_eq!(h.get(&k).copied(), m.iter().find(|x|x.0==k).map(|x|x.1)); }
    }
    assert_eq!(h.iter().map(|(a,b)|(*a,*b)).collect::<Vec<_>>(), m);
    for (a,b) in &m { assert_eq!(h.get(a),Some(b)); assert_eq!(h.get_index_of(a), m.iter().position(|x|x.0==*a)); }
    }
  }
}
#[test] fn index_map_truncate_model() {
  let mut fails=0; let total=2000u64;
  for seed in 1..=(if cfg!(miri) {3u64} else {total}) { let mut r=R(seed*31337+3);
    let mut h:FnvIndexMap<u32,u32,8>=FnvIndexMap::new();
    for _ in 0..7 { let k=r.n(1000) as u32; let _=h.insert(k,k); }
    let l=r.n(h.len()+1); h.truncate(l);
    let keys:Vec<u32>=h.keys().copied().collect();
    if keys.iter().any(|k|h.get(k).is_none()) { fails+=1; }
  }
  eprintln!("truncate lost a live key in {fails}/{total} random FnvIndexMap<u32,_,8> runs");
  assert_eq!(fails,0);
}
#[test] fn linear() { for seed in 1..(if cfg!(miri) {4u64} else {100u64}) { let mut r=R(seed*131+3);
    let mut h:LinearMap<u32,u32,6>=LinearMap::new(); let mut m:Vec<(u32,u32)>=vec![];
    for i in 0..200u32 { let k=r.n(9) as u32; match r.n(3) {
      0=>{ let e=m.iter().position(|x|x.0==k); let a=h.insert(k,i); match e { Some(p)=>{assert_eq!(a,Ok(Some(m[p].1))); m[p].1=i;} None=> if m.len()==6 {assert!(a.is_err());} else {assert_eq!(a,Ok(None)); m.push((k,i));}} }
      1=>{ let e=m.iter().position(|x|x.0==k); let a=h.remove(&k); assert_eq!(a,e.map(|p|m.swap_remove(p).1)); }
      _=>assert_eq!(h.get(&k).copied(), m.iter().find(|x|x.0==k).map(|x|x.1)),
    }
    assert_eq!(h.iter().map(|(a,b)|(*a,*b)).collect::<Vec<_>>(), m);
    }
}}
#[test] fn heap() { for seed in 1..(if cfg!(miri) {4u64} else {100u64}) { let mut r=R(seed*17+3);
    let mut a:BinaryHeap<u32,Max,16>=BinaryHeap::new(); let mut b:BinaryHeap<u32,Min,16>=BinaryHeap::new(); let mut m:Vec<u32>=vec![];
    for _ in 0..300 { let v=r.n(50) as u32; match r.n(6) {
      0|1|2=>{ let f=m.len()==16; assert_eq!(a.push(v).is_err(),f); assert_eq!(b.push(v).is_err(),f); if !f {m.push(v);} }
      3=>{ m.sort(); let x=m.pop(); assert_eq!(a.pop(),x); if let Some(x)=x { // remove same from min heap by rebuilding
             let mut t=Vec::new(); while let Some(y)=b.pop(){t.push(y);} let p=t.iter().rposition(|y|*y==x).unwrap(); t.remove(p); for y in t {b.push(y).unwrap();} } }
      4=>{ if let Some(mut p)=a.peek_mut(){ m.sort(); let l=m.len(); let old=*p; *p=v; assert_eq!(old,m[l-1]); m[l-1]=v; } else {continue;}
           b.clear(); for y in &m { b.push(*y).unwrap(); } }
      _=>{ if let Some(p)=a.peek_mut(){ m.sort(); let x=m.pop(); assert_eq!(Some(PeekMut::pop(p)),x); b.clear(); for y in &m { b.push(*y).unwrap(); } } }
    }
    assert_eq!(a.peek().copied(), m.iter().max().copied()); assert_eq!(b.peek().copied(), m.iter().min().copied());
    }
}}
#[test] fn sorted() { for seed in 1..(if cfg!(miri) {4u64} else {100u64}) { let mut r=R(seed*19+3);
    let mut a:SortedLinkedList<u32,sll::Max,8,u8>=SortedLinkedList::new_u8(); let mut m:Vec<u32>=vec![];
    for _ in 0..300 { let v=r.n(30) as u32; match r.n(5) {
      0|1=>{ let f=m.len()==8; assert_eq!(a.push(v).is_err(),f); if !f {m.push(v);} }
      2=>{ m.sort(); assert_eq!(a.pop(),m.pop()); }
      3=>{ let w=r.n(30) as u32; if let Some(p)=a.find_mut(|x|*x==w){ let x=p.pop(); assert_eq!(x,w); let q=m.iter().position(|y|*y==w).unwrap(); m.remove(q);} else {assert!(!m.contains(&w));} }
      _=>{ let w=r.n(30) as u32; if let Some(mut p)=a.find_mut(|x|*x==w){ *p=v; let q=m.iter().position(|y|*y==w).unwrap(); m[q]=v; } }
    }
    let mut s=m.clone(); s.sort(); s.reverse(); assert_eq!(a.iter().copied().collect::<Vec<_>>(), s); assert_eq!(a.is_full(),m.len()==8);
    }
}}
#[test] fn history() { for n in 0..20u32 {
    let mut h:HistoryBuf<u32,4>=HistoryBuf::new(); let mut m:Vec<u32>=vec![];
    for i in 0..n { h.write(i); m.push(i); }
    let last:Vec<u32>=m.iter().rev().take(4).rev().copied().collect();
    assert_eq!(h.oldest_ordered().copied().collect::<Vec<_>>(), last);
    assert_eq!(h.oldest_ordered().rev().copied().collect::<Vec<_>>(), last.iter().rev().copied().collect::<Vec<_>>());
    assert_eq!(h.recent(), last.last()); assert_eq!(h.oldest(), last.first()); assert_eq!(h.len(),last.len());
    let (a,b)=h.as_slices(); let mut j=a.to_vec(); j.extend_from_slice(b); assert_eq!(j,last);
    assert_eq!(h.clone().oldest_ordered().copied().collect::<Vec<_>>(), last);
}}
#[test] fn spsc() { for seed in 1..(if cfg!(miri) {4u64} else {100u64}) { let mut r=R(seed*23+3);
    let mut q:Queue<u32,5>=Queue::new(); let mut m:VecDeque<u32>=VecDeque::new();
    for i in 0..300u32 { match r.n(4) {
      0|1=>{ let f=m.len()==4; assert_eq!(q.enqueue(i).is_err(),f); if !f {m.push_back(i);} }
      2=>assert_eq!(q.dequeue(),m.pop_front()),
      _=>{ for x in q.iter_mut(){*x+=1;} for x in m.iter_mut(){*x+=1;} }
    }
    assert_eq!(q.len(),m.len()); assert_eq!(q.peek(),m.front()); assert_eq!(q.is_full(),m.len()==4); assert_eq!(q.is_empty(),m.is_empty());
    assert_eq!(q.iter().copied().collect::<Vec<_>>(), m.iter().copied().collect::<Vec<_>>());
    assert_eq!(q.iter().rev().copied().collect::<Vec<_>>(), m.iter().rev().copied().collect::<Vec<_>>());
    let mut it=q.iter(); let mut k=m.iter(); assert_eq!(it.next(),k.next()); assert_eq!(it.next_back(),k.next_back()); assert_eq!(it.next(),k.next());
    let c=q.clone(); assert!(c==q);
    }
}}
#[test] fn vec_ops() { for seed in 1..(if cfg!(miri) {4u64} else {200u64}) { let mut r=R(seed*29+3);
    let mut h:HV<u32,8,u8>=HV::new(); let mut m:Vec<u32>=vec![];
    for i in 0..200u32 { let l=m.len(); match r.n(9) {
      0|1=>{ let f=l==8; assert_eq!(h.push(i).is_err(),f); if !f {m.push(i);} }
      2=>assert_eq!(h.pop(),m.pop()),
      3=>{ let x=r.n(l+1); let f=l==8; assert_eq!(h.insert(x,i).is_err(),f); if !f {m.insert(x,i);} }
      4=>if l>0 { let x=r.n(l); assert_eq!(h.remove(x),m.remove(x)); }
      5=>if l>0 { let x=r.n(l); assert_eq!(h.swap_remove(x),m.swap_remove(x)); }
      6=>{ let a=r.n(l+1); let b=a+r.n(l-a+1); let mut d=h.drain(a..b); let e0=d.next(); let e1=d.next_back(); drop(d);
           let mm:Vec<u32>=m.drain(a..b).collect(); assert_eq!(e0,mm.first().copied()); if mm.len()>1 {assert_eq!(e1,mm.last().copied());} }
      7=>{ let t=r.n(3) as u32; h.retain(|x|x%3!=t); m.retain(|x|x%3!=t); }
      _=>{ let n=r.n(10); let ok=h.resize(n,i).is_ok(); assert_eq!(ok,n<=8); if ok {m.resize(n,i);} }
    }
    assert_eq!(&h[..],&m[..]);
    }
}}
#[test] fn string_ops() { let cs=['a','é','€','😀'];
  for seed in 1..(if cfg!(miri) {4u64} else {200u64}) { let mut r=R(seed*37+3);
    let mut h:HS<24>=HS::new(); let mut m=String::new();
    for _ in 0..200 { let c=cs[r.n(4)]; let bds:Vec<usize>=m.char_indices().map(|x|x.0).chain([m.len()]).collect(); let p=bds[r.n(bds.len())]; match r.n(7) {
      0=>{ let f=m.len()+c.len_utf8()>24; assert_eq!(h.push(c).is_err(),f); if !f {m.push(c);} }
      1=>assert_eq!(h.pop(),m.pop()),
      2=>{ let f=m.len()+c.len_utf8()>24; assert_eq!(h.insert(p,c).is_err(),f); if !f {m.insert(p,c);} }
      3=>{ let s="é€"; let f=m.len()+s.len()>24; assert_eq!(h.insert_str(p,s).is_err(),f); if !f {m.insert_str(p,s);} }
      4=>if p<m.len() { assert_eq!(h.remove(p),m.remove(p)); }
      5=>{ h.truncate(p); m.truncate(p); }
      _=>{ let q=bds[r.n(bds.len())]; let (a,b)=(p.min(q),p.max(q)); let x:String=h.drain(a..b).rev().collect(); let y:String=m.drain(a..b).rev().collect(); assert_eq!(x,y); }
    }
    assert_eq!(h.as_str(),m.as_str());
    }
}}
#[test] fn cstring() {
  let alpha=[0u8,b'a',b'b'];
  // all byte strings up to len 4 starting from "x"
  let mut stack:Vec<Vec<u8>>=vec![vec![]];
  while let Some(b)=stack.pop() {
    let mut c=CString::<4>::new(); c.extend_from_bytes(b"x").unwrap();
    let r=c.extend_from_bytes(&b);
    assert_eq!(c.as_bytes_with_nul().last(),Some(&0));
    assert_eq!(c.as_bytes_with_nul().iter().filter(|x|**x==0).count(),1, "{b:?}");
    let nul=b.iter().position(|x|*x==0);
    let exp_interior=matches!(nul,Some(p) if p+1!=b.len());
    let payload=if nul==Some(b.len().wrapping_sub(1)) {b.len()-1} else {b.len()};
    if r.is_ok() { assert!(!exp_interior,"{b:?}"); let mut e=b"x".to_vec(); e.extend_from_slice(&b[..payload]); assert_eq!(c.as_bytes(),&e[..]); }
    else { assert_eq!(c.as_bytes(),b"x"); }
    if r.is_err() && !exp_interior { assert!(1+payload+1>4,"{b:?}"); }
    if b.len()<5 { for a in alpha { let mut n=b.clone(); n.push(a); stack.push(n);} }
  }
}
