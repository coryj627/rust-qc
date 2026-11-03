use heapless::*;
use heapless::sorted_linked_list::{Max, SortedLinkedList};
use heapless::binary_heap::{BinaryHeap as BH};
use std::panic::{catch_unwind, AssertUnwindSafe};
struct R(u64);
impl R { fn n(&mut self, m: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 % m } }
const IT: u64 = 1500;
fn b(x: u64) -> Box<u32> { Box::new(x as u32) }
#[test] fn deque_box() {
    let mut r = R(77); let mut d = Deque::<Box<u32>,5>::new();
    for i in 0..IT { match r.n(8) { 0|1 => { let _=d.push_back(b(i)); } 2|3 => { let _=d.push_front(b(i)); } 4 => {d.pop_front();} 5=>{d.pop_back();} 6=>{d.swap_remove_back(r.n(5) as usize);} _=>{d.swap_remove_front(r.n(5) as usize);} }
      if r.n(100)==0 { let c=d.clone(); let _: std::vec::Vec<_> = c.into_iter().take(2).collect(); } if r.n(200)==0 { d.clear(); } }
}
#[test] fn vec_box() {
    let mut r = R(78); let mut d = Vec::<Box<u32>,6,u8>::new();
    for i in 0..IT { match r.n(8) { 0|1|2 => { let _=d.push(b(i)); } 3 => {d.pop();} 4 => { if !d.is_empty(){ let k=r.n(d.len() as u64) as usize; d.swap_remove(k);} } 5 => { if !d.is_empty(){ let k=r.n(d.len() as u64) as usize; d.remove(k);} }
        6 => { let a=r.n(d.len() as u64+1) as usize; let e=a+r.n((d.len()-a) as u64+1) as usize; let mut it=d.drain(a..e); if r.n(2)==0 { it.next(); } if r.n(2)==0 { it.next_back(); } }
        _ => { let k=r.n(3) as u32; d.retain(|v| **v%3!=k); } }
      if r.n(100)==0 { let c=d.clone(); let mut it=c.into_iter(); it.next(); let _c2=it.clone(); } if r.n(100)==0 { let a=Vec::<Box<u32>,3>::from_array([b(1),b(2),b(3)]); let _x:[Box<u32>;3]=a.into_array().ok().unwrap(); let a=Vec::<Box<u32>,4>::from_array([b(1),b(2),b(3)]); let _=a.into_array::<2>().err().unwrap().cast_len_type::<u8>(); } }
    let mut z = Vec::<(),4>::new(); for _ in 0..4 { z.push(()).unwrap(); } { let mut it=z.drain(1..3); it.next(); } assert_eq!(z.len(),2);
}
#[test] fn hist_box() { let mut r=R(79); let mut d=HistoryBuf::<Box<u32>,3>::new(); for i in 0..IT { d.write(b(i)); if r.n(20)==0 { let _c=d.clone(); } if r.n(50)==0 { d.clear(); } } }
#[test] fn map_box() {
    let mut r=R(80); let mut d:index_map::FnvIndexMap<u32,Box<u32>,4>=index_map::FnvIndexMap::new(); let mut l:LinearMap<Box<u32>,Box<u32>,3>=LinearMap::new();
    for i in 0..IT { let k=r.n(7) as u32; match r.n(6) { 0|1=>{let _=d.insert(k,b(i));} 2=>{d.remove(&k);} 3=>{d.retain(|k,_| k%2==0 || i%3==0);} 4=>{let _=d.entry(k).or_insert(b(i));} _=>{ let _=d.clone().into_iter().next(); } }
      let _=l.insert(Box::new(k),b(i)); if r.n(2)==0 { l.remove(&Box::new(r.n(7) as u32)); } if r.n(50)==0 { let _=l.clone().into_iter().next(); } }
}
#[test] fn heap_box() { let mut r=R(81); let mut h:BH<Box<u32>,binary_heap::Max,5>=BH::new(); for i in 0..IT { match r.n(4) {0|1=>{let _=h.push(Box::new(r.n(10) as u32 + i as u32 %2));} 2=>{h.pop();} _=>{ if let Some(mut p)=h.peek_mut(){ if r.n(2)==0 { **p=r.n(10) as u32; } else { let _=binary_heap::PeekMut::pop(p); } } } } } }
#[test] fn sll_box() { let mut r=R(82); let mut h:SortedLinkedList<Box<u32>,Max,5,u8>=SortedLinkedList::new_u8(); for _ in 0..IT { let v=r.n(9) as u32; match r.n(4) {0|1=>{let _=h.push(Box::new(v));} 2=>{h.pop();} _=>{ if let Some(mut f)=h.find_mut(|x| **x==v){ if r.n(2)==0 { **f=r.n(9) as u32; } else { let _=f.pop(); } } } } } }
#[test] fn spsc_threads() {
    let mut q=spsc::Queue::<Box<u32>,4>::new(); let (mut p,mut c)=q.split();
    std::thread::scope(|s|{ s.spawn(move||{ for i in 0..60u32 { let mut v=Box::new(i); loop { match p.enqueue(v){Ok(())=>break,Err(x)=>{v=x;std::thread::yield_now();}} } } });
      s.spawn(move||{ let mut n=0; while n<60 { if let Some(v)=c.dequeue(){ assert_eq!(*v,n); n+=1; } else { std::thread::yield_now(); } } }); });
}
#[test] fn mpmc_threads() {
    let q=mpmc::Queue::<Box<u32>,4>::new();
    std::thread::scope(|s|{ for t in 0..3 { let q=&q; s.spawn(move||{ for i in 0..40u32 { let _=q.enqueue(Box::new(i+t)); if let Some(v)=q.dequeue(){ assert!(*v<50);} std::thread::yield_now(); } }); } });
}
#[derive(Hash,PartialEq,Eq,Debug)] struct PK(Box<u32>, bool);
impl Drop for PK { fn drop(&mut self){ if self.1 { self.1=false; panic!("k"); } } }
#[test] fn indexmap_clear_panic_then_insert() {
    let mut m:index_map::FnvIndexMap<u32,PV,8>=index_map::FnvIndexMap::new();
    for i in 0..4 { let _=m.insert(i,PV(Box::new(i),i==0)); }
    let _=catch_unwind(AssertUnwindSafe(||m.clear()));
    println!("len after panicking clear {}", m.len());
    let _=m.insert(100,PV(Box::new(1),false));
    for i in 0..4 { println!("get {i} {:?}", m.get(&i).map(|v|*v.0)); }
}
struct PV(Box<u32>,bool);
impl Drop for PV { fn drop(&mut self){ if self.1 { self.1=false; panic!("v"); } } }
