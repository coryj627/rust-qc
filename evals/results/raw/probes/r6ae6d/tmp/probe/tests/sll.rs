use heapless::sorted_linked_list::{Max, SortedLinkedList};

#[test]
fn find_pop_after_mut_nonhead_box() {
    let mut ll: SortedLinkedList<std::boxed::Box<u32>, Max, 4, u8> = SortedLinkedList::new_u8();
    ll.push(Box::new(1)).unwrap();
    ll.push(Box::new(2)).unwrap();
    ll.push(Box::new(3)).unwrap();
    let mut f = ll.find_mut(|v| **v == 2).unwrap();
    **f = 20;
    let popped = f.pop();
    println!("popped {popped}");
    drop(popped);
    while let Some(x) = ll.pop() { println!("rest {x}"); }
}

#[test]
fn find_pop_after_mut_nonhead_u32() {
    let mut ll: SortedLinkedList<u32, Max, 4, u8> = SortedLinkedList::new_u8();
    ll.push(1).unwrap();
    ll.push(2).unwrap();
    ll.push(3).unwrap();
    let mut f = ll.find_mut(|v| *v == 2).unwrap();
    *f = 20;
    let popped = f.pop();
    println!("popped {popped}");
    let v: std::vec::Vec<u32> = ll.iter().take(8).copied().collect();
    println!("remaining (first 8) {v:?}");
    assert_eq!(v, vec![3, 1]);
}

#[test]
fn find_pop_after_mut_only_head() {
    let mut ll: SortedLinkedList<u32, Max, 4, u8> = SortedLinkedList::new_u8();
    ll.push(1).unwrap();
    let mut f = ll.find_mut(|v| *v == 1).unwrap();
    *f = 20;
    let popped = f.pop();
    println!("popped {popped}");
    println!("empty? {}", ll.is_empty());
}
