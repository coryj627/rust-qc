use super::*;

#[test]
fn negative_control_false_4_ordered_to_unordered_deduplicates_buffered_data() {
    let mut assembler = Assembler::new();
    assembler.insert(0, Bytes::from_static(b"abc"), 3);
    assert_eq!(assembler.read(3, true), Some(Chunk::new(0, Bytes::from_static(b"abc"))));
    assembler.insert(3, Bytes::from_static(b"def"), 3);
    assembler.insert(3, Bytes::from_static(b"def"), 3);
    assembler.insert(4, Bytes::from_static(b"efgh"), 4);
    assembler.ensure_ordering(false).unwrap();
    let mut observed = Vec::new();
    while let Some(chunk) = assembler.read(2, false) {
        observed.extend((chunk.offset..).zip(chunk.bytes.iter().copied()));
    }
    assert_eq!(observed, vec![(3,b'd'), (4,b'e'), (5,b'f'), (6,b'g'), (7,b'h')]);
    assembler.insert(0, Bytes::from_static(b"abcdefghij"), 10);
    assert_eq!(assembler.read(8, false), Some(Chunk::new(8, Bytes::from_static(b"ij"))));
    assert_eq!(assembler.read(8, false), None);
    assert_eq!(assembler.bytes_read(), 10);
    println!("false-4: duplicate and overlapping buffered frames yielded offsets 3..8 exactly once after mode switch; retransmitted prefix suppressed and only new ij delivered");
}
