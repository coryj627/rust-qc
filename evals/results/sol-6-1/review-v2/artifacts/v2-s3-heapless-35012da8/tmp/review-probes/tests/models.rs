use std::collections::VecDeque;

fn next(seed: &mut u64) -> u32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 32) as u32
}

fn deque_model<const N: usize>() {
    let mut actual = heapless::Deque::<u32, N>::new();
    let mut expected = VecDeque::new();
    let mut seed = 72;
    for _ in 0..10_000 {
        let value = next(&mut seed);
        match value % 8 {
            0 | 1 => {
                if expected.len() < N {
                    actual.push_back(value).unwrap();
                    expected.push_back(value);
                } else {
                    assert_eq!(actual.push_back(value), Err(value));
                }
            }
            2 | 3 => {
                if expected.len() < N {
                    actual.push_front(value).unwrap();
                    expected.push_front(value);
                } else {
                    assert_eq!(actual.push_front(value), Err(value));
                }
            }
            4 => assert_eq!(actual.pop_front(), expected.pop_front()),
            5 => assert_eq!(actual.pop_back(), expected.pop_back()),
            6 => assert_eq!(actual.make_contiguous(), expected.make_contiguous()),
            7 => {
                let index = (value / 8) as usize % (N + 1);
                assert_eq!(
                    actual.swap_remove_back(index),
                    expected.swap_remove_back(index)
                );
            }
            _ => unreachable!(),
        }
        assert_eq!(actual.storage_len(), expected.len());
        assert_eq!(
            actual.iter().copied().collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(actual.front(), expected.front());
        assert_eq!(actual.back(), expected.back());
        for i in 0..=N {
            assert_eq!(actual.get(i), expected.get(i));
        }
    }
}

#[test]
fn deque_wrap_and_contiguity() {
    deque_model::<1>();
    deque_model::<2>();
    deque_model::<3>();
    deque_model::<4>();
    deque_model::<5>();
    deque_model::<8>();
}

fn vec_model<const N: usize>() {
    let mut actual = heapless::Vec::<u32, N, u8>::new();
    let mut expected = Vec::new();
    let mut seed = 16;
    for _ in 0..10_000 {
        let value = next(&mut seed);
        match value % 7 {
            0 | 1 => {
                if expected.len() < N {
                    actual.push(value).unwrap();
                    expected.push(value);
                } else {
                    assert_eq!(actual.push(value), Err(value));
                }
            }
            2 => assert_eq!(actual.pop(), expected.pop()),
            3 => {
                let len = value as usize % (N + 2);
                let result = actual.resize(len, value);
                assert_eq!(result.is_ok(), len <= N);
                if len <= N {
                    expected.resize(len, value);
                }
            }
            4 => {
                actual.retain(|v| v % 3 != 0);
                expected.retain(|v| v % 3 != 0);
            }
            5 => {
                let a = value as usize % (expected.len() + 1);
                let b = a + (value / 8) as usize % (expected.len() + 1 - a);
                assert_eq!(
                    actual.drain(a..b).collect::<Vec<_>>(),
                    expected.drain(a..b).collect::<Vec<_>>()
                );
            }
            6 => {
                let index = value as usize % (expected.len() + 1);
                if expected.len() < N {
                    actual.insert(index, value).unwrap();
                    expected.insert(index, value);
                } else {
                    assert_eq!(actual.insert(index, value), Err(value));
                }
            }
            _ => unreachable!(),
        }
        assert_eq!(actual.as_slice(), expected);
    }
}

#[test]
fn vec_insert_resize_retain_drain() {
    vec_model::<0>();
    vec_model::<1>();
    vec_model::<4>();
    vec_model::<255>();
}

#[test]
fn unicode_string_edits() {
    for text in ["", "a", "é", "😀", "aé😀z", "αβγ"] {
        for a in (0..=text.len()).filter(|&a| text.is_char_boundary(a)) {
            for b in (a..=text.len()).filter(|&b| text.is_char_boundary(b)) {
                let mut actual = heapless::String::<32, u8>::try_from(text).unwrap();
                let mut expected = text.to_owned();
                assert_eq!(
                    actual.drain(a..b).collect::<String>(),
                    expected.drain(a..b).collect::<String>()
                );
                assert_eq!(actual.as_str(), expected);
            }
            let mut actual = heapless::String::<32, u8>::try_from(text).unwrap();
            let mut expected = text.to_owned();
            actual.insert(a, '😀').unwrap();
            expected.insert(a, '😀');
            actual.insert_str(a, "é").unwrap();
            expected.insert_str(a, "é");
            assert_eq!(actual.as_str(), expected);
            assert_eq!(actual.remove(a), expected.remove(a));
            assert_eq!(actual.as_str(), expected);
            while !expected.is_empty() {
                assert_eq!(actual.pop(), expected.pop());
            }
            assert_eq!(actual.pop(), None);
        }
    }
}

#[test]
fn history_wraparound() {
    let mut actual = heapless::HistoryBuf::<u32, 3>::new();
    let mut expected = VecDeque::new();
    for n in 0..100 {
        actual.write(n);
        if expected.len() == 3 {
            expected.pop_front();
        }
        expected.push_back(n);
        assert_eq!(
            actual.oldest_ordered().copied().collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(actual.recent(), expected.back());
        assert_eq!(actual.oldest(), expected.front());
        assert_eq!(actual.clone(), actual);
    }
    actual.clear();
    assert_eq!(actual.recent(), None);
    assert!(actual.is_empty());
}

#[test]
fn hash_map_operations() {
    let mut actual = heapless::index_map::FnvIndexMap::<u32, u32, 8>::new();
    let mut expected = Vec::<(u32, u32)>::new();
    let mut seed = 94;
    for _ in 0..10_000 {
        let value = next(&mut seed);
        let key = value % 24;
        match value % 5 {
            0 | 1 => {
                let old_index = expected.iter().position(|&(k, _)| k == key);
                let result = actual.insert(key, value);
                if let Some(index) = old_index {
                    assert_eq!(result, Ok(Some(expected[index].1)));
                    expected[index].1 = value;
                } else if expected.len() == 8 {
                    assert_eq!(result, Err((key, value)));
                } else {
                    assert_eq!(result, Ok(None));
                    expected.push((key, value));
                }
            }
            2 => {
                let removed = expected
                    .iter()
                    .position(|&(k, _)| k == key)
                    .map(|index| expected.swap_remove(index).1);
                assert_eq!(actual.remove(&key), removed);
            }
            3 => {
                actual.retain(|k, _| k % 3 != key % 3);
                expected.retain(|&(k, _)| k % 3 != key % 3);
            }
            _ => {
                actual.entry(key).and_modify(|v| *v += 1);
                if let Some((_, v)) = expected.iter_mut().find(|(k, _)| *k == key) {
                    *v += 1;
                }
            }
        }
        assert_eq!(
            actual.iter().map(|(&k, &v)| (k, v)).collect::<Vec<_>>(),
            expected
        );
        for k in 0..24 {
            assert_eq!(
                actual.get(&k),
                expected.iter().find(|(key, _)| *key == k).map(|(_, v)| v)
            );
        }
    }
}

#[test]
fn queues_fifo() {
    let mut spsc = heapless::spsc::Queue::<u32, 3>::new();
    let mpmc = heapless::mpmc::Queue::<u32, 4>::new();
    let mut expected_spsc = VecDeque::new();
    let mut expected_mpmc = VecDeque::new();
    let mut seed = 8;
    for _ in 0..10_000 {
        let value = next(&mut seed);
        if value % 2 == 0 {
            if expected_spsc.len() < 2 {
                spsc.enqueue(value).unwrap();
                expected_spsc.push_back(value);
            } else {
                assert_eq!(spsc.enqueue(value), Err(value));
            }
            if expected_mpmc.len() < 4 {
                mpmc.enqueue(value).unwrap();
                expected_mpmc.push_back(value);
            } else {
                assert_eq!(mpmc.enqueue(value), Err(value));
            }
        } else {
            assert_eq!(spsc.dequeue(), expected_spsc.pop_front());
            assert_eq!(mpmc.dequeue(), expected_mpmc.pop_front());
        }
        assert_eq!(spsc.len(), expected_spsc.len());
        assert_eq!(spsc.peek(), expected_spsc.front());
    }
}

#[test]
fn cstring_extension_matches_contract() {
    for len in 0..=5 {
        for code in 0..3usize.pow(len) {
            let mut code = code;
            let bytes: Vec<_> = (0..len)
                .map(|_| {
                    let b = [0, b'a', 0xff][code % 3];
                    code /= 3;
                    b
                })
                .collect();
            let mut actual = heapless::CString::<8>::from_bytes_with_nul(b"x\0").unwrap();
            let content = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
            let valid = !content.contains(&0);
            let result = actual.extend_from_bytes(&bytes);
            assert_eq!(result.is_ok(), valid, "{bytes:?}");
            if valid {
                let mut expected = vec![b'x'];
                expected.extend_from_slice(content);
                assert_eq!(actual.as_bytes(), expected);
            } else {
                assert_eq!(actual.as_bytes(), b"x");
            }
        }
    }
}

#[test]
fn serde_boundaries_and_duplicates() {
    use serde::{
        de::value::{Error, MapDeserializer, SeqDeserializer},
        Deserialize,
    };
    let seq = SeqDeserializer::<_, Error>::new([1u8, 2, 3].into_iter());
    let v = heapless::Vec::<u8, 3, u8>::deserialize(seq).unwrap();
    assert_eq!(v.as_slice(), [1, 2, 3]);
    let seq = SeqDeserializer::<_, Error>::new([1u8, 2, 3, 4].into_iter());
    assert!(heapless::Vec::<u8, 3, u8>::deserialize(seq).is_err());
    let seq = SeqDeserializer::<_, Error>::new([1u8, 2, 1, 2].into_iter());
    let set = heapless::index_set::FnvIndexSet::<u8, 2>::deserialize(seq).unwrap();
    assert_eq!(set.len(), 2);
    let entries = [(1u8, 1u8), (2, 2), (1, 7)];
    let map = heapless::index_map::FnvIndexMap::<u8, u8, 2>::deserialize(
        MapDeserializer::<_, Error>::new(entries.into_iter()),
    )
    .unwrap();
    assert_eq!(map.get(&1), Some(&7));
    assert_eq!(map.len(), 2);
}

#[test]
fn heap_priorities_and_mutable_peek() {
    use heapless::binary_heap::{BinaryHeap, Max};
    let mut actual = BinaryHeap::<u32, Max, 8>::new();
    let mut expected = std::collections::BinaryHeap::new();
    let mut seed = 81;
    for _ in 0..10_000 {
        let value = next(&mut seed);
        match value % 4 {
            0 | 1 => {
                if expected.len() < 8 { actual.push(value).unwrap(); expected.push(value); }
                else { assert_eq!(actual.push(value), Err(value)); }
            }
            2 => assert_eq!(actual.pop(), expected.pop()),
            _ => {
                if let Some(mut top) = actual.peek_mut() { *top = value / 4; }
                if let Some(mut top) = expected.peek_mut() { *top = value / 4; }
            }
        }
        assert_eq!(actual.len(), expected.len());
        assert_eq!(actual.peek(), expected.peek());
    }
    while !expected.is_empty() { assert_eq!(actual.pop(), expected.pop()); }
}

#[test]
fn sorted_priorities_and_guard_finish() {
    use heapless::sorted_linked_list::{Max, SortedLinkedList};
    let mut actual = SortedLinkedList::<u32, Max, 4, u8>::new_u8();
    let mut expected = Vec::<u32>::new();
    let mut seed = 71;
    for _ in 0..10_000 {
        let value = next(&mut seed);
        match value % 4 {
            0 | 1 => {
                if expected.len() < 4 { actual.push(value).unwrap(); expected.push(value); }
                else { assert_eq!(actual.push(value), Err(value)); }
            }
            2 => { expected.sort_unstable(); assert_eq!(actual.pop(), expected.pop()); }
            _ => {
                if !expected.is_empty() {
                    let index = value as usize % expected.len();
                    let key = expected[index];
                    let mut guard = actual.find_mut(|v| *v == key).unwrap();
                    *guard = value;
                    guard.finish();
                    expected[index] = value;
                }
            }
        }
        expected.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(actual.iter().copied().collect::<Vec<_>>(), expected);
        assert_eq!(actual.peek(), expected.first());
    }
}
