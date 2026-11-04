
#[cfg(test)]
mod negative_control_probes {
    use super::*;

    #[test]
    fn decoy_2_overlapping_and_duplicate_stream_acks_are_idempotent() {
        let mut buf = SendBuffer::new();
        buf.write(Bytes::from_static(b"abcdefghijklmnop"));
        assert_eq!(buf.poll_transmit(64).0, 0..16);
        buf.ack(8..12);
        buf.ack(8..12); // duplicate still held behind a gap
        buf.ack(10..16); // overlapping suffix
        assert_eq!(buf.unacked(), 8);
        assert_eq!(buf.unacked_len, 16);
        buf.ack(0..10); // merges to 0..16 and releases exactly once
        assert!(buf.is_fully_acked());
        assert!(buf.unacked_segments.is_empty());
        for range in [0..10, 8..12, 10..16, 0..16] {
            buf.ack(range); // repeats wholly below the new base offset
            assert_eq!(buf.unacked(), 0);
        }
        buf.write(Bytes::from_static(b"qrst"));
        buf.ack(0..16); // stale ACK must not free newly appended bytes
        assert_eq!(buf.unacked(), 4);
        assert_eq!(buf.get(16..20), b"qrst");
        println!("decoy-2: overlapping/duplicate ACKs release 16 bytes once; stale ACK preserves 4 new bytes");
    }
}
