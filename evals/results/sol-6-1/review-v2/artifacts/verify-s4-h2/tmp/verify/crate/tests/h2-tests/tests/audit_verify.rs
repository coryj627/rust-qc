use h2_support::prelude::*;
use futures::StreamExt;
use futures::channel::oneshot;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::io;

fn wire(kind:u8, flags:u8, id:u32, payload:&[u8])->Vec<u8> {
    let n=payload.len();
    let mut out=vec![(n>>16) as u8,(n>>8) as u8,n as u8,kind,flags];
    out.extend_from_slice(&id.to_be_bytes()); out.extend_from_slice(payload); out
}
async fn reply_check<F, Fut, T>(bytes:Vec<u8>, limit:u32, check:F)->T
where F:FnOnce(Result<Response<RecvStream>,h2::Error>)->Fut, Fut:Future<Output=T> {
    let (io,mut peer)=mock::new();
    let (tx,rx)=oneshot::channel();
    let server=async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET","https://x/").eos()).await;
        peer.send_bytes(&bytes).await;
        let _=rx.await;
    };
    let client=async move {
        let (mut client,mut conn)=client::Builder::new().max_header_list_size(limit)
            .handshake::<_,Bytes>(io).await.unwrap();
        let (response,_)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),true).unwrap();
        conn.drive(async move { let result=check(response.await).await; let _=tx.send(()); result }).await
    };
    tokio::time::timeout(Duration::from_secs(2),async {join(server,client).await.1}).await.expect("bounded exchange")
}
#[tokio::test]
async fn reservation_truncates() {
    let (io,_peer)=mock::new();
    let (mut client,_conn)=client::handshake(io).await.unwrap();
    let (_response,mut stream)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),false).unwrap();
    stream.reserve_capacity(1); assert_eq!(stream.capacity(),1);
    stream.reserve_capacity(1usize<<32); assert_eq!(stream.capacity(),0);
    stream.reserve_capacity(65535); assert_eq!(stream.capacity(),65535);
}
#[tokio::test]
async fn missing_status_defaults_to_200() {
    reply_check(wire(1,5,1,&[0x0f,13,1,b'0']),4096,|r|async move {
        assert_eq!(r.unwrap().status(),200);
    }).await;
}
#[tokio::test]
async fn header_limit_boundary() {
    for (limit,ok) in [(41,false),(42,false),(43,true)] {
        reply_check(wire(1,5,1,&[0x88]),limit,move |r|async move {
            assert_eq!(r.is_ok(),ok,"limit={limit}, result={r:?}");
            if let Err(e)=r {assert_eq!(e.reason(),Some(Reason::PROTOCOL_ERROR));}
        }).await;
    }
}
#[tokio::test]
async fn response_304_empty_data() {
    for (end_headers,ok) in [(true,true),(false,false)] {
        let mut bytes=wire(1,if end_headers {5}else{4},1,&[0x8b,0x0f,13,3,b'1',b'0',b'0']);
        if !end_headers {bytes.extend(wire(0,1,1,&[]));}
        reply_check(bytes,4096,move |r|async move {
            let mut body=r.unwrap().into_body();
            let result=body.data().await;
            if ok {assert!(result.is_none());} else {
                let e=result.unwrap().unwrap_err(); assert_eq!(e.reason(),Some(Reason::PROTOCOL_ERROR));
            }
        }).await;
    }
}
#[tokio::test]
async fn informational_103_is_skipped() {
    let mut bytes=wire(1,4,1,&[8,3,b'1',b'0',b'3']); bytes.extend(wire(1,5,1,&[0x88]));
    reply_check(bytes,4096,|r|async move {assert_eq!(r.unwrap().status(),200);}).await;
}
#[tokio::test]
async fn data_keeps_empty_body_trailers() {
    let mut bytes=wire(1,4,1,&[0x88]);
    let mut payload=vec![0,8]; payload.extend(b"x-result");payload.push(2);payload.extend(b"ok");
    bytes.extend(wire(1,5,1,&payload));
    reply_check(bytes,4096,|r|async move {
        let mut body=r.unwrap().into_body(); assert!(body.data().await.is_none());
        assert_eq!(body.trailers().await.unwrap().unwrap()["x-result"],"ok");
    }).await;
}
#[tokio::test]
async fn unknown_setting_is_ignored() {
    let (io,mut peer)=mock::new();
    let (tx,rx)=oneshot::channel();
    let server=async move {
        peer.send_bytes(&wire(4,0,0,&[0x0a,0xbc,0,0,0,7])).await;
        peer.read_preface().await.unwrap();
        let settings=peer.next().await.unwrap().unwrap();assert!(matches!(settings,frame::Frame::Settings(_)));
        peer.send_frame(frame::Settings::ack()).await;
        let mut got_ack=false;let mut got_request=false;
        while !got_ack || !got_request {
            match peer.next().await.unwrap().unwrap() {
                frame::Frame::Settings(s)=>{assert!(s.is_ack());got_ack=true;},
                frame::Frame::Headers(_)=>got_request=true,
                other=>panic!("unexpected {other:?}"),
            }
        }
        peer.send_frame(frames::headers(1).response(200).eos()).await;
        let _=rx.await;
    };
    let client=async move {
        let (mut client,mut conn)=client::handshake(io).await.unwrap();
        let (response,_)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),true).unwrap();
        conn.drive(async move { assert_eq!(response.await.unwrap().status(),200);let _=tx.send(()); }).await;
    };
    tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
}
#[tokio::test]
async fn trailers_are_sent_without_validation() {
    let (io,mut peer)=mock::new();
    let (tx,rx)=oneshot::channel();
    let server=async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET","https://x/")).await;
        let e=peer.next().await.unwrap().unwrap_err();
        assert!(format!("{e:?}").contains("PROTOCOL_ERROR"),"{e:?}");
        let _=tx.send(());
    };
    let client=async move {
        let (mut client,mut conn)=client::handshake(io).await.unwrap();
        let (_response,mut stream)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),false).unwrap();
        let mut trailers=HeaderMap::new(); trailers.insert("connection","close".parse().unwrap());
        assert!(stream.send_trailers(trailers).is_ok());
        conn.drive(rx).await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
}
#[tokio::test]
async fn authority_get_is_emitted_without_scheme() {
    let (io,mut peer)=mock::new();let (tx,rx)=oneshot::channel();
    let server=async move {
        peer.assert_client_handshake().await;
        let f=peer.next().await.unwrap().unwrap();
        if let frame::Frame::Headers(h)=f {let (pseudo,_)=h.into_parts();assert!(pseudo.scheme.is_none());assert_eq!(pseudo.method.as_ref().unwrap(),Method::GET);}
        else {panic!("{f:?}");}
        let _=tx.send(());
    };
    let client=async move {
        let (mut client,mut conn)=client::handshake(io).await.unwrap();
        let _result=client.send_request(Request::builder().method("GET").uri("example.com:443").body(()).unwrap(),true).unwrap();
        conn.drive(rx).await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
}
#[derive(Debug)]
struct ZeroOnce { writes:usize }
impl AsyncRead for ZeroOnce {
    fn poll_read(self:Pin<&mut Self>,_:&mut Context<'_>,_:&mut tokio_io::ReadBuf<'_>)->Poll<io::Result<()>> {Poll::Pending}
}
impl AsyncWrite for ZeroOnce {
    fn poll_write(mut self:Pin<&mut Self>,_:&mut Context<'_>,_:&[u8])->Poll<io::Result<usize>> {
        self.writes+=1;if self.writes==1 {Poll::Ready(Ok(0))} else {Poll::Ready(Err(io::Error::new(io::ErrorKind::Other,"sentinel")))}
    }
    fn poll_flush(self:Pin<&mut Self>,_:&mut Context<'_>)->Poll<io::Result<()>> {Poll::Ready(Ok(()))}
    fn poll_shutdown(self:Pin<&mut Self>,_:&mut Context<'_>)->Poll<io::Result<()>> {Poll::Ready(Ok(()))}
}
#[tokio::test]
async fn zero_write_is_retried() {
    let mut codec=Codec::new(ZeroOnce{writes:0});codec.buffer(frame::Settings::default().into()).unwrap();
    let e=poll_fn(|cx|codec.flush(cx)).await.unwrap_err();
    assert_eq!(e.kind(),io::ErrorKind::Other);assert_eq!(e.to_string(),"sentinel");assert_eq!(codec.get_mut().writes,2);
}
#[tokio::test]
async fn padded_data_loses_window_credit() {
    for padded in [false,true] {
        let (io,mut peer)=mock::new();let (read_tx,read_rx)=oneshot::channel();let (done_tx,done_rx)=oneshot::channel();
        let server=async move {
            peer.assert_client_handshake().await;
            peer.recv_frame(frames::headers(1).request("GET","https://x/").eos()).await;
            peer.send_frame(frames::headers(1).response(200)).await;
            peer.send_bytes(&wire(0,if padded{8}else{0},1,if padded{&[6,b'x',0,0,0,0,0,0]}else{b"xxxxxxxx"})).await;
            read_rx.await.unwrap();
            let update=tokio::time::timeout(Duration::from_millis(50),peer.next()).await;
            if padded {assert!(update.is_err(),"padding credit unexpectedly restored: {update:?}");}
            else {let f=update.unwrap().unwrap().unwrap();assert_frame_eq(f,frames::window_update(1,8));}
            let _=done_tx.send(());
        };
        let client=async move {
            let (mut client,mut conn)=client::Builder::new().initial_window_size(8).handshake::<_,Bytes>(io).await.unwrap();
            let (response,_)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),true).unwrap();
            conn.drive(async move {
                let mut body=response.await.unwrap().into_body();let data=body.data().await.unwrap().unwrap();
                assert_eq!(data.len(),if padded{1}else{8});body.flow_control().release_capacity(data.len()).unwrap();
                let _=read_tx.send(());done_rx.await.unwrap();
            }).await;
        };
        tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
    }
}
#[tokio::test]
async fn zero_stream_window_blocks_data() {
    let (io,mut peer)=mock::new();let (ready_tx,ready_rx)=oneshot::channel();let (done_tx,done_rx)=oneshot::channel();
    let server=async move {
        peer.assert_client_handshake_with_settings(frames::settings().initial_window_size(0)).await;
        let _=ready_tx.send(());
        peer.recv_frame(frames::headers(1).request("POST","https://x/")).await;
        assert!(tokio::time::timeout(Duration::from_millis(50),peer.next()).await.is_err(),"DATA sent with zero credit");
        peer.send_frame(frames::window_update(1,3)).await;
        peer.recv_frame(frames::data(1,"abc").eos()).await;
        let _=done_tx.send(());
    };
    let client=async move {
        let (mut client,mut conn)=client::handshake(io).await.unwrap();conn.drive(ready_rx).await.unwrap();
        let (_response,mut stream)=client.send_request(Request::builder().method("POST").uri("https://x/").body(()).unwrap(),false).unwrap();
        stream.send_data(Bytes::from_static(b"abc"),true).unwrap();conn.drive(done_rx).await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
}
#[tokio::test]
async fn pushed_head_lacks_head_semantics() {
    for length in [false,true] {
        let (io,mut peer)=mock::new();let (tx,rx)=oneshot::channel();
        let server=async move {
            peer.assert_client_handshake().await;
            peer.recv_frame(frames::headers(1).request("GET","https://x/").eos()).await;
            peer.send_frame(frames::push_promise(1,2).request("HEAD","https://x/")).await;
            peer.send_frame(frames::headers(1).response(200).eos()).await;
            if length {peer.send_frame(frames::headers(2).response(200).field("content-length","100").eos()).await;}
            else {peer.send_frame(frames::headers(2).response(200).eos()).await;}
            let _=rx.await;
        };
        let client=async move {
            let (mut client,mut conn)=client::handshake(io).await.unwrap();
            let (mut response,_)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),true).unwrap();
            let mut pushes=response.push_promises();
            conn.drive(async move {
                let promise=pushes.next().await.unwrap().unwrap();let (request,pushed)=promise.into_parts();assert_eq!(request.method(),Method::HEAD);
                let result=pushed.await;assert_eq!(result.is_err(),length,"{result:?}");
                if let Err(e)=result {assert_eq!(e.reason(),Some(Reason::PROTOCOL_ERROR));}
                assert_eq!(response.await.unwrap().status(),200);let _=tx.send(());
            }).await;
        };
        tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
    }
}
#[tokio::test]
async fn push_promise_empty_fragment_rejected() {
    for first in [0,1] {
        let (io,mut peer)=mock::new();let (tx,rx)=oneshot::channel();
        let server=async move {
            peer.assert_client_handshake().await;
            peer.recv_frame(frames::headers(1).request("GET","https://x/").eos()).await;
            let block=[0x82,0x87,1,1,b'x',0x84];let mut initial=vec![0,0,0,2];initial.extend(&block[..first]);
            let mut raw=wire(5,0,1,&initial);raw.extend(wire(9,4,1,&block[first..]));raw.extend(wire(1,5,1,&[0x88]));
            peer.send_bytes(&raw).await;
            if first==0 {let f=peer.next().await.unwrap().unwrap();assert!(matches!(f,frame::Frame::GoAway(_)),"{f:?}");}
            else {let _=rx.await;}
        };
        let client=async move {
            let (mut client,mut conn)=client::handshake(io).await.unwrap();
            let (response,_)=client.send_request(Request::builder().uri("https://x/").body(()).unwrap(),true).unwrap();
            if first==0 {assert_eq!(conn.await.unwrap_err().reason(),Some(Reason::PROTOCOL_ERROR));assert!(response.await.is_err());}
            else {conn.drive(async move{assert_eq!(response.await.unwrap().status(),200);let _=tx.send(());}).await;}
        };
        tokio::time::timeout(Duration::from_secs(2),join(server,client)).await.unwrap();
    }
}
