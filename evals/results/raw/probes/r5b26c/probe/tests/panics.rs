// panic-path and misuse probes for Miri
use heapless::{Vec, Deque, BinaryHeap, binary_heap::Max, IndexMap, HistoryBuf, LinearMap};
use heapless::sorted_linked_list::{SortedLinkedList, Max as SMax};
use heapless::spsc::Queue;
use std::panic::{catch_unwind, AssertUnwindSafe as A};
use std::cell::Cell;
use std::cmp::Ordering;
thread_local!(static ARM: Cell<i32> = const { Cell::new(-1) });
fn tick() { ARM.with(|a| { let v=a.get(); if v==0 { a.set(-1); panic!("armed"); } if v>0 { a.set(v-1); } }); }
fn arm(n:i32){ ARM.with(|a|a.set(n)); }
#[derive(Debug)] struct D(Box<u32>);
impl Drop for D { fn drop(&mut self){ tick(); } }
impl Clone for D { fn clone(&self)->Self{ tick(); D(self.0.clone()) } }
impl PartialEq for D { fn eq(&self,o:&D)->bool{ tick(); self.0==o.0 } } impl Eq for D {}
impl PartialOrd for D { fn partial_cmp(&self,o:&D)->Option<Ordering>{ Some(self.cmp(o)) } }
impl Ord for D { fn cmp(&self,o:&D)->Ordering{ tick(); self.0.cmp(&o.0) } }
fn d(x:u32)->D{D(Box::new(x))}
fn vec5()->Vec<D,8>{ let mut v=Vec::new(); for i in 0..5 { v.push(d(i)).ok().unwrap(); } v }

#[test] fn vec_retain_pred_panic() { let mut v=vec5(); let mut i=0; let _=catch_unwind(A(||v.retain(|x|{i+=1; if i==3 {panic!("p")} *x.0!=0}))); assert_eq!(v.len(),4); }
#[test] fn vec_retain_drop_panic() { let mut v=vec5(); let _=catch_unwind(A(||{arm(0); v.retain(|x|*x.0%2==1)})); arm(-1); }
#[test] fn vec_drain_drop_panic() { let mut v=vec5(); let _=catch_unwind(A(||{arm(0); drop(v.drain(1..4));})); arm(-1); assert_eq!(v.len(),2); }
#[test] fn vec_truncate_panic() { let mut v=vec5(); let _=catch_unwind(A(||{arm(0); v.truncate(1);})); arm(-1); assert_eq!(v.len(),1); }
#[test] fn vec_clone_panic() { let v=vec5(); let _=catch_unwind(A(||{arm(2); let _c=v.clone();})); arm(-1); }
#[test] fn vec_extend_slice_panic() { let v=vec5(); let mut w:Vec<D,8>=Vec::new(); let _=catch_unwind(A(||{arm(2); w.extend_from_slice(&v[..]).unwrap();})); arm(-1); assert_eq!(w.len(),2); }
#[test] fn vec_resize_panic() { let mut w:Vec<D,8>=Vec::new(); let _=catch_unwind(A(||{arm(2); w.resize(5,d(1)).unwrap();})); arm(-1); }
#[test] fn vec_drain_forget() { let mut v=vec5(); std::mem::forget(v.drain(1..3)); assert_eq!(v.len(),1); }
#[test] fn heap_cmp_panic() { let mut h:BinaryHeap<D,Max,8>=BinaryHeap::new(); for i in 0..6 { h.push(d(i)).ok().unwrap(); }
  let _=catch_unwind(A(||{arm(1); h.pop();})); arm(-1);
  let _=catch_unwind(A(||{arm(1); let _=h.push(d(9));})); arm(-1);
  while h.pop().is_some() {} }
#[test] fn heap_peek_panic() { let mut h:BinaryHeap<D,Max,8>=BinaryHeap::new(); for i in 0..6 { h.push(d(i)).ok().unwrap(); }
  let _=catch_unwind(A(||{ let mut p=h.peek_mut().unwrap(); *p=d(0); arm(1); drop(p); })); arm(-1); assert_eq!(h.len(),6); }
