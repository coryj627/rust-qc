use super::*;

#[test]
fn negative_control_decoy_1_datagram_backpressure_preserves_queue() {
    let mut pair = Pair::default();
    let mut client = client_config();
    let mut transport = TransportConfig::default();
    transport.datagram_send_buffer_size(8);
    client.transport_config(Arc::new(transport));
    let (client_ch, server_ch) = pair.connect_with(client);
    let first = Bytes::from_static(b"abcd");
    let second = Bytes::from_static(b"efgh");
    let retry = Bytes::from_static(b"i");
    pair.client_datagrams(client_ch).send(first.clone(), false).unwrap();
    pair.client_datagrams(client_ch).send(second.clone(), false).unwrap();
    assert_eq!(pair.client_datagrams(client_ch).send_buffer_space(), 0);
    assert_eq!(pair.client_datagrams(client_ch).send(retry.clone(), false),
               Err(SendDatagramError::Blocked(retry.clone())));
    assert_eq!(pair.client_datagrams(client_ch).send_buffer_space(), 0);
    pair.drive();
    assert_eq!(pair.server_datagrams(server_ch).recv(), Some(first));
    assert_eq!(pair.server_datagrams(server_ch).recv(), Some(second));
    assert_eq!(pair.server_datagrams(server_ch).recv(), None);
    assert_eq!(pair.client_datagrams(client_ch).send_buffer_space(), 8);
    pair.client_datagrams(client_ch).send(retry.clone(), false).unwrap();
    pair.drive();
    assert_eq!(pair.server_datagrams(server_ch).recv(), Some(retry));
    println!("decoy-1: exact 8-byte capacity accepted; excess returned Blocked with original data; queued payloads arrived in order; retry arrived");
}

#[test]
fn negative_control_decoy_2_partial_chunk_preserves_suffix() {
    let mut server = server_config();
    let mut transport = TransportConfig::default();
    transport.stream_receive_window(4u32.into());
    server.transport_config(Arc::new(transport));
    let mut pair = Pair::new(Default::default(), server);
    let (client_ch, server_ch) = pair.connect();
    let id = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    let mut data = [Bytes::new(), Bytes::from_static(b"ab"), Bytes::new(), Bytes::from_static(b"cdef")];
    let written = pair.client_send(client_ch, id).write_chunks(&mut data).unwrap();
    assert_eq!(written, Written { bytes: 4, chunks: 3 });
    assert!(data[..3].iter().all(Bytes::is_empty));
    assert_eq!(&data[3][..], b"ef");
    pair.drive();
    assert_eq!(pair.server_streams(server_ch).accept(Dir::Uni), Some(id));
    assert_eq!(stream_chunks(pair.server_recv(server_ch, id)), b"abcd");
    pair.drive();
    assert_eq!(pair.client_send(client_ch, id).write_chunks(&mut data[3..]).unwrap(),
               Written { bytes: 2, chunks: 1 });
    pair.client_send(client_ch, id).finish().unwrap();
    pair.drive();
    assert_eq!(stream_chunks(pair.server_recv(server_ch, id)), b"ef");
    println!("decoy-2: 4-byte credit wrote abcd; partial cdef was excluded from chunks and retained ef; suffix retry delivered ef");
}
