use heapless::index_map::FnvIndexMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn main() {
    let mut m: FnvIndexMap<Box<u64>, u8, 8> = FnvIndexMap::new();
    for (i, k) in [10u64, 20, 30, 40, 50, 60].iter().enumerate() {
        m.insert(Box::new(*k), i as u8).unwrap();
    }
    let r = catch_unwind(AssertUnwindSafe(|| {
        m.retain(|k, _| match **k {
            10 => true,
            20 => false,
            _ => panic!("user predicate panic"),
        })
    }));
    println!("retain panicked: {}, len: {}", r.is_err(), m.len());
    // entries shifted down but the index slots still hold the old positions.
    let removed = m.remove(&Box::new(60));
    println!("remove(60) = {removed:?}, len: {}", m.len());
    // Key 50's stale slot says position 4, past the end, holding a freed copy of the box of 60.
    println!("get(50) = {:?}", m.get(&Box::new(50)));
    println!("no UB");
}
