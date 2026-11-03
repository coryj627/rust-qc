// crash: truncated max_datagram_frame_size panics on unwrap
// Injected into /work/reprotree and compiled as part of quinn-proto (reaches pub(crate)/pub(super)).


#[cfg(test)]
mod repro_key {
    use super::*;
    use crate::Side;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    // qp-655a8ad (crash-from-input): a truncated MaxDatagramFrameSize transport
    // parameter (declared length 0, so no value bytes) reaches `r.get().unwrap()`
    // on an empty buffer, where the varint decode returns Err and unwrap panics.
    #[test]
    fn repro_key_655a8ad_truncated_max_datagram_frame_size_panics() {
        // id=0x20 (MaxDatagramFrameSize), len=0x00, no value bytes.
        let buf: [u8; 2] = [0x20, 0x00];
        let res = catch_unwind(AssertUnwindSafe(|| {
            let mut slice = &buf[..];
            TransportParameters::read(Side::Server, &mut slice)
        }));
        assert!(
            res.is_err(),
            "expected a panic parsing a truncated max_datagram_frame_size at baseline; \
             instead got {:?}",
            res.ok()
        );
    }

    // qp-621e38a (wrong-behavior): the MaxDatagramFrameSize arm decodes a varint from
    // the whole remaining buffer instead of exactly `len` bytes, and never checks that
    // the value occupied its declared length. A parameter that declares length 8 but
    // encodes a 1-byte value leaves 7 bytes that the parser then interprets as further
    // transport parameters. Per RFC 9000 s7.4 / s18, each parameter's value occupies
    // exactly its declared length and surplus must be rejected (Error::Malformed).
    #[test]
    fn repro_key_621e38a_mismatched_length_smuggles_params() {
        // MaxDatagramFrameSize id=0x20, len=0x08, 8-byte value region:
        //   0x05                 -> 1-byte varint value 5 (consumes 1 of the 8)
        //   0x6A 0xB2 0x00       -> GreaseQuicBit (id 0x2AB2) len 0 -> grease=true
        //   0x08 0x02 0x40 0x64  -> InitialMaxStreamsBidi (id 0x08) len 2, value 100
        let buf: [u8; 10] = [0x20, 0x08, 0x05, 0x6A, 0xB2, 0x00, 0x08, 0x02, 0x40, 0x64];
        let mut slice = &buf[..];
        let params = TransportParameters::read(Side::Server, &mut slice)
            .expect("baseline wrongly accepts a mismatched-length transport parameter");
        assert_eq!(
            params.max_datagram_frame_size,
            Some(VarInt(5)),
            "max_datagram_frame_size parsed from only 1 of its 8 declared bytes"
        );
        assert!(
            params.grease_quic_bit,
            "a GreaseQuicBit smuggled inside the declared length took effect"
        );
        assert_eq!(
            params.initial_max_streams_bidi,
            VarInt(100),
            "an InitialMaxStreamsBidi smuggled inside the declared length took effect"
        );
    }
}
