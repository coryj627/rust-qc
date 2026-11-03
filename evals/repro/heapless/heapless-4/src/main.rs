use heapless::Deque;

fn main() {
    let mut d: Deque<u8, 4> = Deque::new();
    for i in 0..4 {
        d.push_back(i).unwrap();
    }
    d.pop_front();
    d.pop_front();
    d.push_back(4).unwrap();
    d.push_back(5).unwrap();
    // Full and wrapped: front == back == 2, with equally long head and tail, which selects the
    // branch that sets back = len = N.
    assert!(d.is_full());
    assert_eq!(d.make_contiguous(), &[2, 3, 4, 5]);
    d.pop_front(); // no longer full, so the next push_back is allowed
    println!("before push_back: len={} capacity={}", d.len(), d.capacity());
    d.push_back(6).unwrap(); // writes buffer[4] of a 4-slot buffer
    println!("after 1 push_back: len={} capacity={} is_full={}", d.len(), d.capacity(), d.is_full());
    let second = d.push_back(7); // a correct deque would be full and return Err(7)
    println!("second push_back: {:?}, len={} capacity={}", second, d.len(), d.capacity());
    assert!(d.len() <= d.capacity(), "len {} exceeds capacity {}", d.len(), d.capacity());
    println!("consistent: no out-of-bounds state");
}
