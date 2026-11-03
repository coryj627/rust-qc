use heapless::index_map::FnvIndexMap;

fn main() {
    let mut map: FnvIndexMap<u16, u32, 4> = FnvIndexMap::new();
    map.insert(4, 444).unwrap();
    map.insert(2, 222).unwrap();
    map.insert(0, 0).unwrap();
    map.insert(7, 7).unwrap();
    assert_eq!(map.get(&2), Some(&222));

    map.truncate(2); // documented: keeps the first 2 elements

    let kept: std::vec::Vec<_> = map.iter().map(|(k, v)| (*k, *v)).collect();
    println!("after truncate(2): len={} iter={:?}", map.len(), kept);
    println!("get(&4)={:?} get(&2)={:?}", map.get(&4), map.get(&2));
    println!("contains_key(&2)={}", map.contains_key(&2));
    assert_eq!(kept, [(4, 444), (2, 222)]);
    // The key is in iter() so get() must find it.
    assert_eq!(map.get(&2), Some(&222), "key 2 is in iter() but get() cannot find it");
    println!("consistent");
}
