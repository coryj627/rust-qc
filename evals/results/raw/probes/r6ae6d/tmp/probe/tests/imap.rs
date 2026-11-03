use core::hash::{BuildHasherDefault, Hasher};
use heapless::IndexMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Default)]
struct Id(u64);
impl Hasher for Id {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, _: &[u8]) { unreachable!() }
    fn write_u32(&mut self, i: u32) { self.0 = (i >> 16) as u64 }
}
// Key = (id<<... ) high 16 bits select the hash, low 16 bits make it unique.
fn k(hash: u32, uniq: u32) -> u32 { (hash << 16) | uniq }

type M<V, const N: usize> = IndexMap<u32, V, BuildHasherDefault<Id>, N>;

#[test]
fn truncate_loses_retained_key() {
    let mut m: M<u32, 8> = IndexMap::default();
    let r1 = k(2, 1);
    let r2 = k(2, 2);
    m.insert(r1, 1).unwrap();
    m.insert(r2, 2).unwrap();
    m.insert(k(1, 3), 3).unwrap();
    m.insert(k(1, 4), 4).unwrap(); // steals r1's slot
    m.truncate(2);
    println!("len {} iter {:?}", m.len(), m.iter().collect::<std::vec::Vec<_>>());
    println!("get r1 {:?} get r2 {:?}", m.get(&r1), m.get(&r2));
    println!("insert r1 again -> {:?}, len {}", m.insert(r1, 100), m.len());
    println!("iter {:?}", m.iter().collect::<std::vec::Vec<_>>());
    assert_eq!(m.len(), 2);
}

#[test]
fn retain_panic_stale_indices() {
    let mut m: M<std::boxed::Box<u32>, 8> = IndexMap::default();
    for i in 0..4 { m.insert(k(i, i), Box::new(i)).unwrap(); }
    let mut n = 0;
    let _ = catch_unwind(AssertUnwindSafe(|| m.retain(|_, _| { n += 1; if n == 3 { panic!("boom") } n != 1 })));
    println!("len after panic {}", m.len());
    for i in 0..4 { println!("get {i} -> {:?}", m.get(&k(i, i))); }
}

#[test]
fn clear_panic_stale_indices() {
    struct P(bool);
    impl Drop for P { fn drop(&mut self) { if self.0 { panic!("boom") } } }
    let mut m: M<P, 8> = IndexMap::default();
    m.insert(k(1, 1), P(true)).ok().unwrap();
    m.insert(k(2, 2), P(false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| m.clear()));
    println!("len after panic {}", m.len());
    // force non-empty so find() proceeds, then look up a stale slot
    m.insert(k(3, 3), P(false)).ok().unwrap();
    println!("contains stale {}", m.contains_key(&k(2, 2)));
}

fn big<F: FnOnce() + Send + 'static>(f: F) { std::thread::Builder::new().stack_size(1 << 28).spawn(f).unwrap().join().unwrap(); }

#[test]
fn pos_zero_at_full_65536() {
    big(|| {
        let mut m: M<u8, 65536> = IndexMap::default();
        for i in 0..65535u32 { m.insert(k(i, i), 0).ok().unwrap(); }
        assert_eq!(m.len(), 65535);
        let key = k(0xffff, 7);
        println!("insert -> {:?}", m.insert(key, 1));
        println!("len {} get {:?}", m.len(), m.get(&key));
        assert_eq!(m.get(&key), Some(&1));
    });
}

#[test]
fn over_64k_capacity() {
    big(|| {
        let mut m: M<u32, 131072> = IndexMap::default();
        let mut lost = 0u32; let mut first = None;
        for i in 0..70000u32 {
            let key = i.rotate_left(16);
            m.insert(key, i).ok().unwrap();
            if m.get(&key) != Some(&i) { lost += 1; first.get_or_insert(i); }
        }
        println!("len {} not-found-right-after-insert {} first at index {:?}", m.len(), lost, first);
        assert_eq!(lost, 0);
    });
}

#[test]
fn clear_panic_uaf_string_key() {
    use heapless::index_map::FnvIndexMap;
    struct P(bool);
    impl Drop for P { fn drop(&mut self) { if self.0 { panic!("boom") } } }
    let mut m: FnvIndexMap<std::string::String, P, 8> = FnvIndexMap::new();
    m.insert("aaaaaaaa".to_string(), P(true)).ok().unwrap();
    m.insert("bbbbbbbb".to_string(), P(false)).ok().unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| m.clear()));
    m.insert("zz".to_string(), P(false)).ok().unwrap();
    println!("contains {}", m.contains_key("bbbbbbbb"));
}
