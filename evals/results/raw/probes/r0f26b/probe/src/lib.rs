//! Raw archive builder used by the probes.
pub struct E {
    pub name: Vec<u8>,
    pub data: Vec<u8>,
    pub flags: u16,
    pub method: u16,
    pub crc: Option<u32>,
    pub comp: Option<u32>,
    pub uncomp: Option<u32>,
    pub cextra: Vec<u8>,
    pub lextra: Vec<u8>,
    pub comment: Vec<u8>,
    pub vmade: u16,
    pub ext: u32,
    pub time: u16,
    pub date: u16,
    pub offset: Option<u32>,
}
impl E {
    pub fn new(name: &str, data: &[u8]) -> E {
        E { name: name.as_bytes().to_vec(), data: data.to_vec(), flags: 0, method: 0, crc: None, comp: None, uncomp: None,
            cextra: vec![], lextra: vec![], comment: vec![], vmade: 0x031e, ext: 0o100644 << 16, time: 0x6000, date: 0x5021, offset: None }
    }
    pub fn unix(mut self, mode: u32) -> E { self.ext = mode << 16; self.vmade = 0x031e; self }
}
pub fn crc(d: &[u8]) -> u32 { let mut h = crc32fast::Hasher::new(); h.update(d); h.finalize() }
pub fn ext(id: u16, body: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec(); v.extend((body.len() as u16).to_le_bytes()); v.extend(body); v
}
pub fn build(es: &[E]) -> Vec<u8> { build_c(es, b"") }
pub fn build_c(es: &[E], comment: &[u8]) -> Vec<u8> {
    let mut out = vec![]; let mut offs = vec![];
    for e in es {
        offs.push(out.len() as u32);
        let c = e.crc.unwrap_or_else(|| crc(&e.data));
        out.extend(0x04034b50u32.to_le_bytes());
        out.extend(20u16.to_le_bytes());
        out.extend(e.flags.to_le_bytes());
        out.extend(e.method.to_le_bytes());
        out.extend(e.time.to_le_bytes());
        out.extend(e.date.to_le_bytes());
        out.extend(c.to_le_bytes());
        out.extend(e.comp.unwrap_or(e.data.len() as u32).to_le_bytes());
        out.extend(e.uncomp.unwrap_or(e.data.len() as u32).to_le_bytes());
        out.extend((e.name.len() as u16).to_le_bytes());
        out.extend((e.lextra.len() as u16).to_le_bytes());
        out.extend(&e.name); out.extend(&e.lextra); out.extend(&e.data);
    }
    let cd = out.len() as u32;
    for (i, e) in es.iter().enumerate() {
        let c = e.crc.unwrap_or_else(|| crc(&e.data));
        out.extend(0x02014b50u32.to_le_bytes());
        out.extend(e.vmade.to_le_bytes());
        out.extend(20u16.to_le_bytes());
        out.extend(e.flags.to_le_bytes());
        out.extend(e.method.to_le_bytes());
        out.extend(e.time.to_le_bytes());
        out.extend(e.date.to_le_bytes());
        out.extend(c.to_le_bytes());
        out.extend(e.comp.unwrap_or(e.data.len() as u32).to_le_bytes());
        out.extend(e.uncomp.unwrap_or(e.data.len() as u32).to_le_bytes());
        out.extend((e.name.len() as u16).to_le_bytes());
        out.extend((e.cextra.len() as u16).to_le_bytes());
        out.extend((e.comment.len() as u16).to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(e.ext.to_le_bytes());
        out.extend(e.offset.unwrap_or(offs[i]).to_le_bytes());
        out.extend(&e.name); out.extend(&e.cextra); out.extend(&e.comment);
    }
    let sz = out.len() as u32 - cd;
    out.extend(0x06054b50u32.to_le_bytes());
    out.extend(0u16.to_le_bytes()); out.extend(0u16.to_le_bytes());
    out.extend((es.len() as u16).to_le_bytes()); out.extend((es.len() as u16).to_le_bytes());
    out.extend(sz.to_le_bytes()); out.extend(cd.to_le_bytes());
    out.extend((comment.len() as u16).to_le_bytes()); out.extend(comment);
    out
}