#[test] fn sll_cmp_panic() { let mut l:SortedLinkedList<D,SMax,6,u8>=SortedLinkedList::new_u8(); for i in 0..4 { l.push(d(i)).ok().unwrap(); }
  let _=catch_unwind(A(||{arm(1); let _=l.push(d(9));})); arm(-1);
  let _=catch_unwind(A(||{ let mut f=l.find_mut(|x|*x.0==1).unwrap(); *f=d(7); arm(1); drop(f); })); arm(-1);
  let c:std::vec::Vec<u32>=l.iter().map(|x|*x.0).collect(); eprintln!("{c:?}"); }
#[test] fn sll_head_only_modify_pop() { let mut l:SortedLinkedList<Box<u32>,SMax,4,u8>=SortedLinkedList::new_u8(); l.push(Box::new(1)).ok().unwrap();
  let mut f=l.find_mut(|_|true).unwrap(); **f=2; let v=f.pop(); assert_eq!(*v,2); }
#[test] fn map_insert_eq_panic() { let mut m:heapless::index_map::FnvIndexMap<u32,D,4>=Default::default(); for i in 0..4 { m.insert(i,d(i)).ok().unwrap(); } m.remove(&1); m.insert(1,d(1)).ok().unwrap(); }
#[test] fn linear_drop_panic() { let mut m:LinearMap<u32,D,4>=LinearMap::new(); m.insert(1,d(1)).ok().unwrap(); m.insert(2,d(2)).ok().unwrap(); let _=catch_unwind(A(||{arm(0); m.clear();})); arm(-1); assert_eq!(m.len(),0); }
#[test] fn hist_clone_panic() { let mut h:HistoryBuf<D,4>=HistoryBuf::new(); for i in 0..6 {h.write(d(i));} let _=catch_unwind(A(||{arm(2); let _c=h.clone();})); arm(-1); }
#[test] fn deque_clone_panic() { let mut h:Deque<D,4>=Deque::new(); for i in 0..4 {h.push_back(d(i)).ok().unwrap();} h.pop_front(); h.push_back(d(5)).ok().unwrap(); let _=catch_unwind(A(||{arm(2); let _c=h.clone();})); arm(-1); }
#[test] fn deque_drop_panic_no_clear() { let mut h:Deque<D,4>=Deque::new(); for i in 0..4 {h.push_back(d(i)).ok().unwrap();} let _=catch_unwind(A(||{arm(0); drop(h);})); arm(-1); }
#[test] fn spsc_clone_drop_panic() { let mut q:Queue<D,5>=Queue::new(); for i in 0..4 {q.enqueue(d(i)).ok().unwrap();} let _=catch_unwind(A(||{arm(2); let _c=q.clone();})); arm(-1); let _=catch_unwind(A(||{arm(1); drop(q);})); arm(-1); }
#[test] fn vec_into_iter_clone() { let v=vec5(); let mut it=v.into_iter(); it.next(); let c=it.clone(); drop(it); assert_eq!(c.count(),4); }
#[test] fn vec_from_iter_lying_bad() { let _v:Vec<D,8>=(0..3).map(d).collect(); }
#[test] fn from_array_full_and_partial() { let a:Vec<D,2,u8>=Vec::from_array([d(1),d(2)]); let b:Vec<D,4,u8>=Vec::from_array([d(1),d(2)]); assert_eq!(a.len(),2); assert_eq!(b.len(),2); let c:Vec<u32,3,u8>=Vec::from_array([1,2,3]); let _:[u32;3]=c.into_array().unwrap(); let e:Vec<D,4,u8>=b; let _:std::vec::Vec<D>=e.try_into().unwrap(); }
#[test] fn cast_len() { let a:Vec<D,4,usize>=Vec::from_array([d(1),d(2)]); let b:Vec<D,4,u8>=a.cast_len_type(); assert_eq!(b.len(),2); }
#[test] fn views() { let mut v=vec5(); { let w:&mut heapless::VecView<D,usize>=&mut v; w.pop(); w.push(d(7)).ok().unwrap(); w.drain(..2); } assert_eq!(v.len(),3);
  let mut q:Deque<D,4>=Deque::new(); q.push_back(d(1)).ok().unwrap(); { let w:&mut heapless::deque::DequeView<D>=&mut q; w.push_front(d(2)).ok().unwrap(); assert_eq!(w.storage_len(),2);} }
#[test] fn map_big() { let _ = IndexMap::<u32,u32,std::hash::BuildHasherDefault<std::collections::hash_map::DefaultHasher>,4>::default(); }
