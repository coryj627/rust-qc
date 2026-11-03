
#[cfg(test)]
mod negative_control_probes {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn false_5_fin_final_size_is_enforced_on_later_stream_frames() {
        let mut recv = Recv::new(1024);
        let stream = |offset, fin, data: &'static [u8]| frame::Stream {
            id: StreamId::new(crate::Side::Client, crate::Dir::Bi, 0),
            offset,
            fin,
            data: Bytes::from_static(data),
        };
        assert_eq!(recv.ingest(stream(0, true, b"abcd"), 4, 0, 1024).unwrap().0, 4);
        assert_eq!(recv.final_offset(), Some(4));
        // A retransmission wholly inside the final size remains valid.
        assert_eq!(recv.ingest(stream(1, false, b"bc"), 2, 4, 1024).unwrap().0, 0);
        let beyond = recv.ingest(stream(4, false, b"e"), 1, 4, 1024).unwrap_err();
        assert_eq!(beyond.code, crate::TransportErrorCode::FINAL_SIZE_ERROR);
        let conflicting_fin = recv.ingest(stream(0, true, b"abc"), 3, 4, 1024).unwrap_err();
        assert_eq!(conflicting_fin.code, crate::TransportErrorCode::FINAL_SIZE_ERROR);
        assert_eq!(recv.end, 4);
        assert_eq!(recv.final_offset(), Some(4));
        println!("false-5: in-range retransmission accepted; beyond-FIN data and conflicting FIN rejected with FINAL_SIZE_ERROR");
    }
}
