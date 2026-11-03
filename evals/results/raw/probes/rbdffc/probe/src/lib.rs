#![allow(dead_code)]
#[derive(Clone)]
pub struct E { pub name: Vec<u8>, pub method: u16, pub flags: u16, pub vmade: u16, pub attr: u32, pub crc: u32, pub csize: u32, pub usize_: u32, pub data: Vec<u8>, pub cx: Vec<u8>, pub lx: Vec<u8>, pub comment: Vec<u8>, pub time: u16, pub date: u16 }
pub fn crc(d: &[u8]) -> u32 { let mut h = crc32fast::Hasher::new(); h.update(d); h.finalize() }
pub fn e(name: &str, data: &[u8]) -> E { E { name: name.as_bytes().to_vec(), method: 0, flags: 0, vmade: 0x031E, attr: 0o100644u32 << 16, crc: crc(data), csize: data.len() as u32, usize_: data.len() as u32, data: data.to_vec(), cx: vec![], lx: vec![], comment: vec![], time: 0, date: 0x4A21 } }
fn p16(v: &mut Vec<u8>, x: u16) { v.extend_from_slice(&x.to_le_bytes()) }
fn p32(v: &mut Vec<u8>, x: u32) { v.extend_from_slice(&x.to_le_bytes()) }
pub fn local(v: &mut Vec<u8>, x: &E) {
    p32(v, 0x04034b50); p16(v, 20); p16(v, x.flags); p16(v, x.method); p16(v, x.time); p16(v, x.date); p32(v, x.crc); p32(v, x.csize); p32(v, x.usize_); p16(v, x.name.len() as u16); p16(v, x.lx.len() as u16); v.extend_from_slice(&x.name); v.extend_from_slice(&x.lx); v.extend_from_slice(&x.data);
}
pub fn central(v: &mut Vec<u8>, x: &E, off: u32) {
    p32(v, 0x02014b50); p16(v, x.vmade); p16(v, 20); p16(v, x.flags); p16(v, x.method); p16(v, x.time); p16(v, x.date); p32(v, x.crc); p32(v, x.csize); p32(v, x.usize_); p16(v, x.name.len() as u16); p16(v, x.cx.len() as u16); p16(v, x.comment.len() as u16); p16(v, 0); p16(v, 0); p32(v, x.attr); p32(v, off); v.extend_from_slice(&x.name); v.extend_from_slice(&x.cx); v.extend_from_slice(&x.comment);
}
pub fn eocd(v: &mut Vec<u8>, n: u16, size: u32, off: u32, comment: &[u8]) {
    p32(v, 0x06054b50); p16(v, 0); p16(v, 0); p16(v, n); p16(v, n); p32(v, size); p32(v, off); p16(v, comment.len() as u16); v.extend_from_slice(comment);
}
pub fn build(es: &[E]) -> Vec<u8> {
    let mut v = Vec::new(); let mut offs = vec![];
    for x in es { offs.push(v.len() as u32); local(&mut v, x); }
    let cs = v.len() as u32;
    for (x, o) in es.iter().zip(&offs) { central(&mut v, x, *o); }
    let sz = v.len() as u32 - cs;
    eocd(&mut v, es.len() as u16, sz, cs, b""); v
}
pub fn zip64x(unc: u64, comp: u64) -> Vec<u8> { let mut v = vec![]; p16(&mut v, 1); p16(&mut v, 16); v.extend_from_slice(&unc.to_le_bytes()); v.extend_from_slice(&comp.to_le_bytes()); v }
pub struct Bits { pub out: Vec<u8>, cur: u64, n: u32 }
impl Bits { pub fn new() -> Self { Bits { out: vec![], cur: 0, n: 0 } }
    pub fn put(&mut self, val: u32, bits: u32) { self.cur |= (val as u64) << self.n; self.n += bits; while self.n >= 8 { self.out.push(self.cur as u8); self.cur >>= 8; self.n -= 8; } }
    pub fn finish(mut self) -> Vec<u8> { if self.n > 0 { self.out.push(self.cur as u8); } self.out } }
pub fn tmp(tag: &str) -> std::path::PathBuf { let p = std::env::temp_dir().join(format!("p{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), tag)); std::fs::create_dir_all(&p).unwrap(); p }
pub struct Rng(pub u64);
impl Rng { pub fn next(&mut self) -> u64 { let mut x = self.0; x ^= x << 13; x ^= x >> 7; x ^= x << 17; self.0 = x; x } pub fn below(&mut self, n: u64) -> u64 { self.next() % n } }
