
mod negative_control_probes {
    use super::*;
    use crate::{packet::{InitialHeader, PacketNumber}, token::{IncomingToken, Token, TokenPayload}};

    #[test]
    fn false_2_retry_token_is_bound_to_udp_port() {
        let config = server_config();
        let original: SocketAddr = "127.0.0.1:4000".parse().unwrap();
        let changed_port: SocketAddr = "127.0.0.1:4001".parse().unwrap();
        let orig_dst_cid = ConnectionId::new(&[1; 8]);
        let token = Token::new(TokenPayload::Retry {
            address: original,
            orig_dst_cid,
            issued: config.time_source.now(),
        }, &mut rand::rng());
        let header = InitialHeader {
            dst_cid: ConnectionId::new(&[2; 8]),
            src_cid: ConnectionId::new(&[3; 8]),
            token: token.encode(&*config.token_key).into(),
            number: PacketNumber::U8(0),
            version: 1,
        };
        assert!(IncomingToken::from_header(&header, &config, changed_port).is_err());
        let accepted = IncomingToken::from_header(&header, &config, original)
            .unwrap_or_else(|_| panic!("original address rejected"));
        assert!(accepted.validated);
        assert_eq!(accepted.orig_dst_cid, orig_dst_cid);
        assert_eq!(accepted.retry_src_cid, Some(header.dst_cid));
        println!("false-2: genuine encrypted Retry token accepted at original port; same IP/different port rejected");
    }
}
