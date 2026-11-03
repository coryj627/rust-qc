use heapless::index_map::{FnvIndexMap, IndexMap};
use std::hash::{BuildHasherDefault, Hasher};

// All insertion orders of the upstream key set, all populated prefixes, and all
// requested length classes. Expectations come solely from insertion order.
#[test]
fn fnv_truncation_matrix() {
    let keys = [4_u16, 2, 0, 7];
    for &a in &keys {
        for &b in &keys {
            for &c in &keys {
                for &d in &keys {
                    let order = [a, b, c, d];
                    if (0..4).any(|i| (i + 1..4).any(|j| order[i] == order[j])) {
                        continue;
                    }
                    for populated in 0..=4 {
                        for requested in [0, 1, 2, 3, 4, 5, 6, usize::MAX] {
                            let mut map = FnvIndexMap::<u16, String, 4>::new();
                            let original: Vec<_> = order[..populated]
                                .iter()
                                .map(|&key| (key, format!("value-{key}")))
                                .collect();
                            for (key, value) in &original {
                                map.insert(*key, value.clone()).unwrap();
                            }
                            map.truncate(requested);
                            let kept = populated.min(requested);
                            let expected = &original[..kept];
                            assert_eq!(map.len(), kept, "{order:?}, {populated}, {requested}");
                            assert_eq!(map.is_empty(), kept == 0);
                            assert_eq!(map.iter().map(|(&k, v)| (k, v.clone())).collect::<Vec<_>>(), expected);
                            assert_eq!(map.keys().copied().collect::<Vec<_>>(), expected.iter().map(|(k, _)| *k).collect::<Vec<_>>());
                            assert_eq!(map.values().collect::<Vec<_>>(), expected.iter().map(|(_, v)| v).collect::<Vec<_>>());
                            for key in &keys {
                                let wanted = expected.iter().find(|(k, _)| k == key).map(|(_, v)| v);
                                assert_eq!(map.get(key), wanted, "key {key}, {order:?}, {populated}, {requested}");
                                assert_eq!(map.contains_key(key), wanted.is_some());
                            }
                            // Applying an equal length again must preserve the same state.
                            map.truncate(kept);
                            assert_eq!(map.len(), kept);
                            assert_eq!(map.iter().map(|(&k, v)| (k, v.clone())).collect::<Vec<_>>(), expected);
                            for (key, value) in expected {
                                assert_eq!(map.get(key), Some(value));
                                assert!(map.contains_key(key));
                            }
                        }
                    }
                }
            }
        }
    }
}

// Public generic API: keys and values need neither Copy nor Clone. All hashes
// collide, including a chain that crosses the end of the table.
#[derive(Debug, Eq, PartialEq, Hash)]
struct Key(u8);

#[derive(Debug, Eq, PartialEq)]
struct Value(String);

#[derive(Default)]
struct CollisionHasher;

impl Hasher for CollisionHasher {
    fn write(&mut self, _: &[u8]) {}
    fn finish(&self) -> u64 { u64::MAX }
}

#[test]
fn custom_hasher_nonclone_entries_and_repeated_truncation() {
    for requested in [0, 1, 3, 7, 8, 9, usize::MAX] {
        let mut map: IndexMap<Key, Value, BuildHasherDefault<CollisionHasher>, 8> =
            IndexMap::default();
        for key in 0..8_u8 {
            map.insert(Key(key), Value(format!("owned-{key}"))).unwrap();
        }
        let mut remaining = 8;
        for len in [requested, requested, 3, 1, 0] {
            map.truncate(len);
            remaining = remaining.min(len);
            assert_eq!(map.len(), remaining);
            assert_eq!(map.iter().map(|(k, v)| (k.0, v.0.as_str())).collect::<Vec<_>>(),
                (0..remaining).map(|i| (i as u8, format!("owned-{i}"))).collect::<Vec<_>>()
                    .iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>());
            for key in 0..8_u8 {
                assert_eq!(map.contains_key(&Key(key)), (key as usize) < remaining);
                if (key as usize) < remaining {
                    assert_eq!(map.get(&Key(key)).unwrap().0, format!("owned-{key}"));
                } else {
                    assert_eq!(map.get(&Key(key)), None);
                }
            }
        }
    }
}
