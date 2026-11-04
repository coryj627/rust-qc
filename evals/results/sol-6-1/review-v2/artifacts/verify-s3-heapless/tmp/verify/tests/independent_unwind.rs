use heapless::{Vec, Deque, HistoryBuf};
use std::{sync::atomic::{AtomicUsize, AtomicBool, Ordering::SeqCst}, panic::{catch_unwind, AssertUnwindSafe}};
struct D { id:usize, counts:&'static [AtomicUsize;3], armed:&'static AtomicBool }
impl Drop for D {fn drop(&mut self) {
    self.counts[self.id].fetch_add(1,SeqCst);
    if self.id==1 && self.armed.swap(false,SeqCst) {panic!("once");}
}}
fn state()->(&'static [AtomicUsize;3],&'static AtomicBool) {
    (Box::leak(Box::new(std::array::from_fn(|_| AtomicUsize::new(0)))),Box::leak(Box::new(AtomicBool::new(true))))
}
fn counts(c:&[AtomicUsize;3])->[usize;3] {std::array::from_fn(|i| c[i].load(SeqCst))}
#[test] fn vec_iterator_no_double_drop() {
    let (c,a)=state(); let mut v=Vec::<D,3>::new();
    for i in 0..3 {v.push(D{id:i,counts:c,armed:a}).ok();}
    let mut it=v.into_iter(); drop(it.next().unwrap());
    assert!(catch_unwind(AssertUnwindSafe(||drop(it))).is_err());
    assert_eq!(counts(c),[1,1,1]);
}
#[test] fn deque_clear_no_double_drop() {
    let (c,a)=state(); let mut v=Deque::<D,3>::new();
    for i in 0..3 {v.push_back(D{id:i,counts:c,armed:a}).ok();}
    assert!(catch_unwind(AssertUnwindSafe(||v.clear())).is_err());
    drop(v); assert_eq!(counts(c),[1,1,1]);
}
#[test] fn history_clear_no_double_drop() {
    let (c,a)=state(); let mut v=HistoryBuf::<D,3>::new();
    for i in 0..3 {v.write(D{id:i,counts:c,armed:a});}
    assert!(catch_unwind(AssertUnwindSafe(||v.clear())).is_err());
    drop(v); assert_eq!(counts(c),[1,1,1]);
}
#[test] fn history_clone_cleans_completed() {
    struct C {id:usize, cloned:bool, drops:&'static AtomicUsize}
    impl Clone for C {fn clone(&self)->Self {if self.id==1 {panic!("clone");} Self{id:self.id,cloned:true,drops:self.drops}}}
    impl Drop for C {fn drop(&mut self){if self.cloned {self.drops.fetch_add(1,SeqCst);}}}
    let drops=Box::leak(Box::new(AtomicUsize::new(0)));
    let mut h=HistoryBuf::<C,3>::new();
    for id in 0..3 {h.write(C{id,cloned:false,drops});}
    assert!(catch_unwind(AssertUnwindSafe(||h.clone())).is_err());
    assert_eq!(drops.load(SeqCst),1);
}
