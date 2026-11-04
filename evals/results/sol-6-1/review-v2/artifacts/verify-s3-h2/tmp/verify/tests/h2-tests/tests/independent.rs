use h2_support::prelude::*;
use futures::StreamExt;
use std::{pin::Pin, task::{Context, Poll}, io, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn request() -> Request<()> { Request::builder().uri("https://example.com/").body(()).unwrap() }

#[tokio::test]
async fn pushed_early_hints() {
 let (io, mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::push_promise(1,2).request("GET","https://example.com/pushed")).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  srv.send_frame(frames::headers(2).response(103)).await;
  srv.send_frame(frames::headers(2).response(200).eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client, mut conn)=client::handshake(io).await.unwrap();
  let (mut response,_)=client.send_request(request(),true).unwrap();
  let mut pushes=response.push_promises();
  conn.drive(async move {
   let push=pushes.next().await.unwrap().unwrap();
   let (_,response)=push.into_parts();
   assert_eq!(response.await.unwrap().status(),200);
  }).await;
 };
 join(peer,app).await;
}

#[tokio::test]
async fn pushed_head_length() {
 let (io, mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::push_promise(1,2).request("HEAD","https://example.com/pushed")).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  srv.send_frame(frames::headers(2).response(200).field("content-length","100").eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client, mut conn)=client::handshake(io).await.unwrap();
  let (mut response,_)=client.send_request(request(),true).unwrap();
  let mut pushes=response.push_promises();
  conn.drive(async move {
   let push=pushes.next().await.unwrap().unwrap();
   let (req,response)=push.into_parts();
   assert_eq!(req.method(),Method::HEAD);
   let result=response.await;
   assert!(result.is_ok(),"pushed HEAD rejected: {:?}",result);
  }).await;
 };
 join(peer,app).await;
}

async fn not_modified(empty_data: bool) {
 let (io, mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  let h=frames::headers(1).response(304).field("content-length","100");
  if empty_data { srv.send_frame(h).await; srv.send_frame(frames::data(1,"").eos()).await; }
  else { srv.send_frame(h.eos()).await; }
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client, mut conn)=client::handshake(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  conn.drive(async move {
   let response=response.await.unwrap(); assert_eq!(response.status(),304);
   let mut body=response.into_body();
   while let Some(data)=body.data().await { assert!(data.unwrap().is_empty()); }
  }).await;
 };
 join(peer,app).await;
}
#[tokio::test] async fn response_304_empty_data() {not_modified(true).await;}
#[tokio::test] async fn response_304_headers_control() {not_modified(false).await;}

#[tokio::test]
async fn informational_response_waits_for_final() {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(103)).await;
  idle_ms(20).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::handshake(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  assert_eq!(conn.drive(response).await.unwrap().status(),200);
 };
 join(peer,app).await;
}

#[tokio::test]
async fn empty_body_preserves_trailers() {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200)).await;
  srv.send_frame(frames::headers(1).field("x-result","ok").eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::handshake(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  conn.drive(async move {
   let mut body=response.await.unwrap().into_body();
   assert!(body.data().await.is_none());
   assert_eq!(body.trailers().await.unwrap().unwrap()["x-result"],"ok");
  }).await;
 };
 join(peer,app).await;
}

#[tokio::test]
async fn missing_status_is_rejected() {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).field("content-length","0").eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::handshake(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  let result=conn.drive(response).await;
  assert!(result.is_err(),"missing status accepted: {:?}",result);
 };
 join(peer,app).await;
}

#[tokio::test]
async fn oversized_trailers_are_rejected() {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200)).await;
  srv.send_frame(frames::headers(1).field("checksum","x".repeat(100)).eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::Builder::new().max_header_list_size(64).handshake::<_,Bytes>(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  conn.drive(async move {
   let mut body=response.await.unwrap().into_body();
   assert!(body.data().await.is_none());
   let result=body.trailers().await;
   assert!(result.is_err(),"oversized trailers accepted: {:?}",result);
  }).await;
 };
 join(peer,app).await;
}

async fn header_limit(limit:u32) {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::Builder::new().max_header_list_size(limit).handshake::<_,Bytes>(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  let result=conn.drive(response).await;
  assert!(result.is_ok(),"limit {limit}, decoded size 42: {:?}",result);
 };
 join(peer,app).await;
}
#[tokio::test] async fn exact_header_limit() {header_limit(42).await;}
#[tokio::test] async fn header_limit_control() {header_limit(43).await;}

#[tokio::test]
async fn reservation_four_gib() {
 let (io,_peer)=mock::new();
 let (mut client,_conn)=client::handshake(io).await.unwrap();
 let (_response,mut stream)=client.send_request(request(),false).unwrap();
 stream.reserve_capacity(65535);
 assert_eq!(stream.capacity(),65535);
 stream.reserve_capacity(1usize<<32);
 assert!(stream.capacity()>0,"4 GiB cancelled capacity: {}",stream.capacity());
}

