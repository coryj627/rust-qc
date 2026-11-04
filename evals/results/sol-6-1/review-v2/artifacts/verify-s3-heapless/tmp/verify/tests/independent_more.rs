use heapless::index_map::FnvIndexMap;
#[test]
fn large_map_positions() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let mut m=FnvIndexMap::<u32,u32,131072>::new();
        for i in 0..=65536 {assert_eq!(m.insert(i,i),Ok(None));}
        assert_eq!(m.len(),65537);
        assert_eq!(m.get(&65536),Some(&65536));
    }).unwrap().join().unwrap();
}
#[test]
fn spsc_peek_publication() {
    let mut q=heapless::spsc::Queue::<u32,2>::new();
    let (mut p,c)=q.split();
    std::thread::scope(|s| {
        s.spawn(move || {p.enqueue(42).unwrap();});
        loop {
            if let Some(x)=c.peek() {assert_eq!(*x,42); break;}
            std::thread::yield_now();
        }
    });
}
#[test]
fn retain_recovery_second_key() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let mut m=FnvIndexMap::<u32,u32,4>::new();
    for i in 0..3 {m.insert(i,i).unwrap();}
    let _=catch_unwind(AssertUnwindSafe(||m.retain(|k,_| match *k {0=>false,1=>panic!("predicate"),_=>true})));
    assert_eq!(m.get(&2),Some(&2));
}
