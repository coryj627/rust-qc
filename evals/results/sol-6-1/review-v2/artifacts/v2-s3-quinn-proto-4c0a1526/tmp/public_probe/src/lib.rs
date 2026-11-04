#[cfg(test)]
mod tests {
    use quinn_proto::transport_parameters::TransportParameters;
    use quinn_proto::{ConnectionId, ConnectionIdGenerator, HashedConnectionIdGenerator, Side};

    #[test]
    fn nonminimal_transport_integer_is_valid() {
        let data = [0x04, 0x02, 0x40, 0x01];
        let result = TransportParameters::read(Side::Server, &mut &data[..]);
        assert!(result.is_ok(), "expected valid parameters, got {result:?}");
    }

    #[test]
    fn truncated_optional_datagram_integer_returns_error() {
        let data = [0x20, 0x00];
        assert!(TransportParameters::read(Side::Server, &mut &data[..]).is_err());
    }

    #[test]
    fn cid_validation_handles_supported_cid_lengths() {
        let generator = HashedConnectionIdGenerator::from_key(1);
        let mut panicked_lengths = Vec::new();
        for len in [0, 1, 2, 3, 7, 8, 9, 20] {
            let cid = ConnectionId::new(&[0; 20][..len]);
            let result = std::panic::catch_unwind(|| generator.validate(&cid));
            if result.is_err() {
                panicked_lengths.push(len);
            }
        }
        assert!(
            panicked_lengths.is_empty(),
            "validation panicked for CID lengths {panicked_lengths:?}"
        );
    }
}

#[cfg(test)]
mod boundary_tests {
    use quinn_proto::{
        coding::Codec, transport_parameters::TransportParameters, Dir, Side, StreamId, VarInt,
    };

    #[test]
    fn control_varint_accepts_longer_encoding() {
        assert_eq!(
            VarInt::decode(&mut &[0x40, 0x01][..]).unwrap(),
            VarInt::from_u32(1)
        );
        assert!(TransportParameters::read(Side::Server, &mut &[0x04, 0x01, 0x01][..]).is_ok());
        assert!(TransportParameters::read(Side::Server, &mut &[][..]).is_ok());
    }

    #[test]
    fn stream_id_constructor_preserves_varint_invariant() {
        let id = StreamId::new(Side::Client, Dir::Uni, 1 << 60);
        let value = VarInt::from(id);
        assert!(
            value <= VarInt::MAX,
            "stream ID produced invalid VarInt {value}"
        );
    }

    #[test]
    fn truncated_min_ack_delay_returns_error() {
        let mut data = Vec::new();
        VarInt::from_u32(0xff04de1b).encode(&mut data);
        data.push(0);
        assert!(TransportParameters::read(Side::Server, &mut &data[..]).is_err());
    }
}

#[cfg(test)]
mod configuration_tests {
    use quinn_proto::{crypto::rustls::QuicClientConfig, ClientConfig, Endpoint};
    use std::{sync::Arc, time::Instant};

    fn tls12_config() -> rustls::ClientConfig {
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS12])
        .unwrap()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth()
    }

    #[test]
    fn tls12_only_client_conversion_returns_error() {
        assert!(QuicClientConfig::try_from(tls12_config()).is_err());
    }

    #[test]
    fn invalid_tls_configuration_connect_returns_result() {
        let config = ClientConfig::new(Arc::new(
            QuicClientConfig::try_from(tls12_config()).unwrap(),
        ));
        let mut endpoint = Endpoint::new(Default::default(), None, true, None);
        let result = endpoint.connect(
            Instant::now(),
            config,
            "127.0.0.1:443".parse().unwrap(),
            "localhost",
        );
        assert!(result.is_err());
    }
}
