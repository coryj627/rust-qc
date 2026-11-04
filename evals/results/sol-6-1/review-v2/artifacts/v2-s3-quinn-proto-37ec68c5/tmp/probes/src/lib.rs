#[cfg(test)]
mod tests {
    use quinn_proto::{
        coding::Codec, transport_parameters::TransportParameters, ConnectionId,
        ConnectionIdGenerator, HashedConnectionIdGenerator, Side, VarInt,
    };

    #[test]
    fn review_transport_parameters_accept_nonminimal_integer() {
        let mut canonical = &[0x04, 0x01, 0x01][..];
        let expected = TransportParameters::read(Side::Server, &mut canonical).unwrap();
        let mut padded = &[0x04, 0x02, 0x40, 0x01][..];
        assert_eq!(
            TransportParameters::read(Side::Server, &mut padded),
            Ok(expected)
        );
    }

    #[test]
    fn review_truncated_datagram_parameter_returns_error() {
        let mut panicked = Vec::new();
        for data in [&[0x20, 0x00][..], &[0x20, 0x01, 0x40][..]] {
            let result = std::panic::catch_unwind(|| {
                TransportParameters::read(Side::Server, &mut &data[..])
            });
            match result {
                Err(_) => panicked.push(data.to_vec()),
                Ok(value) => assert!(value.is_err()),
            }
        }
        assert!(
            panicked.is_empty(),
            "truncated parameters panicked: {panicked:02x?}"
        );
    }

    #[test]
    fn review_truncated_ack_delay_parameter_returns_error() {
        let mut data = Vec::new();
        VarInt::from_u32(0xff04de1b).encode(&mut data);
        VarInt::from_u32(0).encode(&mut data);
        let result = std::panic::catch_unwind(|| {
            TransportParameters::read(Side::Server, &mut data.as_slice())
        });
        assert!(result.is_ok(), "empty min_ack_delay must return an error");
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn review_cid_validation_rejects_short_ids_without_panicking() {
        let generator = HashedConnectionIdGenerator::from_key(7);
        let mut panicked_lengths = Vec::new();
        for n in 0..3 {
            let cid = ConnectionId::new(&[0; 2][..n]);
            let result = std::panic::catch_unwind(|| generator.validate(&cid));
            match result {
                Err(_) => panicked_lengths.push(n),
                Ok(value) => assert!(value.is_err()),
            }
        }
        assert!(
            panicked_lengths.is_empty(),
            "validation panicked for CID lengths: {panicked_lengths:?}"
        );
    }

    #[test]
    fn review_varint_boundary_roundtrips_and_truncation() {
        for value in [
            0,
            1,
            63,
            64,
            16383,
            16384,
            (1 << 30) - 1,
            1 << 30,
            (1 << 62) - 1,
        ] {
            let number = VarInt::from_u64(value).unwrap();
            let mut encoded = Vec::new();
            number.encode(&mut encoded);
            assert_eq!(VarInt::decode(&mut encoded.as_slice()), Ok(number));
            for n in 0..encoded.len() {
                assert!(VarInt::decode(&mut &encoded[..n]).is_err());
            }
        }
        assert!(VarInt::from_u64(1 << 62).is_err());
    }
}
