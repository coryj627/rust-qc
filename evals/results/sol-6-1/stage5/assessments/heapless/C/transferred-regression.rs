    #[test]
    fn truncate_collisions() {
        // These u8 keys hash to alternating buckets 0 and 1, or 7 and 0,
        // respectively. Later insertions displace earlier entries, including
        // across the end of the index array in the second case.
        for keys in [
            [5u8, 6, 13, 14, 21, 22, 29, 30],
            [0u8, 5, 8, 13, 16, 21, 24, 29],
        ] {
            for initial_len in 0..=keys.len() {
                for len in (0..=keys.len() + 1).chain(core::iter::once(usize::MAX)) {
                    let mut map = FnvIndexMap::<_, _, 8>::new();
                    for (i, &key) in keys[..initial_len].iter().enumerate() {
                        map.insert(key, 100 + i).unwrap();
                    }

                    map.truncate(len);

                    let retained = initial_len.min(len);
                    assert_eq!(map.len(), retained);
                    assert!(map.iter().map(|(&k, &v)| (k, v)).eq(keys[..retained]
                        .iter()
                        .enumerate()
                        .map(|(i, &k)| (k, 100 + i))));
                    for (i, key) in keys.iter().enumerate() {
                        let expected = if i < retained { Some(100 + i) } else { None };
                        assert_eq!(map.get(key).copied(), expected);
                        assert_eq!(map.contains_key(key), expected.is_some());
                    }

                    // Retained keys must update in place, and removed keys must
                    // be insertable again without stale indices or duplicates.
                    for (i, &key) in keys.iter().enumerate() {
                        let expected = if i < retained { Some(100 + i) } else { None };
                        assert_eq!(map.insert(key, 200 + i), Ok(expected));
                    }
                    assert_eq!(map.len(), keys.len());
                    assert!(map
                        .iter()
                        .map(|(&k, &v)| (k, v))
                        .eq(keys.iter().enumerate().map(|(i, &k)| (k, 200 + i))));
                    for (i, key) in keys.iter().enumerate() {
                        assert_eq!(map.get(key), Some(&(200 + i)));
                        assert!(map.contains_key(key));
                    }
                }
            }
        }
    }

