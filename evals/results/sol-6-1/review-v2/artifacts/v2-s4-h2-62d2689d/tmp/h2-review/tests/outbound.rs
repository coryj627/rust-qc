use h2_support::prelude::*;
use futures::StreamExt;

#[tokio::test]
async fn accepted_connection_trailer_is_rejected_on_decode() {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/")).await;
        let result = peer.next().await.unwrap();
        let error: h2::Error = result.unwrap_err().into();
        assert_eq!(error.reason(), Some(h2::Reason::PROTOCOL_ERROR));
    };
    let client_task = async move {
        let (mut client, mut connection) = client::handshake(io).await.unwrap();
        let (_response, mut body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), false).unwrap();
        let mut trailers = HeaderMap::new();
        trailers.insert("connection", "close".parse().unwrap());
        assert!(body.send_trailers(trailers).is_ok()); // observed behavior
        let _: Result<(), _> = tokio::time::timeout(Duration::from_millis(20), &mut connection).await.map(|_| ());
    };
    join(peer_task, client_task).await;
}

#[tokio::test]
async fn accepted_authority_form_request_is_sent_without_scheme() {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        let frame = peer.next().await.unwrap().unwrap();
        match frame {
            h2::frame::Frame::Headers(headers) => {
                let (pseudo, _) = headers.into_parts();
                assert_eq!(pseudo.method, Some(Method::GET));
                assert!(pseudo.authority.is_some());
                assert!(pseudo.scheme.is_none());
            }
            other => panic!("expected request headers; actual={other:?}"),
        }
    };
    let client_task = async move {
        let (mut client, mut connection) = client::handshake(io).await.unwrap();
        let request = Request::builder().version(Version::HTTP_2).uri("example.com:443").body(()).unwrap();
        let (_response, _body) = client.send_request(request, true).unwrap();
        let _: Result<(), _> = tokio::time::timeout(Duration::from_millis(20), &mut connection).await.map(|_| ());
    };
    join(peer_task, client_task).await;
}
