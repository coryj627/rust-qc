    #[test]
    fn truncate_preserves_collision_chains() {
        // Later insertions displace earlier entries in these collision chains.
        // The second chain wraps around the end of the table.
        for entries in [
            [(2_u8, 20), (1, 10), (5, 50), (6, 60)],
            [(0_u8, 0), (3, 30), (7, 70), (4, 40)],
        ] {
            for initial_len in 0..=entries.len() {
                for len in (0..=entries.len() + 1).chain(core::iter::once(usize::MAX)) {
                    let mut map = FnvIndexMap::<_, _, 4>::new();
                    for &(key, value) in &entries[..initial_len] {
                        map.insert(key, value).unwrap();
                    }

                    map.truncate(len);

                    let expected = &entries[..initial_len.min(len)];
                    assert_eq!(map.len(), expected.len());
                    assert!(map
                        .iter()
                        .map(|(&key, &value)| (key, value))
                        .eq(expected.iter().copied()));
                    for (index, (key, value)) in entries.iter().enumerate() {
                        let retained = index < expected.len();
                        assert_eq!(map.get(key), retained.then_some(value));
                        assert_eq!(map.contains_key(key), retained);
                    }
                }
            }
        }
    }

