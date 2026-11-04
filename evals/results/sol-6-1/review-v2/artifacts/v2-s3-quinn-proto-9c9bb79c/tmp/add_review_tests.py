from pathlib import Path
root=Path('/work/tmp/review/quinn-proto/src')
p=root/'tests/mod.rs'
s=p.read_text()
start=s.index('#[test]\nfn zero_rtt_rejection()')
end=s.index('\nfn test_zero_rtt_incoming_limit', start)
t=s[start:end].replace('fn zero_rtt_rejection()', 'fn review_zero_rtt_rejection_restores_send_window()')
t=t.replace('let client_config = ClientConfig::new(client_crypto.clone());', 'let mut client_config = ClientConfig::new(client_crypto.clone());\n    client_config.transport_config(Arc::new(TransportConfig { send_window: 32, ..Default::default() }));')
t=t.replace('let client_config = ClientConfig::new(client_crypto);', 'let mut client_config = ClientConfig::new(client_crypto);\n    client_config.transport_config(Arc::new(TransportConfig { send_window: 32, ..Default::default() }));')
t=t.replace('const MSG: &[u8] = b"Hello, 0-RTT!";', 'const MSG: &[u8] = &[42; 32];')
t=t.replace('assert_eq!(s, s2);', 'assert_eq!(s, s2);\n    assert_eq!(pair.client_send(client_ch, s2).write(b"retry"), Ok(5), "rejected data must not consume the new stream send window");')
p.write_text(s+'\n'+t+'''
#[test]
fn review_illegal_ordered_read_preserves_stream() {
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    pair.client_send(client_ch, s).write(b"hello").unwrap();
    pair.client_send(client_ch, s).finish().unwrap();
    pair.drive();
    assert_eq!(pair.server_streams(server_ch).accept(Dir::Uni), Some(s));
    {
        let mut recv = pair.server_recv(server_ch, s);
        let mut chunks = recv.read(false).unwrap();
        assert_eq!(chunks.next(1).unwrap().unwrap().bytes, b"h"[..]);
        let _ = chunks.finalize();
    }
    assert_matches!(pair.server_recv(server_ch, s).read(true), Err(ReadableError::IllegalOrderedRead));
    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).expect("an ordering error must leave the stream available for unordered reads");
    let chunk = chunks.next(usize::MAX).unwrap().unwrap();
    assert_eq!(chunk.offset, 1);
    assert_eq!(chunk.bytes, b"ello"[..]);
    assert_eq!(chunks.next(usize::MAX), Ok(None));
}

#[test]
fn review_datagram_drop_respects_buffer_limit() {
    let mut pair = Pair::default();
    let mut config = client_config();
    config.transport_config(Arc::new(TransportConfig {
        datagram_send_buffer_size: 10,
        ..Default::default()
    }));
    let (client_ch, server_ch) = pair.connect_with(config);
    pair.client_datagrams(client_ch).send(Bytes::from_static(b"first!"), true).unwrap();
    assert_eq!(pair.client_datagrams(client_ch).send_buffer_space(), 4);
    pair.client_datagrams(client_ch).send(Bytes::from_static(b"second"), true).unwrap();
    pair.drive();
    assert_eq!(pair.server_datagrams(server_ch).recv().unwrap(), b"second"[..], "older datagram must be evicted before exceeding ten buffered bytes");
    assert_eq!(pair.server_datagrams(server_ch).recv(), None);
}

#[test]
fn review_datagram_max_size_when_disabled() {
    let mut pair = Pair::default();
    let mut config = client_config();
    config.transport_config(Arc::new(TransportConfig {
        datagram_receive_buffer_size: None,
        ..Default::default()
    }));
    let (client_ch, _) = pair.connect_with(config);
    assert_eq!(pair.client_datagrams(client_ch).send(Bytes::new(), true), Err(SendDatagramError::Disabled));
    assert_eq!(pair.client_datagrams(client_ch).max_size(), None);
}
''')
p=root/'connection/datagrams.rs'
p.write_text(p.read_text()+'''
#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn review_drop_oversized_keeps_limit_sized_datagram() {
        let mut state = DatagramState::default();
        state.outgoing.push_back(Datagram { data: Bytes::from_static(b"hello") });
        state.outgoing_total = 5;
        state.drop_oversized(5);
        assert_eq!(state.outgoing_total, 5);
        assert_eq!(state.outgoing.len(), 1);
    }
}
''')
p=root/'connection/mtud.rs'
p.write_text(p.read_text()+'''
#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn review_zero_minimum_change_completes() {
        let mut config = MtuDiscoveryConfig::default();
        config.minimum_change(0);
        let mut mtud = MtuDiscovery::new(1200, 1200, None, config);
        let now = Instant::now();
        let mut probes = Vec::new();
        for pn in 0..32 {
            let Some(size) = mtud.poll_transmit(now, pn) else { return };
            probes.push(size);
            mtud.on_acked(SpaceId::Data, pn, size);
        }
        panic!("MTU discovery did not complete after 32 successful probes: {probes:?}");
    }
}
''')
