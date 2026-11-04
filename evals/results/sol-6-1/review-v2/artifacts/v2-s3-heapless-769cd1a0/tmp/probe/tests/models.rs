use heapless::{Deque, HistoryBuf, String as HString, Vec as HVec};
use heapless::index_map::FnvIndexMap;
use heapless::index_set::FnvIndexSet;
use std::collections::{BTreeSet, VecDeque};

#[test]
fn deque_ordinary_operations_match_vecdeque() {
    // Exhaust all five-operation sequences for every small capacity.
    fn check<const N: usize>() {
        for code in 0..6usize.pow(5) {
            let mut code = code;
            let mut a = Deque::<u8, N>::new();
            let mut b = VecDeque::new();
            for step in 0..5 {
                match code % 6 {
                    0 => { let r = a.push_front(step); if b.len() < N { b.push_front(step); assert!(r.is_ok()); } else { assert_eq!(r, Err(step)); } }
                    1 => { let r = a.push_back(step); if b.len() < N { b.push_back(step); assert!(r.is_ok()); } else { assert_eq!(r, Err(step)); } }
                    2 => assert_eq!(a.pop_front(), b.pop_front()),
                    3 => assert_eq!(a.pop_back(), b.pop_back()),
                    4 => { a.clear(); b.clear(); }
                    _ => { if !a.is_full() { assert_eq!(a.make_contiguous(), b.make_contiguous()); } }
                }
                assert_eq!(a.iter().copied().collect::<std::vec::Vec<_>>(), b.iter().copied().collect::<std::vec::Vec<_>>());
                assert_eq!(a.len(), b.len());
                assert_eq!(a.front(), b.front());
                assert_eq!(a.back(), b.back());
                code /= 6;
            }
        }
    }
    check::<1>(); check::<2>(); check::<3>(); check::<4>(); check::<5>();
}

#[test]
fn vec_drain_and_retain_match_std_including_small_length_type() {
    for n in 0..=8 {
        for start in 0..=n {
            for end in start..=n {
                for consumed in 0..=end-start {
                    let mut a = HVec::<usize, 8, u8>::from_iter(0..n);
                    let mut b = std::vec::Vec::from_iter(0..n);
                    let mut ad = a.drain(start..end);
                    let mut bd = b.drain(start..end);
                    for _ in 0..consumed { assert_eq!(ad.next(), bd.next()); }
                    drop(ad); drop(bd);
                    assert_eq!(a.as_slice(), b.as_slice());
                    a.retain_mut(|v| { *v += 1; *v % 2 == 0 });
                    b.retain_mut(|v| { *v += 1; *v % 2 == 0 });
                    assert_eq!(a.as_slice(), b.as_slice());
                }
            }
        }
    }
    let mut a = HVec::<u8, 255, u8>::from_iter(0..255);
    a.retain(|n| n % 2 == 0);
    assert_eq!(a.len(), 128);
    assert_eq!(a.pop(), Some(254));
}

#[test]
fn unicode_string_edits_match_std() {
    for initial in ["", "a", "é", "𝄞", "aé𝄞b"] {
        let boundaries = initial.char_indices().map(|(i,_)|i).chain([initial.len()]);
        for at in boundaries {
            for inserted in ["", "x", "ö", "💖"] {
                let mut a = HString::<32, u8>::try_from(initial).unwrap();
                let mut b = initial.to_owned();
                a.insert_str(at, inserted).unwrap(); b.insert_str(at, inserted);
                assert_eq!(a.as_str(), b);
                assert_eq!(a.pop(), b.pop());
                assert_eq!(a.as_str(), b);
            }
            for end in initial.char_indices().map(|(i,_)|i).chain([initial.len()]).filter(|e|*e>=at) {
                let mut a = HString::<32, u8>::try_from(initial).unwrap();
                let mut b = initial.to_owned();
                let ac: std::string::String = a.drain(at..end).collect();
                let bc: std::string::String = b.drain(at..end).collect();
                assert_eq!(ac, bc); assert_eq!(a.as_str(), b);
            }
        }
    }
}

#[test]
fn index_map_insert_remove_retain_match_reference() {
    for code in 0..8usize.pow(5) {
        let mut code = code;
        let mut a = FnvIndexMap::<usize, usize, 4>::new();
        let mut b = std::vec::Vec::new();
        for step in 0..5 {
            let op = code % 8;
            if op < 4 {
                let old = b.iter().position(|(k,_)| *k == op).map(|at|std::mem::replace(&mut b[at].1, step));
                if old.is_none() { b.push((op, step)); }
                assert_eq!(a.insert(op, step), Ok(old));
            } else if op < 7 {
                let key = op - 4;
                let old = b.iter().position(|(k,_)|*k==key).map(|at|b.swap_remove(at).1);
                assert_eq!(a.remove(&key), old);
            } else {
                a.retain(|k,_| *k % 2 == 0); b.retain(|(k,_)|*k%2==0);
            }
            assert_eq!(a.iter().map(|(k,v)|(*k,*v)).collect::<std::vec::Vec<_>>(), b);
            for key in 0..5 { assert_eq!(a.get(&key), b.iter().find(|(k,_)|*k==key).map(|(_,v)|v)); }
            code /= 8;
        }
    }
}

#[test]
fn set_operations_match_btree_set() {
    for left in 0..16 {
        for right in 0..16 {
            let a: FnvIndexSet<_,4> = (0..4).filter(|k|left & (1<<k)!=0).collect();
            let b: FnvIndexSet<_,4> = (0..4).filter(|k|right & (1<<k)!=0).collect();
            let ar: BTreeSet<_> = a.iter().copied().collect();
            let br: BTreeSet<_> = b.iter().copied().collect();
            assert_eq!(a.union(&b).copied().collect::<BTreeSet<_>>(), ar.union(&br).copied().collect());
            assert_eq!(a.intersection(&b).copied().collect::<BTreeSet<_>>(), ar.intersection(&br).copied().collect());
            assert_eq!(a.symmetric_difference(&b).copied().collect::<BTreeSet<_>>(), ar.symmetric_difference(&br).copied().collect());
            assert_eq!(a.difference(&b).copied().collect::<BTreeSet<_>>(), ar.difference(&br).copied().collect());
        }
    }
}

#[test]
fn history_keeps_last_values_in_order() {
    let mut h = HistoryBuf::<_, 3>::new();
    for value in 0..20 {
        h.write(value);
        assert_eq!(h.oldest_ordered().copied().collect::<std::vec::Vec<_>>(), (0..=value).rev().take(3).collect::<std::vec::Vec<_>>().into_iter().rev().collect::<std::vec::Vec<_>>());
        assert_eq!(h.recent(), Some(&value));
        assert_eq!(h.clone(), h);
    }
}

#[test]
fn queues_reuse_capacity_across_counter_wraparound() {
    let m = heapless::mpmc::Queue::<usize, 128>::new();
    let mut s = heapless::spsc::Queue::<usize, 7>::new();
    for round in 0..10 {
        for value in 0..128 { assert_eq!(m.enqueue(round*128+value), Ok(())); }
        assert_eq!(m.enqueue(9999), Err(9999));
        for value in 0..128 { assert_eq!(m.dequeue(), Some(round*128+value)); }
        assert_eq!(m.dequeue(), None);
        for value in 0..6 { assert_eq!(s.enqueue(round*6+value), Ok(())); }
        assert_eq!(s.enqueue(9999), Err(9999));
        for value in 0..6 { assert_eq!(s.dequeue(), Some(round*6+value)); }
        assert_eq!(s.dequeue(), None);
    }
}
