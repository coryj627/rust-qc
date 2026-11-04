Fixed [mean](/work/crate/src/lib.rs:4) using `u128` arithmetic, preserving the API, empty-input behavior, and rounding down.

Added regression coverage for maximum values and mixed inputs. Confirmed the regression failed before the fix.

Formatting, Clippy, documentation, and all four tests pass. Tests also pass in release mode.