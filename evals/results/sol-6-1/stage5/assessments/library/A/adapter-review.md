Candidate A adapter review

Reviewed source: /work/candidates/A/src/lib.rs.
SpanError is a public enum. Its documented and implemented discriminants map as follows:
- Reversed: returned by ByteSpan::new when start > end; maps to Reversed.
- OutOfBounds: returned by slice when end > text.len(), before UTF-8 validation; maps to Bounds.
- InvalidUtf8Boundary: returned for an in-bounds endpoint inside a UTF-8 code point; maps to Utf8Boundary.

The adapter inspects only public enum discriminants and ignores payload fields. It does not parse Display/Debug or use call-site information. The enum is non_exhaustive; the required wildcard arm explicitly panics for a future unknown discriminant rather than assigning a fabricated category.
