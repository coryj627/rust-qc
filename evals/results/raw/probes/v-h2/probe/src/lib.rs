pub use futures::StreamExt;
pub use h2_support::mock::Handle;
pub use h2_support::prelude::*;
pub use std::sync::{Arc, Mutex};
pub use std::time::{Duration, Instant};
pub use tokio::time::{sleep, timeout};

pub fn f(ty: u8, flags: u8, id: u32, p: &[u8]) -> Vec<u8> {
    let l = p.len() as u32;
    let mut v = vec![(l >> 16) as u8, (l >> 8) as u8, l as u8, ty, flags];
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(p);
    v
}
pub const DATA: u8 = 0;
pub const HEADERS: u8 = 1;
pub const RST: u8 = 3;
pub const PP: u8 = 5;
pub const GOAWAY: u8 = 7;
pub const WU: u8 = 8;
pub const CONT: u8 = 9;
pub const ES: u8 = 1;
pub const EH: u8 = 4;
pub const PADDED: u8 = 8;
pub const PRIO: u8 = 0x20;

pub fn hint(first: u8, bits: u8, mut val: usize) -> Vec<u8> {
    let max = (1usize << bits) - 1;
    if val < max {
        return vec![first | val as u8];
    }
    let mut v = vec![first | max as u8];
    val -= max;
    while val >= 128 {
        v.push((val % 128 + 128) as u8);
        val /= 128;
    }
    v.push(val as u8);
    v
}
fn s(x: &str) -> Vec<u8> {
    let mut v = hint(0, 7, x.len());
    v.extend_from_slice(x.as_bytes());
    v
}
/// literal without indexing, new name
pub fn lit(n: &str, val: &str) -> Vec<u8> {
    let mut v = vec![0u8];
    v.extend(s(n));
    v.extend(s(val));
    v
}
/// literal with incremental indexing, new name
pub fn liti(n: &str, val: &str) -> Vec<u8> {
    let mut v = vec![0x40u8];
    v.extend(s(n));
    v.extend(s(val));
    v
}
pub fn block(fields: &[(&str, &str)]) -> Vec<u8> {
    let mut v = vec![];
    for (n, val) in fields {
        v.extend(lit(n, val));
    }
    v
}
pub fn req(extra: &[(&str, &str)]) -> Vec<u8> {
    let mut v = block(&[(":method", "GET"), (":scheme", "https"), (":path", "/"), (":authority", "a.test")]);
    v.extend(block(extra));
    v
}
pub fn resp(status: &str, extra: &[(&str, &str)]) -> Vec<u8> {
    let mut v = block(&[(":status", status)]);
    v.extend(block(extra));
    v
}
pub fn pp_payload(promised: u32, b: &[u8]) -> Vec<u8> {
    let mut v = promised.to_be_bytes().to_vec();
    v.extend_from_slice(b);
    v
}
pub fn pp(parent: u32, promised: u32) -> Vec<u8> {
    f(PP, EH, parent, &pp_payload(promised, &req(&[])))
}
pub fn rst(id: u32, reason: u32) -> Vec<u8> {
    f(RST, 0, id, &reason.to_be_bytes())
}
pub fn wu(id: u32, inc: u32) -> Vec<u8> {
    f(WU, 0, id, &inc.to_be_bytes())
}
pub fn goaway(id: u32, last: u32, reason: u32) -> Vec<u8> {
    let mut p = last.to_be_bytes().to_vec();
    p.extend_from_slice(&reason.to_be_bytes());
    f(GOAWAY, 0, id, &p)
}
/// Collect frames until idle for ms
pub async fn dump(h: &mut Handle, ms: u64) -> Vec<String> {
    let mut out = vec![];
    loop {
        match timeout(Duration::from_millis(ms), h.next()).await {
            Err(_) => {
                out.push("<idle>".to_string());
                break;
            }
            Ok(None) => {
                out.push("<eof>".to_string());
                break;
            }
            Ok(Some(Ok(fr))) => {
                let s = format!("{:?}", fr);
                out.push(if s.len() > 200 { s[..200].to_string() } else { s });
            }
            Ok(Some(Err(e))) => {
                out.push(format!("<codec err {:?}>", e));
                break;
            }
        }
    }
    out
}
pub type Log = Arc<Mutex<Vec<String>>>;
pub fn log() -> Log {
    Arc::new(Mutex::new(vec![]))
}
pub fn lg(l: &Log, s: String) {
    l.lock().unwrap().push(s);
}
pub fn show(name: &str, frames: &[String], l: &Log) {
    println!("== {name}\n  frames: {:#?}\n  log: {:#?}", frames, l.lock().unwrap());
}
/// Default server: accept loop, log requests, answer 200 eos, holding body
pub fn spawn_srv(b: server::Builder, io: mock::Mock, l: Log) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut c = match b.handshake::<_, Bytes>(io).await {
            Ok(c) => c,
            Err(e) => {
                lg(&l, format!("handshake err {e:?}"));
                return;
            }
        };
        while let Some(r) = c.accept().await {
            match r {
                Ok((req, mut sr)) => {
                    lg(&l, format!("REQ {} uri={:?} headers={:?}", req.method(), req.uri().to_string(), req.headers()));
                    let _ = sr.send_response(Response::new(()), true);
                    let l2 = l.clone();
                    tokio::spawn(async move {
                        let mut body = req.into_body();
                        let mut n = 0;
                        while let Some(d) = body.data().await {
                            match d {
                                Ok(d) => {
                                    n += d.len();
                                    let _ = body.flow_control().release_capacity(d.len());
                                }
                                Err(e) => {
                                    lg(&l2, format!("body err {e:?}"));
                                    return;
                                }
                            }
                        }
                        lg(&l2, format!("body end n={n}"));
                    });
                }
                Err(e) => {
                    lg(&l, format!("accept err {e:?}"));
                    break;
                }
            }
        }
        lg(&l, "conn done".into());
    })
}
pub fn req_get(uri: &str) -> Request<()> {
    Request::builder().uri(uri).body(()).unwrap()
}
pub struct Cli {
    pub c: client::SendRequest<Bytes>,
    pub s: Handle,
    pub j: tokio::task::JoinHandle<()>,
    pub l: Log,
}
pub async fn cli(b: client::Builder) -> Cli {
    cli_set(b, frame_settings()).await
}
pub fn frame_settings() -> h2::frame::Settings {
    Default::default()
}
pub async fn cli_set(b: client::Builder, st: h2::frame::Settings) -> Cli {
    let (io, mut s) = mock::new();
    let l = log();
    let l2 = l.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let j = tokio::spawn(async move {
        let (c, conn) = b.handshake::<_, Bytes>(io).await.unwrap();
        let _ = tx.send(c);
        let r = conn.await;
        lg(&l2, format!("conn result {r:?}"));
    });
    s.assert_client_handshake_with_settings(st).await;
    let c = rx.await.unwrap();
    Cli { c, s, j, l }
}
pub fn panicked(j: &tokio::task::JoinHandle<()>) -> bool {
    j.is_finished()
}
pub async fn report(name: &str, frames: &[String], c: Cli) {
    let Cli { j, l, .. } = c;
    sleep(Duration::from_millis(20)).await;
    if j.is_finished() {
        if let Err(e) = j.await {
            if e.is_panic() {
                lg(&l, "CONN TASK PANICKED".into());
            }
        }
    }
    show(name, frames, &l);
}
