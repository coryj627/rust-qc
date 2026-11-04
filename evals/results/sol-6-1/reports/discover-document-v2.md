Added rustdoc to [saturating_seconds](/work/crate/src/lib.rs:3) explaining minutes-to-seconds units and saturation at `u64::MAX` without wrapping or panicking. Two runnable examples cover normal conversion and overflow boundaries.

Implementation and API are unchanged. Formatting, Clippy, rustdoc, and both doctests pass.