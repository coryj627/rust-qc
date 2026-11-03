use core::hash::Hasher;

/// Public-API hasher that makes the hash of a u32 or u16 key equal to the key.
#[derive(Default)]
struct IdentityHasher(u64);
impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, _b: &[u8]) { unimplemented!() }
    fn write_u16(&mut self, i: u16) { self.0 = i as u64; }
    fn write_u32(&mut self, i: u32) { self.0 = i as u64; }
}
type Ident = hash32::BuildHasherDefault<IdentityHasher>;

fn big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> std::thread::Result<R> {
    // The map is built by value by the safe constructor, so give it room in the dev profile.
    std::thread::Builder::new().stack_size(128 << 20).spawn(f).unwrap().join()
}

fn fnv16(key: u32) -> u16 {
    use core::hash::BuildHasher;
    hash32::BuildHasherDefault::<hash32::FnvHasher>::default().hash_one(key) as u16
}

use heapless::index_map::{FnvIndexMap, IndexMap};

fn main() {
    let mode = std::env::args().nth(1).expect("mode");
    big_stack(move || match mode.as_str() {
        // Keys hash to themselves; key 0xFFFF is inserted last, at index 0xFFFF.
        "identity" => {
            let mut m = Box::new(IndexMap::<u16, u16, Ident, 65536>::new());
            for k in 0..=u16::MAX {
                m.insert(k, k).unwrap();
            }
            println!("len={} get(0xFFFF)={:?}", m.len(), m.get(&0xFFFF));
            assert_eq!(m.get(&0xFFFF), Some(&0xFFFF), "last inserted key is unreachable");
        }
        // The default FNV hasher, with a key chosen to hash to 0xFFFF inserted last.
        "fnv" => {
            let special = (0u32..).find(|k| fnv16(*k) == 0xFFFF).unwrap();
            println!("key {special} has an FNV hash ending in 0xFFFF");
            let mut m = Box::new(FnvIndexMap::<u32, u32, 65536>::new());
            for k in (0u32..).filter(|k| *k != special).take(65535) {
                m.insert(k, k).unwrap();
            }
            println!("inserted 65535 entries; inserting the 65536th");
            m.insert(special, special).unwrap();
            println!("len={} get(special)={:?}", m.len(), m.get(&special));
            assert_eq!(m.get(&special), Some(&special), "last inserted key is unreachable");
        }
        _ => panic!("bad mode"),
    })
    .map_err(|_| std::process::exit(101))
    .unwrap();
    println!("consistent");
}
