use heapless::*;
#[test]
fn deque_full_mc_never_empties() {
    let mut d = Deque::<u32, 4>::new();
    for i in 0..4 { d.push_back(i).unwrap(); }
    d.pop_front(); d.pop_front();
    d.push_back(4).unwrap(); d.push_back(5).unwrap();
    d.make_contiguous();
    for _ in 0..9 { print!("{:?} ", d.pop_front()); }
    println!("len={}", d.len());
}
#[test]
fn deque_len_15() {
    let mut d = Deque::<u32, 16>::new();
    for _ in 0..8 { d.push_back(0).unwrap(); d.pop_front(); }
    for i in 1..=9 { d.push_back(i).unwrap(); }
    println!("{:?}", d.as_slices());
    println!("{:?}", d.make_contiguous());
    println!("len={} contents={:?}", d.len(), d);
}
#[test]
fn string_insert_at_end() {
    let mut s = String::<8>::new();
    s.insert(0, 'é').unwrap();
    assert_eq!(s, "é");
}
