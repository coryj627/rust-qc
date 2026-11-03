
#[cfg(test)]
mod negative_control_probes {
    use super::*;

    #[test]
    fn decoy_1_ack_range_validation_precedes_iteration() {
        // Type, largest, delay, range count, first range, then gap/range pairs.
        let malformed: &[&[u8]] = &[
            &[0x02, 0, 0, 0, 1],       // first range exceeds largest
            &[0x02, 1, 0, 1, 0, 0, 0], // gap requires subtracting 2 from 1
            &[0x02, 3, 0, 1, 0, 0, 2], // second range exceeds remaining 1
            &[0x02, 3, 0, 1, 0, 0],    // second range truncated
        ];
        for packet in malformed {
            let mut frames = Iter::new(Bytes::copy_from_slice(packet)).unwrap();
            let error = frames.next().unwrap().unwrap_err();
            let error: TransportError = error.into();
            assert_eq!(error.code, crate::TransportErrorCode::FRAME_ENCODING_ERROR);
            assert!(frames.next().is_none());
        }
        // Exact boundary: ranges 3..=3 and 0..=1 are valid.
        let mut frames = Iter::new(Bytes::from_static(&[0x02, 3, 0, 1, 0, 0, 1])).unwrap();
        let Frame::Ack(ack) = frames.next().unwrap().unwrap() else { panic!("not ACK") };
        assert_eq!(ack.iter().collect::<Vec<_>>(), vec![3..=3, 0..=1]);
        println!("decoy-1: 4 malformed ACKs rejected with FRAME_ENCODING_ERROR; boundary ACK iterates safely");
    }
}
