    #[test]
    fn truncate_preserves_colliding_entries() {
        // Later insertions displace earlier entries in these FNV probe chains.
        // The second chain wraps around the end of the table.
        for keys in [
            &[][..],
            &[5_u8, 6, 13][..],
            &[7, 0, 15][..],
            &[5, 6, 13, 21, 29, 37, 45, 53][..],
        ] {
            for len in (0..=keys.len() + 1).chain(core::iter::once(usize::MAX)) {
                let mut map = FnvIndexMap::<_, _, 8>::new();
                for &key in keys {
                    map.insert(key, u16::from(key) + 100).unwrap();
                }

                map.truncate(len);

                let retained = keys.len().min(len);
                assert_eq!(map.len(), retained);
                assert!(map
                    .iter()
                    .map(|(&key, &value)| (key, value))
                    .eq(keys[..retained]
                        .iter()
                        .map(|&key| (key, u16::from(key) + 100))));
                for (index, key) in keys.iter().enumerate() {
                    let expected = (index < retained).then(|| u16::from(*key) + 100);
                    assert_eq!(map.get(key).copied(), expected);
                    assert_eq!(map.contains_key(key), expected.is_some());
                }

                // Replacing retained entries must not insert duplicate keys.
                for &key in &keys[..retained] {
                    assert_eq!(map.insert(key, 200), Ok(Some(u16::from(key) + 100)));
                }
                assert_eq!(map.len(), retained);

                // Removed slots must be reusable, including after truncating to zero.
                for &key in &keys[retained..] {
                    assert_eq!(map.insert(key, u16::from(key) + 100), Ok(None));
                }
                assert_eq!(map.len(), keys.len());
                assert!(map.keys().copied().eq(keys.iter().copied()));
                for (index, key) in keys.iter().enumerate() {
                    let expected = if index < retained {
                        200
                    } else {
                        u16::from(*key) + 100
                    };
                    assert_eq!(map.get(key), Some(&expected));
                    assert!(map.contains_key(key));
                }
            }
        }
    }

