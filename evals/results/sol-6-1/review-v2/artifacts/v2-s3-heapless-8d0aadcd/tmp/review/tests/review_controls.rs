use heapless::{Deque, HistoryBuf, LinearMap, String, Vec};
use heapless::binary_heap::{BinaryHeap, Max, Min};
use heapless::sorted_linked_list::{SortedLinkedList, Max as ListMax};
use serde::{Deserialize, Serialize};
use serde::de::value::{Error, MapDeserializer, SeqDeserializer, StrDeserializer};

#[test]
fn serde_deserialization_boundaries_and_duplicates() {
    let v = Vec::<u8, 3>::deserialize(SeqDeserializer::<_, Error>::new([1u8,2,3].into_iter())).unwrap();
    assert_eq!(v.as_slice(), [1,2,3]);
    assert!(Vec::<u8, 2>::deserialize(SeqDeserializer::<_, Error>::new([1u8,2,3].into_iter())).is_err());
    let empty = Vec::<u8, 0>::deserialize(SeqDeserializer::<_, Error>::new(std::iter::empty::<u8>())).unwrap();
    assert!(empty.is_empty());
    let map = LinearMap::<u8,u8,1>::deserialize(MapDeserializer::<_, Error>::new([(1u8,2u8),(1,3)].into_iter())).unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&3));
    let history = HistoryBuf::<u8,2>::deserialize(SeqDeserializer::<_, Error>::new([1u8,2,3].into_iter())).unwrap();
    assert_eq!(history.oldest_ordered().copied().collect::<std::vec::Vec<_>>(), [2,3]);
    let string = String::<4>::deserialize(StrDeserializer::<Error>::new("💖")).unwrap();
    assert_eq!(string, "💖");
    assert!(String::<3>::deserialize(StrDeserializer::<Error>::new("💖")).is_err());
}

#[test]
fn serialize_available_for_other_views() {
    fn serializable<T: Serialize + ?Sized>(_: &T) {}
    let string = String::<4>::new();
    let deque = Deque::<u8,4>::new();
    let heap = BinaryHeap::<u8,Max,4>::new();
    let history = HistoryBuf::<u8,4>::new();
    let linear = LinearMap::<u8,u8,4>::new();
    serializable(string.as_view());
    serializable(deque.as_view());
    serializable(heap.as_view());
    serializable(history.as_view());
    serializable(linear.as_view());
}

#[test]
fn narrow_length_maximum_capacity() {
    let mut vec = Vec::<u16, 255, u8>::new();
    for i in 0..255 { vec.push(i).unwrap(); }
    assert_eq!(vec.len(), 255);
    assert_eq!(vec.push(999), Err(999));
    vec.retain(|n| n % 3 == 0);
    assert_eq!(vec.len(), 85);
    assert_eq!(vec.into_iter().collect::<std::vec::Vec<_>>(), (0..255).filter(|n| n%3==0).collect::<std::vec::Vec<_>>());
}

#[test]
fn string_operations_match_std_at_character_boundaries() {
    for text in ["", "abc", "αβγ", "a💖z"] {
        let offsets = text.char_indices().map(|(i,_)| i).chain([text.len()]).collect::<std::vec::Vec<_>>();
        for &start in &offsets {
            let mut ours = String::<64>::try_from(text).unwrap();
            let mut reference = text.to_owned();
            ours.insert_str(start, "💖δ").unwrap();
            reference.insert_str(start, "💖δ");
            assert_eq!(ours.as_str(), reference);
            assert_eq!(ours.remove(start), reference.remove(start));
            assert_eq!(ours.as_str(), reference);
            for &end in offsets.iter().filter(|&&end| end >= start) {
                let mut ours = String::<64>::try_from(text).unwrap();
                let mut reference = text.to_owned();
                let removed = ours.drain(start..end).collect::<std::string::String>();
                let expected = reference.drain(start..end).collect::<std::string::String>();
                assert_eq!(removed, expected);
                assert_eq!(ours.as_str(), reference);
            }
        }
    }
}

#[test]
fn priority_containers_match_sorted_values() {
    let mut max = BinaryHeap::<_, Max, 8>::new();
    let mut min = BinaryHeap::<_, Min, 8>::new();
    let mut list = SortedLinkedList::<_, ListMax, 8, u8>::new_u8();
    for n in [3,1,4,1,5,9,2,6] { max.push(n).unwrap(); min.push(n).unwrap(); list.push(n).unwrap(); }
    for expected in [9,6,5,4,3,2,1,1] {
        assert_eq!(max.pop(), Some(expected));
        assert_eq!(list.pop(), Some(expected));
    }
    for expected in [1,1,2,3,4,5,6,9] { assert_eq!(min.pop(), Some(expected)); }
    assert_eq!(max.pop(), None);
    assert_eq!(min.pop(), None);
    assert_eq!(list.pop(), None);
}
