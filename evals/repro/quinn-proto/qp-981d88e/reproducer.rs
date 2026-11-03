// wrong: empty-range replace corrupts RangeSet invariant (zero-len STREAM frame)
// Injected into /work/reprotree and compiled as part of quinn-proto (reaches pub(crate)/pub(super)).


#[cfg(test)]
mod repro_key2 {
    use super::*;

    // qp-981d88e (observed as wrong-behavior at baseline): replacing an empty range
    // (as happens when a zero-length STREAM frame reaches the unordered-read assembler,
    // which calls recvd.replace(offset..offset)) makes Replace::drop store an empty
    // (start==end) entry. This violates the RangeSet invariant that it holds no empty
    // ranges and no pair of ranges that could be simplified. A subsequent overlapping
    // replace then fails to merge correctly. The advisory (RUSTSEC-2026-0338) describes
    // the downstream consequence as a connection panic / DoS; this harness demonstrates
    // the invariant corruption that causes it, matching the maintainers' own fix test.
    #[test]
    fn repro_key_981d88e_empty_range_corrupts_rangeset() {
        let mut set = RangeSet::new();
        set.replace(3..3); // empty range -> stores (3,3)
        set.replace(6..10);
        set.replace(5..5); // empty range -> stores (5,5)
        set.replace(2..7); // overlaps 6..10, should merge to (2,10)
        let contents: Vec<(u64, u64)> = set.0.iter().map(|(&s, &e)| (s, e)).collect();
        // Correct per RangeSet's documented invariant: a single merged range [2,10).
        let correct = vec![(2u64, 10u64)];
        assert_ne!(
            contents, correct,
            "baseline should NOT produce the correct merged set"
        );
        // Baseline actually leaves a stray empty range and an unmerged overlap.
        assert!(
            contents.iter().any(|&(s, e)| s == e),
            "expected a stray empty range at baseline; got {contents:?}"
        );
        assert!(
            contents.len() > 1,
            "expected unmerged/corrupted ranges at baseline; got {contents:?}"
        );
    }
}
