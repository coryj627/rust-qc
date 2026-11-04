use super::*;

#[test]
fn negative_control_false_3_overlapping_acks_do_not_double_subtract() {
    let mut buffer = SendBuffer::new();
    buffer.write(Bytes::from_static(b"abcd"));
    buffer.write(Bytes::from_static(b"efgh"));
    buffer.write(Bytes::from_static(b"ijkl"));
    assert_eq!(buffer.poll_transmit(32).0, 0..12);
    buffer.ack(0..4);
    assert_eq!(buffer.unacked(), 8);
    buffer.ack(8..12);
    assert_eq!(buffer.unacked(), 4);
    buffer.ack(8..12);
    assert_eq!(buffer.unacked(), 4);
    buffer.ack(2..6);
    assert_eq!(buffer.unacked(), 2);
    assert_eq!(buffer.get(6..8), b"gh");
    buffer.ack(4..10);
    assert_eq!(buffer.unacked(), 0);
    assert!(buffer.is_fully_acked());
    buffer.ack(0..12);
    assert_eq!(buffer.unacked(), 0);
    assert!(buffer.is_fully_acked());
    println!("false-3: duplicate suffix ACK stayed at 4 outstanding bytes; overlapping ACK across discarded prefix left gh; final overlap reached exactly zero; late duplicate stayed zero");
}
