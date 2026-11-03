use heapless::*;
use heapless::index_map::FnvIndexMap;
use heapless::sorted_linked_list::{SortedLinkedList, Max};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::string::{String as S};
use std::sync::atomic::{AtomicUsize, Ordering};

// A droppable that owns heap memory (so a double drop is a double free) and may panic once.
struct D(std::boxed::Box<u32>, bool);
impl Drop for D { fn drop(&mut self) { if self.1 { self.1 = false; panic!("boom") } } }
fn d(v: u32, p: bool) -> D { D(std::boxed::Box::new(v), p) }

#[test] fn deque_full_contig_then_push_oob() {
    let mut d: Deque<u32, 8> = Deque::new();
    for i in 0..8 { d.push_back(i).unwrap(); }
    for _ in 0..5 { d.pop_front(); }
    for i in 8..13 { d.push_back(i).unwrap(); }
    assert!(d.is_full());
    d.make_contiguous();
    d.pop_front();
    d.push_back(99).unwrap(); // writes buffer[8]
    println!("{:?}", d);
}
#[test] fn deque_contig_len_uninit_read() {
    let mut d: Deque<S, 12> = Deque::new();
    for i in 0..12 { d.push_back(i.to_string()).unwrap(); }
    for _ in 0..7 { d.pop_front(); }
    for i in 12..15 { d.push_back(i.to_string()).unwrap(); }
    d.make_contiguous();
    println!("{}", d.len());
    println!("{:?}", d); // reads an uninit slot
}
#[test] fn deque_clear_panic_double_drop() {
    let mut q: Deque<D, 4> = Deque::new();
    q.push_back(d(1, true)).ok().unwrap();
    q.push_back(d(2, false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| q.clear()));
    drop(q);
}
#[test] fn hist_clear_panic_double_drop() {
    let mut q: HistoryBuf<D, 4> = HistoryBuf::new();
    q.write(d(1, true));
    q.write(d(2, false));
    let _ = catch_unwind(AssertUnwindSafe(|| q.clear()));
    drop(q);
}
#[test] fn hist_write_panic_double_drop() {
    let mut q: HistoryBuf<D, 2> = HistoryBuf::new();
    q.write(d(1, true));
    q.write(d(2, false));
    let _ = catch_unwind(AssertUnwindSafe(|| q.write(d(3, false))));
    drop(q);
}
#[test] fn vec_into_iter_panic_double_drop() {
    let mut q: heapless::Vec<D, 4> = heapless::Vec::new();
    q.push(d(1, false)).ok().unwrap();
    q.push(d(2, true)).ok().unwrap();
    q.push(d(3, false)).ok().unwrap();
    let mut it = q.into_iter();
    drop(it.next());
    let _ = catch_unwind(AssertUnwindSafe(|| drop(it)));
}
#[test] fn vec_retain_panic() {
    let mut q: heapless::Vec<S, 8> = heapless::Vec::new();
    for i in 0..6 { q.push(i.to_string()).unwrap(); }
    let mut n = 0;
    let _ = catch_unwind(AssertUnwindSafe(|| q.retain(|_| { n += 1; if n == 3 { panic!("x") } n != 1 })));
    println!("{:?}", q);
}
#[test] fn vec_drain_panic_drop() {
    let mut q: heapless::Vec<D, 8> = heapless::Vec::new();
    for i in 0..6 { q.push(d(i, i == 2)).ok().unwrap(); }
    let _ = catch_unwind(AssertUnwindSafe(|| { q.drain(1..4); }));
    assert_eq!(q.len(), 3);
    assert_eq!(*q[1].0, 4);
}
#[test] fn indexmap_retain_panic_stale() {
    let mut m = FnvIndexMap::<S, S, 8>::new();
    for i in 0..5 { m.insert(i.to_string(), i.to_string()).unwrap(); }
    let mut n = 0;
    let _ = catch_unwind(AssertUnwindSafe(|| m.retain(|_, _| { n += 1; if n == 3 { panic!("x") } n != 1 })));
    println!("{} {:?}", m.len(), m);
    for i in 0..5 { println!("{:?}", m.get(&i.to_string())); }
}
#[test] fn indexmap_clear_panic_stale() {
    let mut m = FnvIndexMap::<u32, D, 8>::new();
    m.insert(1, d(1, true)).ok().unwrap();
    m.insert(2, d(2, false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| m.clear()));
    assert_eq!(m.len(), 0);
    let _ = m.insert(1, d(5, false));
    println!("{:?}", m.get(&2).map(|x| *x.0));
}
#[test] fn sorted_find_mut_pop_double() {
    let mut l: SortedLinkedList<S, Max, 3, u8> = SortedLinkedList::new_u8();
    l.push("a".to_string()).unwrap();
    l.push("b".to_string()).unwrap();
    l.push("c".to_string()).unwrap();
    let mut f = l.find_mut(|x| x == "b").unwrap();
    f.push('x');
    let v = f.pop();
    println!("{v}");
    while let Some(x) = l.pop() { println!("{x}"); }
}
#[test] fn sorted_find_mut_pop_head_only() {
    let mut l: SortedLinkedList<S, Max, 3, u8> = SortedLinkedList::new_u8();
    l.push("a".to_string()).unwrap();
    let mut f = l.find_mut(|x| x == "a").unwrap();
    f.push('x');
    let v = f.pop();
    println!("{v}");
}
#[test] fn spsc_peek_race() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let mut q: spsc::Queue<u64, 4> = spsc::Queue::new();
    let (mut p, c) = q.split();
    std::thread::scope(|s| {
        s.spawn(move || { for i in 0..3 { p.enqueue(i).unwrap(); COUNT.fetch_add(0, Ordering::Relaxed); } });
        s.spawn(move || { for _ in 0..200 { if let Some(v) = c.peek() { std::hint::black_box(*v); break; } std::thread::yield_now(); } });
    });
}
#[test] fn spsc_ready_dequeue_unchecked_race() {
    let mut q: spsc::Queue<u64, 4> = spsc::Queue::new();
    let (mut p, mut c) = q.split();
    std::thread::scope(|s| {
        s.spawn(move || { for i in 0..3 { p.enqueue(i).unwrap(); } });
        s.spawn(move || { for _ in 0..200 { if c.ready() { std::hint::black_box(unsafe { c.dequeue_unchecked() }); break; } std::thread::yield_now(); } });
    });
}
#[test] fn spsc_peek_loop() {
    let mut q: spsc::Queue<u64, 4> = spsc::Queue::new();
    let (mut p, c) = q.split();
    std::thread::scope(|s| {
        s.spawn(move || { p.enqueue(7).unwrap(); });
        s.spawn(move || { loop { if let Some(v) = c.peek() { std::hint::black_box(*v); break; } std::thread::yield_now(); } });
    });
}
#[test] fn spsc_prod_ready_enqueue_unchecked() {
    let mut q: spsc::Queue<u64, 2> = spsc::Queue::new();
    let (mut p, mut c) = q.split();
    unsafe { p.enqueue_unchecked(1) };
    std::thread::scope(|s| {
        s.spawn(move || { std::hint::black_box(c.dequeue()); });
        s.spawn(move || { loop { if p.ready() { unsafe { p.enqueue_unchecked(2) }; break; } std::thread::yield_now(); } });
    });
}
#[derive(PartialEq, Eq)] struct PO(std::boxed::Box<u32>);
static TRIP: AtomicUsize = AtomicUsize::new(usize::MAX);
impl PartialOrd for PO { fn partial_cmp(&self, o: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(o)) } }
impl Ord for PO { fn cmp(&self, o: &Self) -> core::cmp::Ordering {
    if TRIP.fetch_sub(1, Ordering::Relaxed) == 0 { panic!("ord") }
    self.0.cmp(&o.0) } }
