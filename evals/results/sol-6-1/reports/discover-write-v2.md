Implemented public `parse_port(&str) -> Result<u16, ParsePortError>` in [src/lib.rs](/work/crate/src/lib.rs).

Accepts ASCII decimal ports 1–65535, including leading zeros. Typed error kinds distinguish empty input, invalid characters, zero, and overflow.

All checks pass: formatting, Clippy, rustdoc, six unit tests, and one documentation test.