    #[test]
    fn truncate_with_collisions() {
        // Map each u8 key directly to its hash so the probe chains are predictable.
        #[derive(Default)]
        struct IdentityHasher(u64);

        impl core::hash::Hasher for IdentityHasher {
            fn finish(&self) -> u64 {
                self.0
            }

            fn write(&mut self, bytes: &[u8]) {
                assert_eq!(bytes.len(), 1);
                self.0 = u64::from(bytes[0]);
            }
        }

        // The third key displaces the first key in each case. Truncating to two
        // entries must close that hole, including when the chain wraps around.
        for keys in [[1u8, 0, 8, 16, 2, 10, 3, 11], [0, 7, 15, 23, 1, 9, 6, 14]] {
            for initial_len in 0..=keys.len() {
                for len in (0..=keys.len() + 1).chain(core::iter::once(usize::MAX)) {
                    let mut map = IndexMap::<_, _, BuildHasherDefault<IdentityHasher>, 8>::new();
                    for &key in &keys[..initial_len] {
                        map.insert(key, u16::from(key) + 100).unwrap();
                    }

                    map.truncate(len);

                    let retained_len = len.min(initial_len);
                    assert_eq!(map.len(), retained_len);
                    assert_eq!(map.is_empty(), retained_len == 0);
                    assert!(map.iter().map(|(&k, &v)| (k, v)).eq(keys[..retained_len]
                        .iter()
                        .map(|&k| (k, u16::from(k) + 100))));
                    for (index, key) in keys.iter().enumerate() {
                        let expected = if index < retained_len {
                            Some(u16::from(*key) + 100)
                        } else {
                            None
                        };
                        assert_eq!(map.get(key).copied(), expected);
                        assert_eq!(map.contains_key(key), expected.is_some());
                    }
                }
            }
        }
    }

