//! Raw ZIP builder for crafted archives.
use std::io::Write;
#[derive(Clone)]
pub struct E {
    pub name: Vec<u8>, pub data: Vec<u8>, pub method: u16, pub flags: u16,
    pub extra_local: Vec<u8>, pub extra_central: Vec<u8>, pub comment: Vec<u8>,
    pub ext_attr: u32, pub made_by: u16, pub crc: Option<u32>,
    pub usize32: Option<u32>, pub csize32: Option<u32>, pub date: u16, pub time: u16,
}
impl E {
    pub fn new(name: &str, data: &[u8]) -> E {
        E { name: name.as_bytes().to_vec(), data: data.to_vec(), method: 0, flags: 0,
            extra_local: vec![], extra_central: vec![], comment: vec![], ext_attr: 0,
            made_by: 0x031e, crc: None, usize32: None, csize32: None, date: 0x5021, time: 0x6000 }
    }
    pub fn mode(mut self, m: u32) -> E { self.ext_attr = m << 16; self }
}
pub fn crc(d: &[u8]) -> u32 { let mut h = crc32fast::Hasher::new(); h.update(d); h.finalize() }
pub fn field(id: u16, body: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec(); v.extend((body.len() as u16).to_le_bytes()); v.extend(body); v
}
/// Returns (local section + data, central dir bytes)
pub fn sections(es: &[E], base: u32) -> (Vec<u8>, Vec<u8>) {
    let mut out = vec![]; let mut cd = vec![];
    for e in es {
        let off = base + out.len() as u32;
        let c = e.crc.unwrap_or_else(|| crc(&e.data));
        let cs = e.csize32.unwrap_or(e.data.len() as u32);
        let us = e.usize32.unwrap_or(e.data.len() as u32);
        out.extend(0x04034b50u32.to_le_bytes());
        out.extend(20u16.to_le_bytes()); out.extend(e.flags.to_le_bytes()); out.extend(e.method.to_le_bytes());
        out.extend(e.time.to_le_bytes()); out.extend(e.date.to_le_bytes()); out.extend(c.to_le_bytes());
        out.extend(cs.to_le_bytes()); out.extend(us.to_le_bytes());
        out.extend((e.name.len() as u16).to_le_bytes()); out.extend((e.extra_local.len() as u16).to_le_bytes());
        out.extend(&e.name); out.extend(&e.extra_local); out.extend(&e.data);
        cd.extend(0x02014b50u32.to_le_bytes());
        cd.extend(e.made_by.to_le_bytes()); cd.extend(20u16.to_le_bytes()); cd.extend(e.flags.to_le_bytes()); cd.extend(e.method.to_le_bytes());
        cd.extend(e.time.to_le_bytes()); cd.extend(e.date.to_le_bytes()); cd.extend(c.to_le_bytes());
        cd.extend(cs.to_le_bytes()); cd.extend(us.to_le_bytes());
        cd.extend((e.name.len() as u16).to_le_bytes()); cd.extend((e.extra_central.len() as u16).to_le_bytes()); cd.extend((e.comment.len() as u16).to_le_bytes());
        cd.extend(0u16.to_le_bytes()); cd.extend(0u16.to_le_bytes()); cd.extend(e.ext_attr.to_le_bytes()); cd.extend(off.to_le_bytes());
        cd.extend(&e.name); cd.extend(&e.extra_central); cd.extend(&e.comment);
    }
    (out, cd)
}
pub fn eocd(n: u16, cdsize: u32, cdoff: u32, comment: &[u8]) -> Vec<u8> {
    let mut v = 0x06054b50u32.to_le_bytes().to_vec();
    v.extend([0u8; 4]); v.extend(n.to_le_bytes()); v.extend(n.to_le_bytes());
    v.extend(cdsize.to_le_bytes()); v.extend(cdoff.to_le_bytes());
    v.extend((comment.len() as u16).to_le_bytes()); v.extend(comment); v
}
pub fn build(es: &[E]) -> Vec<u8> {
    let (mut out, cd) = sections(es, 0);
    let off = out.len() as u32; out.extend(&cd);
    out.extend(eocd(es.len() as u16, cd.len() as u32, off, b"")); out
}
pub fn tempdir(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("pz-{}-{}-{tag}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&p).unwrap(); p
}
pub fn guard<F: FnOnce() -> String + std::panic::UnwindSafe>(name: &str, f: F) {
    let r = std::panic::catch_unwind(f);
    match r { Ok(s) => println!("[{name}] {s}"), Err(e) => {
        let m = e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        println!("[{name}] PANIC: {m}") } }
    std::io::stdout().flush().unwrap();
}
