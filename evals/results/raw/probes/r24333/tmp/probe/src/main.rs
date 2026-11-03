use quinn_proto::{Side, transport_parameters::TransportParameters};
use std::panic::catch_unwind;
fn try_it(name: &str, side: Side, b: &[u8]) {
    let bb = b.to_vec();
    let r = catch_unwind(move || { let mut s = &bb[..]; let r = TransportParameters::read(side, &mut s); (format!("{:?}", r.map(|_| "ok")), s.len()) });
    println!("{name}: {:?} -> {:?}", b, r.map_err(|_| "PANIC"));
}
fn main() {
    try_it("max_datagram len0 at end", Side::Server, &[0x20, 0x00]);
    try_it("min_ack_delay len0 at end", Side::Server, &[0x40, 0xde, 0x1a, 0x00]);
    try_it("min_ack_delay draft07 varint", Side::Server, &[0x80, 0xde, 0x1a, 0x00, 0x00]);
    try_it("max_datagram len2 with 1-byte value then param", Side::Server, &[0x20, 0x02, 0x05, 0x0c, 0x00]);
    try_it("dup grease", Side::Server, &[0x6a,0xb2,0,0x6a,0xb2,0]);
    // exhaustive short inputs
    std::panic::set_hook(Box::new(|_| {}));
    let mut panics = 0u64; let mut first=None;
    for a in 0u16..256 { for b in 0u16..256 { for c in 0u16..256 {
        let v=[a as u8,b as u8,c as u8];
        for l in 1..=3 { let vv=v[..l].to_vec(); let vv2=vv.clone();
            if catch_unwind(move || { let mut s=&vv[..]; let _=TransportParameters::read(Side::Client,&mut s);}).is_err(){ panics+=1; if first.is_none(){first=Some(vv2);} }
        }
    }}}
    println!("panics {panics} first {first:?}");
}
