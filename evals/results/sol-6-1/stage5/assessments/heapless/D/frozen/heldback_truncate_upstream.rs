use heapless::index_map::FnvIndexMap;

    /// See <https://github.com/rust-embedded/heapless/issues/674>
    #[test]
    fn truncate_invalid() {
        let mut map: FnvIndexMap<u16, u32, 4> = FnvIndexMap::new();
        map.insert(4, 444).unwrap();
        map.insert(2, 222).unwrap();
        map.insert(0, 0).unwrap();
        map.insert(7, 7).unwrap();

        assert_eq!(map.get(&2), Some(&222)); // ok before map.truncate(2);
        map.truncate(2);
        assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&4, &444), (&2, &222)]); // ok
        assert_eq!(map.get(&4), Some(&444)); // ok
        assert_eq!(map.get(&2), Some(&222)); // <-- key present in iter() but unreachable
    }
