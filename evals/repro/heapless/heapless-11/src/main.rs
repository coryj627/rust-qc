fn big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> std::thread::Result<R> {
    // The map is built by value by the safe constructor, so give it room in the dev profile.
    std::thread::Builder::new().stack_size(128 << 20).spawn(f).unwrap().join()
}

fn fnv16(key: u32) -> u16 {
    use core::hash::BuildHasher;
    hash32::BuildHasherDefault::<hash32::FnvHasher>::default().hash_one(key) as u16
}

use heapless::index_map::FnvIndexMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const N: usize = 131072; // accepted: a power of two greater than 1, no upper limit

fn filled(entries: u32) -> Box<FnvIndexMap<u32, u32, N>> {
    let mut m = Box::new(FnvIndexMap::<u32, u32, N>::new());
    for k in 0..entries {
        m.insert(k, k).unwrap();
    }
    m
}

fn main() {
    let args: std::vec::Vec<String> = std::env::args().collect();
    let mode = args[1].clone();
    let n: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(0);
    if mode == "remove" {
        // Time one remove on a map of `n` entries, with a watchdog.
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new().stack_size(128 << 20).spawn(move || {
            let t = Instant::now();
            let mut m = filled(n);
            println!("entries={n}: fill took {:?}", t.elapsed());
            let t = Instant::now();
            let r = m.remove(&0);
            tx.send((r, t.elapsed())).unwrap();
            std::mem::forget(m);
        }).unwrap();
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok((r, e)) => println!("entries={n}: remove(&0)={r:?} took {e:?}"),
            Err(_) => { println!("entries={n}: remove(&0) did not return within 20s"); std::process::exit(3) }
        }
        return;
    }
    big_stack(move || match mode.as_str() {
        // Plain sequential keys, default hasher.
        "insert" => {
            let m = filled(N as u32);
            println!("inserted all, len={}", m.len());
        }
        // How many inserted keys can be read back?
        "lookup" => {
            let m = filled(n);
            let missing = (0..n).filter(|k| m.get(k) != Some(k)).count();
            println!("entries={n}: len={} keys that get() cannot return correctly: {missing}", m.len());
        }
        "where" => {
            let k = (0..N as u32).filter(|k| fnv16(*k) == 0xFFFF).collect::<std::vec::Vec<_>>();
            println!("sequential keys with hash 0xFFFF: {k:?}");
        }
        _ => panic!("bad mode"),
    }).map_err(|_| std::process::exit(101)).unwrap();
}