#[tokio::test]
async fn duplicate_ping_is_error() {
 let (io,_peer)=mock::new();
 let (_client,mut conn)=client::handshake(io).await.unwrap();
 let mut pp=conn.ping_pong().unwrap();
 assert!(pp.send_ping(Ping::opaque()).is_ok());
 assert!(pp.send_ping(Ping::opaque()).is_err());
}

#[derive(Debug)]
struct ZeroThenError {calls:Arc<AtomicUsize>}
impl AsyncRead for ZeroThenError {
 fn poll_read(self:Pin<&mut Self>,_cx:&mut Context<'_>,_buf:&mut tokio_io::ReadBuf<'_>)->Poll<io::Result<()>> {Poll::Pending}
}
impl AsyncWrite for ZeroThenError {
 fn poll_write(self:Pin<&mut Self>,_cx:&mut Context<'_>,buf:&[u8])->Poll<io::Result<usize>> {
  assert!(!buf.is_empty());
  if self.calls.fetch_add(1,Ordering::SeqCst)==0 {Poll::Ready(Ok(0))}
  else {Poll::Ready(Err(io::Error::new(io::ErrorKind::Other,"retry sentinel")))}
 }
 fn poll_flush(self:Pin<&mut Self>,_cx:&mut Context<'_>)->Poll<io::Result<()>> {Poll::Ready(Ok(()))}
 fn poll_shutdown(self:Pin<&mut Self>,_cx:&mut Context<'_>)->Poll<io::Result<()>> {Poll::Ready(Ok(()))}
}
#[tokio::test]
async fn zero_write_reports_write_zero() {
 let calls=Arc::new(AtomicUsize::new(0));
 let result=server::handshake(ZeroThenError{calls:calls.clone()}).await;
 let err=result.unwrap_err();
 assert_eq!(err.get_io().unwrap().kind(),io::ErrorKind::WriteZero,"writes={}, error={err}",calls.load(Ordering::SeqCst));
}

async fn padding_case(padded:bool) {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake().await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200)).await;
  if padded {
   let mut payload=vec![14,b'x']; payload.extend_from_slice(&[0;14]);
   srv.send_frame(frames::data(1,payload).padded()).await;
  } else {srv.send_frame(frames::data(1,vec![b'x';16])).await;}
  let update=tokio::time::timeout(Duration::from_millis(200),srv.next()).await;
  assert!(update.is_ok(),"no WINDOW_UPDATE after releasing all body bytes; padded={}",padded);
  let update=update.unwrap().unwrap().unwrap();
  match update {h2::frame::Frame::WindowUpdate(w)=>assert_eq!(w.stream_id(),StreamId::from(1)), f=>panic!("unexpected frame: {:?}",f)}
  idle_ms(300).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::Builder::new().initial_window_size(16).handshake::<_,Bytes>(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  conn.drive(async move {
   let mut body=response.await.unwrap().into_body();
   let data=body.data().await.unwrap().unwrap();
   assert_eq!(data.len(),if padded {1}else{16});
   eprintln!("padded={padded}, available={}, used={}",body.flow_control().available_capacity(),body.flow_control().used_capacity());
   body.flow_control().release_capacity(data.len()).unwrap();
   idle_ms(250).await;
  }).await;
 };
 tokio::time::timeout(Duration::from_secs(2),join(peer,app)).await.unwrap();
}
#[tokio::test] async fn padding_returns_credit() {padding_case(true).await;}
#[tokio::test] async fn unpadded_credit_control() {padding_case(false).await;}

#[tokio::test]
async fn zero_stream_window_blocks_data() {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake_with_settings(frames::settings().initial_window_size(0)).await;
  // Warmup confirms SETTINGS processed before the POST is created.
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  srv.recv_frame(frames::headers(3).request("POST","https://example.com/")).await;
  assert!(tokio::time::timeout(Duration::from_millis(100),srv.next()).await.is_err(),"DATA sent with no stream credit");
  srv.send_frame(frames::window_update(3,3)).await;
  srv.recv_frame(frames::data(3,"abc").eos()).await;
  srv.send_frame(frames::headers(3).response(200).eos()).await;
  idle_ms(10).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::handshake(io).await.unwrap();
  let (warm,_)=client.send_request(request(),true).unwrap();
  conn.drive(warm).await.unwrap();
  client=conn.drive(client.ready()).await.unwrap();
  let (response,mut stream)=client.send_request(Request::builder().method("POST").uri("https://example.com/").body(()).unwrap(),false).unwrap();
  stream.send_data(Bytes::from_static(b"abc"),true).unwrap();
  conn.drive(response).await.unwrap();
 };
 tokio::time::timeout(Duration::from_secs(2),join(peer,app)).await.unwrap();
}

