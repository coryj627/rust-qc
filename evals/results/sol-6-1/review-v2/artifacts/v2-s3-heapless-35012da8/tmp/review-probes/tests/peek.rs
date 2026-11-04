#[test]
fn consumer_peek_observes_published_value() {
    let mut queue = heapless::spsc::Queue::<u32, 2>::new();
    let (mut producer, consumer) = queue.split();
    std::thread::scope(|scope| {
        scope.spawn(move || producer.enqueue(42).unwrap());
        scope.spawn(move || loop {
            if let Some(value) = consumer.peek() {
                assert_eq!(*value, 42);
                break;
            }
            std::thread::yield_now();
        });
    });
}
