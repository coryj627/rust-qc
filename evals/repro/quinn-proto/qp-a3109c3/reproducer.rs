// resource: empty DATAGRAM frames queue unbounded
// Injected into /work/reprotree and compiled as part of quinn-proto (reaches pub(crate)/pub(super)).


#[cfg(test)]
mod repro_key {
    use super::*;
    use crate::frame::Datagram;
    use bytes::Bytes;

    // qp-a3109c3 (resource-exhaustion): DatagramState::received charges only the payload
    // length against the receive window, so a zero-length DATAGRAM frame adds nothing to
    // `recv_buffered` and never trips the eviction loop. `incoming` has no other bound, so
    // a peer can queue an unbounded number of empty datagrams. recv_buffered stays 0 while
    // the queue grows one entry per frame, independent of the configured window.
    fn queued_after(n: usize) -> (usize, usize) {
        let mut state = DatagramState::default();
        let window = Some(1024usize);
        for _ in 0..n {
            state
                .received(Datagram { data: Bytes::new() }, &window)
                .expect("empty datagram accepted");
        }
        (state.incoming.len(), state.recv_buffered)
    }

    #[test]
    fn repro_key_a3109c3_empty_datagrams_grow_unbounded() {
        let (small_len, small_buf) = queued_after(1_000);
        let (big_len, big_buf) = queued_after(20_000);
        // The window (1024 bytes) is never consulted because 0 bytes are charged.
        assert_eq!(small_buf, 0, "recv_buffered should stay 0 for empty datagrams");
        assert_eq!(big_buf, 0, "recv_buffered should stay 0 for empty datagrams");
        // One queued entry per empty datagram, with no cap: 20x input -> 20x entries.
        assert_eq!(small_len, 1_000, "queued {small_len}, expected 1000");
        assert_eq!(big_len, 20_000, "queued {big_len}, expected 20000");
    }
}