async fn queued_case(reserve:bool) {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  srv.recv_frame(frames::headers(3).request("POST","https://example.com/")).await;
  srv.recv_frame(frames::data(3,"a").eos()).await;
  srv.send_frame(frames::headers(3).response(200).eos()).await;
  idle_ms(20).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::handshake(io).await.unwrap();
  let (warm,_)=client.send_request(request(),true).unwrap();
  conn.drive(warm).await.unwrap();
  client=conn.drive(client.ready()).await.unwrap();
  let post=||Request::builder().method("POST").uri("https://example.com/").body(()).unwrap();
  let (response,mut active)=client.send_request(post(),false).unwrap();
  client=conn.drive(client.ready()).await.unwrap();
  let (_queued_response,mut queued)=client.send_request(post(),false).unwrap();
  if reserve {queued.reserve_capacity(65535); assert_eq!(queued.capacity(),65535);}
  queued.send_data(Bytes::from(vec![0;65535]),true).unwrap();
  eprintln!("reserve={reserve}, queued capacity={}",queued.capacity());
  active.send_data(Bytes::from_static(b"a"),true).unwrap();
  conn.drive(response).await.unwrap();
 };
 assert!(tokio::time::timeout(Duration::from_millis(500),join(peer,app)).await.is_ok(),"active request blocked; queued explicit reserve={}",reserve);
}
#[tokio::test] async fn queued_reservation_allows_active() {queued_case(true).await;}
#[tokio::test] async fn queued_without_reserve_control() {queued_case(false).await;}

#[tokio::test]
async fn unknown_setting_is_ignored() {
 let (io,mut srv)=mock::new();
 let peer=async move {
  srv.send_bytes(&[0,0,6,4,0,0,0,0,0, 0x0a,0xbc,0,0,0,7]).await;
  srv.read_preface().await.unwrap();
  let initial=srv.next().await.unwrap().unwrap();
  assert!(matches!(initial,h2::frame::Frame::Settings(_)));
  srv.send_frame(frames::settings_ack()).await;
  srv.recv_frame(frames::settings_ack()).await;
  srv.recv_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  srv.send_frame(frames::headers(1).response(200).eos()).await;
  idle_ms(10).await;
 };
 let app=async move {
  let (mut client,mut conn)=client::handshake(io).await.unwrap();
  let (response,_)=client.send_request(request(),true).unwrap();
  assert_eq!(conn.drive(response).await.unwrap().status(),200);
 };
 tokio::time::timeout(Duration::from_secs(2),join(peer,app)).await.unwrap();
}

#[derive(Debug)]
struct OsErrorReader;
impl AsyncRead for OsErrorReader {
 fn poll_read(self:Pin<&mut Self>,_cx:&mut Context<'_>,_buf:&mut tokio_io::ReadBuf<'_>)->Poll<io::Result<()>> {Poll::Ready(Err(io::Error::from_raw_os_error(5)))}
}
impl AsyncWrite for OsErrorReader {
 fn poll_write(self:Pin<&mut Self>,_cx:&mut Context<'_>,buf:&[u8])->Poll<io::Result<usize>> {Poll::Ready(Ok(buf.len()))}
 fn poll_flush(self:Pin<&mut Self>,_cx:&mut Context<'_>)->Poll<io::Result<()>> {Poll::Ready(Ok(()))}
 fn poll_shutdown(self:Pin<&mut Self>,_cx:&mut Context<'_>)->Poll<io::Result<()>> {Poll::Ready(Ok(()))}
}
#[tokio::test]
async fn connection_preserves_os_error() {
 let (_client,conn)=client::handshake(OsErrorReader).await.unwrap();
 let err=conn.await.unwrap_err();
 assert_eq!(err.get_io().unwrap().raw_os_error(),Some(5),"converted I/O error: {:?}",err);
}

async fn invalid_push_cleanup(post:bool) {
 let (io,mut cli)=mock::new();
 let peer=async move {
  cli.assert_server_handshake().await;
  cli.send_frame(frames::headers(1).request("GET","https://example.com/").eos()).await;
  cli.recv_frame(frames::headers(1).response(200).eos()).await;
  cli.send_frame(frames::headers(3).request("GET","https://example.com/").eos()).await;
  idle_ms(50).await;
 };
 let app=async move {
  let mut conn=server::handshake(io).await.unwrap();
  let (req,mut respond)=conn.accept().await.unwrap().unwrap();
  let before=conn.num_wired_streams(); assert_eq!(before,1);
  let rejected=if post {Request::builder().method("POST").uri("https://example.com/push").body(()).unwrap()}
   else {Request::builder().uri("https://example.com/push").header("connection","close").body(()).unwrap()};
  assert!(respond.push_request(rejected).is_err());
  eprintln!("invalid POST={post}, streams before={before}, after rejection={}",conn.num_wired_streams());
  respond.send_response(Response::new(()),true).unwrap();
  drop(respond); drop(req);
  let (_next_req,_next_respond)=conn.accept().await.unwrap().unwrap();
  assert_eq!(conn.num_wired_streams(),1,"rejected POST={post}, stale child survives parent completion");
 };
 tokio::time::timeout(Duration::from_secs(2),join(peer,app)).await.unwrap();
}
#[tokio::test] async fn invalid_post_push_cleans_state() {invalid_push_cleanup(true).await;}
#[tokio::test] async fn invalid_connection_header_push_control() {invalid_push_cleanup(false).await;}
