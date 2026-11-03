use heapless::*;
use heapless::index_map::FnvIndexMap;
use std::sync::atomic::*;
use std::sync::Arc as SArc;

#[derive(Default, Clone)] struct H(u64);
impl core::hash::Hasher for H {
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = self.0 << 8 | *x as u64 } }
    fn write_u32(&mut self, x: u32) { self.0 = x as u64 }
    fn finish(&self) -> u64 { self.0 }
}
#[derive(Default, Clone)] struct HB;
impl core::hash::BuildHasher for HB { type Hasher = H; fn build_hasher(&self) -> H { H(0) } }

#[test] fn pos_zero_nonzero() {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let mut m: std::boxed::Box<IndexMap<u32, u32, HB, 65536>> = std::boxed::Box::new(IndexMap::default());
        for i in 0..65536u32 { m.insert(i, i).unwrap(); }
        assert_eq!(m.get(&0xffff), Some(&0xffff));
        println!("len {} indices probably has None for last", m.len());
    }).unwrap().join().unwrap();
}
#[test] fn truncate_with_default_hasher() {
    // search with the crate's default FNV hasher
    let mut found = 0;
    let mut sample = None;
    for a in 0..40u32 { for b in 0..40u32 { for c in 0..40u32 {
        if a == b || b == c || a == c { continue; }
        let mut m = FnvIndexMap::<u32, u32, 8>::new();
        m.insert(a, 1).unwrap(); m.insert(b, 2).unwrap(); m.insert(c, 3).unwrap();
        m.truncate(1);
        if m.get(&a).is_none() { found += 1; if sample.is_none() { sample = Some((a, b, c)); } }
    }}}
    println!("lost-key triples: {found} sample {sample:?}");
    let (a, b, c) = sample.unwrap();
    let mut m = FnvIndexMap::<u32, u32, 8>::new();
    m.insert(a, 1).unwrap(); m.insert(b, 2).unwrap(); m.insert(c, 3).unwrap();
    m.truncate(1);
    println!("get={:?} contains={} iter={:?}", m.get(&a), m.contains_key(&a), m);
    println!("reinsert -> {:?}; now {:?}", m.insert(a, 9), m);
    assert!(found == 0);
}
#[test] fn linear_map_eq_nan() { 
    fn is_eq<T: Eq>(_: &T) {}
    let m: LinearMap<u8, f32, 2> = LinearMap::new();
    is_eq(&m); // compiles though f32: !Eq
    let mut m = m; m.insert(1, f32::NAN).unwrap();
    assert!(m != m, "reflexivity violated by Eq type");
}
#[test] fn deque_others() {
    let mut seed = 0x5234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..3000 {
        let mut d: Deque<u32, 7> = Deque::new();
        let mut m = std::collections::VecDeque::new();
        for s in 0..40u32 {
            let i = (r() as usize) % 8;
            match r() % 7 {
                0 => { if d.push_back(s).is_ok() { m.push_back(s); } }
                1 => { if d.push_front(s).is_ok() { m.push_front(s); } }
                2 => { assert_eq!(d.pop_front(), m.pop_front()); }
                3 => { assert_eq!(d.pop_back(), m.pop_back()); }
                4 => { assert_eq!(d.swap_remove_back(i), m.swap_remove_back(i)); }
                5 => { assert_eq!(d.swap_remove_front(i), m.swap_remove_front(i)); }
                _ => { assert_eq!(d.get(i), m.get(i)); if i < m.len() { let j = (r() as usize) % m.len(); d.swap(i, j); m.swap(i, j); } }
            }
            assert!(d.len() == m.len() && d.iter().eq(m.iter()) && d.iter().rev().eq(m.iter().rev()));
            assert_eq!(d.back(), m.back()); assert_eq!(d.front(), m.front());
            assert!(d.clone() == d);
        }
    }
}
#[test] fn mpmc_wrap_single() {
    let q: mpmc::Queue<u32, 128> = mpmc::Queue::new();
    let mut m = std::collections::VecDeque::new();
    let mut seed = 0x6234_5678_9abc_def1u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for s in 0..200000u32 {
        if r() % 2 == 0 { let ok = q.enqueue(s).is_ok(); assert_eq!(ok, m.len() < 128); if ok { m.push_back(s); } }
        else { assert_eq!(q.dequeue(), m.pop_front()); }
    }
}
// Stress the u8 sequence counters: more threads than cores so a thread gets preempted between its
// load of the position and the CAS while others wrap the counter.
#[test] fn mpmc_stress_u8_wrap() {
    let q: &'static mpmc::Queue<u64, 2> = std::boxed::Box::leak(std::boxed::Box::new(mpmc::Queue::new()));
    const THREADS: u64 = 12;
    const PER: u64 = 3_000_000;
    let enq = SArc::new(AtomicU64::new(0));
    let deq = SArc::new(AtomicU64::new(0));
    let stop = SArc::new(AtomicBool::new(false));
    let mut hs = std::vec::Vec::new();
    for t in 0..THREADS {
        let (enq, deq, stop) = (enq.clone(), deq.clone(), stop.clone());
        hs.push(std::thread::spawn(move || {
            let mut seen = 0u64;
            let mut i = 0;
            while i < PER && !stop.load(Ordering::Relaxed) {
                let v = (t << 40) | i;
                if q.enqueue(v).is_ok() { enq.fetch_add(v & 0xffff, Ordering::Relaxed); i += 1; }
                if let Some(x) = q.dequeue() { deq.fetch_add(x & 0xffff, Ordering::Relaxed); seen += 1; }
            }
            seen
        }));
    }
    let start = std::time::Instant::now();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if hs.iter().all(|h| h.is_finished()) { break; }
        if start.elapsed().as_secs() > 150 { stop.store(true, Ordering::Relaxed); println!("TIMEOUT/possible hang"); break; }
    }
    if start.elapsed().as_secs() > 165 { return; }
    for h in hs { let _ = h.join(); }
    while let Some(x) = q.dequeue() { deq.fetch_add(x & 0xffff, Ordering::Relaxed); }
    let (e, d) = (enq.load(Ordering::Relaxed), deq.load(Ordering::Relaxed));
    println!("enqueued sum {e} dequeued sum {d} diff {}", e as i128 - d as i128);
    assert_eq!(e, d);
}
#[test] fn ser_view() {
    // check at compile level below
    fn ser<T: serde::Serialize + ?Sized>(_: &T) {}
    let d: Deque<u8, 2> = Deque::new();
    ser(d.as_view());
    let s: heapless::String<2> = heapless::String::new();
    ser(s.as_view());
    let v: heapless::Vec<u8, 2> = heapless::Vec::new();
    ser(&v);
    #[cfg(feature = "x")] ser(v.as_view());
}
#[test] fn iter_misc() {
    let v: heapless::Vec<u32, 4> = heapless::Vec::from_slice(&[1, 2, 3]).unwrap();
    let mut it = v.clone().into_iter();
    it.next();
    println!("into_iter size_hint {:?}", it.size_hint());
    let c = it.clone();
    assert_eq!(c.collect::<std::vec::Vec<_>>(), [2, 3]);
    let mut d: Deque<u32, 4> = Deque::new(); d.push_back(1).unwrap(); println!("deque into_iter hint {:?}", d.into_iter().size_hint());
    let mut m = FnvIndexMap::<u32, u32, 4>::new(); m.insert(1, 1).unwrap(); println!("indexmap iter hint {:?}", m.iter().size_hint());
    let a: [u32; 3] = v.clone().into_array().unwrap(); assert_eq!(a, [1, 2, 3]);
    let w: heapless::Vec<u32, 4, u8> = v.cast_len_type(); assert_eq!(w.len(), 3);
    // history 0
}
