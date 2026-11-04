use heapless::index_map::FnvIndexMap;
#[test]
fn large_map_retrieves_all_inserted_keys() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let mut m = FnvIndexMap::<u32, u32, 131072>::new();
        for i in 0..=65536 { m.insert(i, i).unwrap(); }
        assert_eq!(m.len(), 65537);
        for i in 0..=65536 { assert_eq!(m.get(&i), Some(&i), "key {i}"); }
    }).unwrap().join().unwrap();
}
