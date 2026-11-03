use heapless::{binary_heap::{BinaryHeap, Max, Min}, spsc::Queue, CString, LinearMap, String as HString, Vec as HVec};
use std::{cell::Cell, rc::Rc, sync::atomic::{AtomicUsize, Ordering}};

#[test]
fn decoy_1_overlapping_string_insert() {
    let mut cases = 0;
    for original in ["abcdef", "aé日🦀z", "", "ééé"] {
        for idx in (0..=original.len()).filter(|&i| original.is_char_boundary(i)) {
            for inserted in ["X", "é", "日🦀", ""] {
                let mut actual = HString::<64>::try_from(original).unwrap();
                let mut expected = original.to_owned();
                actual.insert_str(idx, inserted).unwrap();
                expected.insert_str(idx, inserted);
                assert_eq!(actual.as_str(), expected);
                cases += 1;
            }
        }
    }
    let mut exact = HString::<7>::try_from("abcdef").unwrap();
    exact.insert_str(1, "X").unwrap();
    assert_eq!(exact.as_str(), "aXbcdef");
    assert_eq!(exact.len(), exact.capacity());
    println!("decoy-1: {cases} boundary/UTF-8 insertion cases match std; exact-fit overlapping insertion = {exact}");
}

#[test]
fn decoy_2_replace_in_full_linear_map() {
    let mut map = LinearMap::<u8, u16, 2>::new();
    assert_eq!(map.insert(1, 10), Ok(None));
    assert_eq!(map.insert(2, 20), Ok(None));
    assert!(map.is_full());
    assert_eq!(map.insert(2, 99), Ok(Some(20)));
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&1), Some(&10));
    assert_eq!(map.get(&2), Some(&99));
    assert_eq!(map.insert(3, 30), Err((3, 30)));
    println!("decoy-2: full map replacement returned Ok(Some(20)), stored 99, retained len=2; new key correctly rejected");
}

#[derive(Debug)]
struct CountClone { id: u8, calls: Rc<Cell<usize>> }
impl Clone for CountClone {
    fn clone(&self) -> Self {
        self.calls.set(self.calls.get() + 1);
        Self { id: self.id, calls: self.calls.clone() }
    }
}

#[test]
fn false_1_rejected_extension_is_atomic() {
    let calls = Rc::new(Cell::new(0));
    let mut v = HVec::<CountClone, 3>::new();
    for id in [1, 2] { v.push(CountClone { id, calls: calls.clone() }).unwrap(); }
    let other = [CountClone { id: 3, calls: calls.clone() }, CountClone { id: 4, calls: calls.clone() }];
    assert!(v.extend_from_slice(&other).is_err());
    assert_eq!(v.len(), 2);
    assert_eq!(v.iter().map(|x| x.id).collect::<std::vec::Vec<_>>(), [1, 2]);
    assert_eq!(calls.get(), 0);
    v.extend_from_slice(&other[..1]).unwrap();
    assert_eq!(v.len(), 3);
    assert_eq!(v[2].id, 3);
    assert_eq!(calls.get(), 1);
    println!("false-1: rejected 2-item extension with 1 free slot: len=2, contents=[1,2], clone calls=0; exact-fit extension succeeded");
}

#[test]
fn false_2_peek_mut_repairs_heap() {
    let mut max = BinaryHeap::<i32, Max, 8>::new();
    let mut min = BinaryHeap::<i32, Min, 8>::new();
    for x in [9, 7, 8, 3, 5, 6] { max.push(x).unwrap(); min.push(x).unwrap(); }
    { let mut root = max.peek_mut().unwrap(); *root = 0; }
    { let mut root = min.peek_mut().unwrap(); *root = 10; }
    assert_eq!(max.peek(), Some(&8));
    assert_eq!(min.peek(), Some(&5));
    let max_order = std::iter::from_fn(|| max.pop()).collect::<std::vec::Vec<_>>();
    let min_order = std::iter::from_fn(|| min.pop()).collect::<std::vec::Vec<_>>();
    assert_eq!(max_order, [8, 7, 6, 5, 3, 0]);
    assert_eq!(min_order, [5, 6, 7, 8, 9, 10]);
    println!("false-2: after ordinary guard drop, Max pop order={max_order:?}; Min pop order={min_order:?}");
}

