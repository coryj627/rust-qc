#[test]
fn map_capacity_above_u16_keeps_all_insertions_findable() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut map = heapless::index_map::FnvIndexMap::<u32, u32, 131072>::new();
            for key in 0..65537u32 {
                map.insert(key, key).unwrap();
            }
            assert_eq!(map.len(), 65537);
            assert_eq!(map.get(&65536), Some(&65536));
        })
        .unwrap()
        .join()
        .unwrap();
}