#[test] fn heap_and_list_ord_panic() {
    use heapless::binary_heap::{BinaryHeap, Max as HMax, PeekMutInner};
    for trip in 0..12 {
        let mut h: BinaryHeap<PO, HMax, 8> = BinaryHeap::new();
        TRIP.store(usize::MAX, Ordering::Relaxed);
        for i in 0..7u32 { let _ = h.push(PO(std::boxed::Box::new(i * 7 % 5))); }
        TRIP.store(trip, Ordering::Relaxed);
        let _ = catch_unwind(AssertUnwindSafe(|| { h.pop(); h.push(PO(std::boxed::Box::new(9))).ok(); }));
        TRIP.store(trip, Ordering::Relaxed);
        let _ = catch_unwind(AssertUnwindSafe(|| { if let Some(mut p) = h.peek_mut() { *p = PO(std::boxed::Box::new(0)); } }));
        TRIP.store(trip, Ordering::Relaxed);
        let _ = catch_unwind(AssertUnwindSafe(|| { if let Some(p) = h.peek_mut() { let _ = PeekMutInner::pop(p); } }));
        TRIP.store(usize::MAX, Ordering::Relaxed);
        drop(h);
        let mut l: SortedLinkedList<PO, Max, 8, u8> = SortedLinkedList::new_u8();
        for i in 0..7u32 { let _ = l.push(PO(std::boxed::Box::new(i * 7 % 5))); }
        TRIP.store(trip, Ordering::Relaxed);
        let _ = catch_unwind(AssertUnwindSafe(|| { let _ = l.push(PO(std::boxed::Box::new(3))); }));
        let _ = catch_unwind(AssertUnwindSafe(|| { let _ = l.push(PO(std::boxed::Box::new(4))); }));
        TRIP.store(trip, Ordering::Relaxed);
        let _ = catch_unwind(AssertUnwindSafe(|| { if let Some(mut f) = l.find_mut(|x| *x.0 == 2) { *f = PO(std::boxed::Box::new(9)); } }));
        TRIP.store(usize::MAX, Ordering::Relaxed);
        while l.pop().is_some() {}
    }
}
use heapless::{box_pool, pool::boxed::{BoxBlock, BoxPool}};
box_pool!(BP: u128);
#[test] fn pool_race() {
    use std::ptr::addr_of_mut;
    static mut B: [BoxBlock<u128>; 2] = [const { BoxBlock::new() }; 2];
    let blocks: &'static mut [BoxBlock<u128>; 2] = unsafe { addr_of_mut!(B).as_mut().unwrap() };
    for b in blocks.iter_mut() { BP.manage(b); }
    std::thread::scope(|s| {
        for t in 0..3u128 {
            s.spawn(move || { for i in 0..4u128 {
                if let Ok(mut bx) = BP.alloc(t * 100 + i) { *bx += 1; std::thread::yield_now(); assert_eq!(*bx, t * 100 + i + 1); drop(bx); }
                std::thread::yield_now();
            }});
        }
    });
}
#[test] fn spsc_producer_ready_race2() {
    let mut q: spsc::Queue<u64, 3> = spsc::Queue::new();
    let (mut p, mut c) = q.split();
    p.enqueue(1).unwrap(); p.enqueue(2).unwrap();
    std::thread::scope(|s| {
        s.spawn(move || { let mut n = 0; while n < 3 { if let Some(v) = c.dequeue() { std::hint::black_box(v); n += 1; } else { std::thread::yield_now(); } } });
        s.spawn(move || { let mut n = 0; let mut sp = 0; while n < 2 && sp < 2000 { if p.ready() { unsafe { p.enqueue_unchecked(9) }; n += 1; } else { sp += 1; std::thread::yield_now(); } } });
    });
}
