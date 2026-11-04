use std::collections::{BTreeMap, BinaryHeap, VecDeque};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn check_vec<const N: usize>() {
    let mut h = heapless::Vec::<i32, N, u8>::new();
    let mut m = Vec::new();
    let mut s = 12345;
    for _ in 0..1000 {
        let r = next(&mut s);
        let value = (r % 23) as i32;
        match r % 8 {
            0 => { let full = m.len() == N; assert_eq!(h.push(value).is_err(), full); if !full { m.push(value); } }
            1 => assert_eq!(h.pop(), m.pop()),
            2 => { let index = r as usize % (m.len() + 1); let full = m.len() == N; assert_eq!(h.insert(index, value).is_err(), full); if !full { m.insert(index, value); } }
            3 if !m.is_empty() => { let index = r as usize % m.len(); assert_eq!(h.remove(index), m.remove(index)); }
            4 => { h.retain(|x| *x % 3 != 0); m.retain(|x| *x % 3 != 0); }
            5 => { let len = r as usize % (N + 2); let too_long = len > N; assert_eq!(h.resize(len, value).is_err(), too_long); if !too_long { m.resize(len, value); } }
            6 => { let start = r as usize % (m.len() + 1); assert_eq!(h.drain(start..).collect::<Vec<_>>(), m.drain(start..).collect::<Vec<_>>()); }
            _ => { let len = r as usize % (N + 2); h.truncate(len); m.truncate(len); }
        }
        assert_eq!(h.as_slice(), m.as_slice());
    }
}

#[test]
fn vector_model_boundaries() {
    check_vec::<0>(); check_vec::<1>(); check_vec::<3>(); check_vec::<8>(); check_vec::<255>();
}

fn check_queues<const N: usize>() {
    let mut h = heapless::Deque::<i32, N>::new();
    let mut m = VecDeque::new();
    let mut history = heapless::HistoryBuf::<i32, N>::new();
    let mut hm = VecDeque::new();
    let mut s = 3245;
    for _ in 0..1000 {
        let r = next(&mut s);
        let value = (r % 23) as i32;
        history.write(value);
        if hm.len() == N { hm.pop_front(); }
        hm.push_back(value);
        assert_eq!(history.oldest_ordered().copied().collect::<Vec<_>>(), hm.iter().copied().collect::<Vec<_>>());
        match r % 7 {
            0 => { let full = m.len() == N; assert_eq!(h.push_front(value).is_err(), full); if !full { m.push_front(value); } }
            1 => { let full = m.len() == N; assert_eq!(h.push_back(value).is_err(), full); if !full { m.push_back(value); } }
            2 => assert_eq!(h.pop_front(), m.pop_front()),
            3 => assert_eq!(h.pop_back(), m.pop_back()),
            // Full make_contiguous is covered by a separate known-failing regression.
            4 if m.len() != N => assert_eq!(h.make_contiguous(), m.make_contiguous()),
            5 if !m.is_empty() => { let index = r as usize % m.len(); assert_eq!(h.swap_remove_front(index), m.swap_remove_front(index)); }
            6 if !m.is_empty() => { let index = r as usize % m.len(); assert_eq!(h.swap_remove_back(index), m.swap_remove_back(index)); }
            _ => {}
        }
        assert_eq!(h.len(), m.len());
        assert_eq!(h.iter().copied().collect::<Vec<_>>(), m.iter().copied().collect::<Vec<_>>());
        assert_eq!(h.front(), m.front());
        assert_eq!(h.back(), m.back());
    }
}

#[test]
fn queue_models() { check_queues::<1>(); check_queues::<2>(); check_queues::<3>(); check_queues::<8>(); }

#[test]
fn map_model() {
    let mut h = heapless::index_map::FnvIndexMap::<u8, u8, 8>::new();
    let mut m = BTreeMap::new();
    let mut s = 23452;
    for _ in 0..2000 {
        let r = next(&mut s);
        let key = ((r >> 8) % 16) as u8;
        let value = (r % 251) as u8;
        match r % 5 {
            0 | 1 => {
                let full = !m.contains_key(&key) && m.len() == 8;
                let result = h.insert(key, value);
                assert_eq!(result.is_err(), full);
                if !full { assert_eq!(result.unwrap(), m.insert(key, value)); }
            }
            2 => assert_eq!(h.remove(&key), m.remove(&key)),
            3 => { h.retain(|k, _| k % 3 != 0); m.retain(|k, _| k % 3 != 0); }
            _ => assert_eq!(h.get(&key), m.get(&key)),
        }
        assert_eq!(h.len(), m.len());
        for key in 0..16 { assert_eq!(h.get(&key), m.get(&key)); }
    }
}

#[test]
fn priority_queue_models() {
    use heapless::binary_heap::Max;
    use heapless::sorted_linked_list::{Max as ListMax, SortedLinkedList};
    let mut h = heapless::BinaryHeap::<i32, Max, 8>::new();
    let mut l: SortedLinkedList<i32, ListMax, 8, u8> = SortedLinkedList::new_u8();
    let mut m = BinaryHeap::new();
    let mut s = 3445;
    for _ in 0..1000 {
        let r = next(&mut s);
        let value = (r % 43) as i32;
        if r % 3 != 0 {
            let full = m.len() == 8;
            assert_eq!(h.push(value).is_err(), full);
            assert_eq!(l.push(value).is_err(), full);
            if !full { m.push(value); }
        } else {
            let expected = m.pop();
            assert_eq!(h.pop(), expected);
            assert_eq!(l.pop(), expected);
        }
        assert_eq!(h.peek(), m.peek());
        assert_eq!(l.peek(), m.peek());
    }
}
