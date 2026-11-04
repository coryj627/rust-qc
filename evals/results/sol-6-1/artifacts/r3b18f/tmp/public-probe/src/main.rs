use std::{io::Cursor, panic::{catch_unwind, AssertUnwindSafe}};
use bytes::BytesMut;
use quinn_proto::{coding::Codec, transport_parameters::TransportParameters, FixedLengthConnectionIdParser, PartialDecode, ProtectedHeader, Side, VarInt, DEFAULT_SUPPORTED_VERSIONS};

fn main() {
    for input in [vec![0x20, 0], vec![0x20, 1, 0x40], vec![0xc0, 0, 0, 0, 0xff, 0x04, 0xde, 0x1b, 0]] {
        let outcome = catch_unwind(|| TransportParameters::read(Side::Server, &mut input.as_slice()));
        println!("transport input {input:02x?}: {outcome:?}");
    }
    let mut seed: u64 = 0xdead_beef_1234_5678;
    let mut tp_panics = 0;
    let mut header_panics = 0;
    let mut partial_panics = 0;
    let mut varint_panics = 0;
    std::panic::set_hook(Box::new(|_| {}));
    let mut shortest_tp: Option<Vec<u8>> = None;
    for i in 0..2_000_000 {
        let len = i % 65;
        let input: Vec<u8> = (0..len).map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        }).collect();
        if catch_unwind(|| TransportParameters::read(Side::Client, &mut input.as_slice())).is_err() {
            tp_panics += 1;
            if shortest_tp.as_ref().is_none_or(|prev| input.len() < prev.len()) {
                shortest_tp = Some(input.clone());
            }
        }
        let parser = FixedLengthConnectionIdParser::new(i % 21);
        header_panics += catch_unwind(AssertUnwindSafe(|| {
            ProtectedHeader::decode(&mut Cursor::new(BytesMut::from(input.as_slice())), &parser, DEFAULT_SUPPORTED_VERSIONS, true)
        })).is_err() as usize;
        partial_panics += catch_unwind(AssertUnwindSafe(|| {
            PartialDecode::new(BytesMut::from(input.as_slice()), &parser, DEFAULT_SUPPORTED_VERSIONS, true)
        })).is_err() as usize;
        varint_panics += catch_unwind(|| VarInt::decode(&mut input.as_slice())).is_err() as usize;
    }
    println!("2M mutations: TP panics={tp_panics}, shortest={shortest_tp:02x?}, header panics={header_panics}, partial panics={partial_panics}, varint panics={varint_panics}");
}
