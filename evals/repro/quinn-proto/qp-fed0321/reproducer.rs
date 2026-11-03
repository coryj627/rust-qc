// resource: gapped fragments grow retained chunk count unbounded
// Injected into /work/reprotree and compiled as part of quinn-proto (reaches pub(crate)/pub(super)).


#[cfg(test)]
mod repro_key {
    use super::*;
    use bytes::Bytes;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    // qp-fed0321 (resource-exhaustion): Assembler::insert has no limit on the number of
    // buffered out-of-order fragments and no gap limit. A peer sending many small,
    // non-adjacent (gapped) fragments forces the assembler to retain one Buffer per
    // fragment; defragment cannot merge across gaps, so the retained chunk count grows
    // linearly with the number of gapped fragments, pinning far more memory (per-chunk
    // overhead + allocation) than the stream's byte count suggests.
    fn chunks_after(n: usize) -> (usize, usize, u64) {
        let mut a = Assembler::new();
        a.ensure_ordering(false).unwrap();
        // 1-byte fragments at offsets 0, 2, 4, ... leaving a 1-byte gap between each.
        // allocation_size mimics each byte arriving in its own ~1200-byte datagram.
        for i in 0..n as u64 {
            a.insert(i * 2, Bytes::from_static(b"x"), 1200);
        }
        (a.data.len(), a.allocated, a.end)
    }

    #[test]
    fn repro_key_fed0321_gapped_fragments_grow_unbounded() {
        let (small_chunks, _small_alloc, _) = chunks_after(2_000);
        let (big_chunks, _big_alloc, _) = chunks_after(20_000);
        // Retained chunk count tracks the number of gapped fragments, with no cap: linear.
        assert!(
            small_chunks >= 2_000,
            "expected >= 2000 retained chunks, got {small_chunks}"
        );
        assert!(
            big_chunks >= 20_000,
            "expected >= 20000 retained chunks, got {big_chunks}"
        );
        // ~10x growth in input gives ~10x growth in retained chunks.
        assert!(
            big_chunks >= small_chunks * 9,
            "chunk count did not grow ~linearly: {small_chunks} -> {big_chunks}"
        );
    }

    // qp-981d88e DIAGNOSTIC: zero-length STREAM frame in unordered mode -> empty range
    // in RangeSet. Observe whether a later overlapping insert panics.
    #[test]
    fn diag_981d88e_assembler_empty_range() {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut a = Assembler::new();
            a.ensure_ordering(false).unwrap();
            // A real fragment at [6,10)
            a.insert(6, Bytes::from_static(b"ABCD"), 4);
            // A zero-length STREAM frame at offset 5 -> recvd.replace(5..5) (empty range)
            a.insert(5, Bytes::new(), 0);
            // An overlapping fragment spanning the empty range
            a.insert(2, Bytes::from_static(b"abcde"), 5);
            a.data.len()
        }));
        println!("DIAG 981d88e assembler outcome = {outcome:?}");
    }
}