static ZST_DROPS: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug)]
struct Zst;
impl Drop for Zst { fn drop(&mut self) { ZST_DROPS.fetch_add(1, Ordering::Relaxed); } }

#[test]
fn false_3_drain_handles_zero_sized_elements() {
    assert_eq!(std::mem::size_of::<Zst>(), 0);
    ZST_DROPS.store(0, Ordering::Relaxed);
    let mut v = HVec::<Zst, 8>::new();
    for _ in 0..6 { v.push(Zst).unwrap(); }
    {
        let mut drain = v.drain(1..5);
        drop(drain.next().unwrap());
        drop(drain.next_back().unwrap());
        assert_eq!(ZST_DROPS.load(Ordering::Relaxed), 2);
        // The two unconsumed zero-sized elements are dropped by Drain::drop.
    }
    assert_eq!(v.len(), 2);
    assert_eq!(ZST_DROPS.load(Ordering::Relaxed), 4);
    drop(v);
    assert_eq!(ZST_DROPS.load(Ordering::Relaxed), 6);
    println!("false-3: partial double-ended ZST drain completed without panic; len=2; drop counts 2 consumed, 4 after drain, 6 after vector");
}

fn queue_wrap<const N: usize>() {
    let mut queue = Queue::<u32, N>::new();
    let mut expected = std::collections::VecDeque::new();
    let (mut producer, mut consumer) = queue.split();
    let mut serial = 0;
    for _ in 0..100 {
        while expected.len() < N - 1 {
            producer.enqueue(serial).unwrap();
            expected.push_back(serial);
            serial += 1;
        }
        assert_eq!(producer.enqueue(u32::MAX), Err(u32::MAX));
        assert_eq!(producer.len(), expected.len());
        assert_eq!(consumer.len(), expected.len());
        for _ in 0..2.min(N - 1) {
            assert_eq!(consumer.dequeue(), expected.pop_front());
        }
    }
    while let Some(value) = expected.pop_front() { assert_eq!(consumer.dequeue(), Some(value)); }
    assert_eq!(consumer.dequeue(), None);
    assert_eq!(producer.len(), 0);
}

#[test]
fn false_4_non_power_of_two_queue_wraps() {
    queue_wrap::<3>();
    queue_wrap::<5>();
    queue_wrap::<6>();
    queue_wrap::<7>();
    println!("false-4: split queues N=3,5,6,7 each passed 100 fill/reject/dequeue cycles across repeated wraps, matching VecDeque FIFO and len");
}

#[test]
fn false_5_cstring_rejects_interior_nul() {
    for bytes in [&b"a\0b\0"[..], &b"\0b\0"[..], &b"a\0b"[..]] {
        assert!(matches!(CString::<16>::from_bytes_with_nul(bytes), Err(heapless::c_string::ExtendError::InteriorNul { .. })));
    }
    let valid = CString::<4>::from_bytes_with_nul(b"abc\0").unwrap();
    assert_eq!(valid.as_bytes_with_nul(), b"abc\0");
    let mut existing = CString::<16>::from_bytes_with_nul(b"ok\0").unwrap();
    assert!(matches!(existing.extend_from_bytes(b"a\0b\0"), Err(heapless::c_string::ExtendError::InteriorNul { position: 1 })));
    assert_eq!(existing.as_bytes_with_nul(), b"ok\0");
    println!("false-5: safe CString constructor rejected 3 interior-NUL inputs; append rejected interior NUL at position=1 and retained ok\\0; exact-fit valid abc\\0 accepted");
}
