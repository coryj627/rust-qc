The review found **5 Major and 3 Minor findings**. The requested baseline build and tests pass, but six focused local regressions demonstrate behavior or error-handling problems. No production files were edited; regression harnesses are retained in `/work/tmp/probes`.

## Major

### 1. DATA padding consumes flow-control credit without returning it

Location: [src/proto/streams/recv.rs:580](/work/crate/src/proto/streams/recv.rs:580).

HTTP/2 flow control counts the entire DATA payload, including the padding-length octet and padding. This code counts only the application payload after padding has been stripped.

**Executed evidence:** with a 256-byte stream window, a legal DATA frame containing one padding-length octet and 255 padding octets produced an empty application buffer. h2 reported available capacity of 256 and used capacity of zero, and sent no WINDOW_UPDATE within the test’s 200 ms timeout. The peer has exhausted its window and cannot continue the body.

Fix: account for the original DATA payload length and automatically release discarded padding on both stream and connection windows. The existing `data_padding` test verifies body contents without checking credit accounting.

Regression: [padding.rs](/work/tmp/probes/tests/padding.rs).

### 2. Oversized trailers silently lose fields

Location: [src/proto/streams/recv.rs:361](/work/crate/src/proto/streams/recv.rs:361).

Header decoding marks oversized blocks and omits fields beyond the configured limit. Initial-header handling checks that marker; trailer handling ignores it and delivers the remaining fields successfully.

**Executed evidence:** with `max_header_list_size(64)`, trailers containing `checksum` with a 100-byte value returned `Ok(Some({}))`. A caller receives successful completion with missing metadata.

Fix: reject oversized trailers before delivering them, consistently with oversized initial headers.

Regression: [trailers.rs](/work/tmp/probes/tests/trailers.rs).

### 3. A response without `:status` becomes HTTP 200

Location: [src/client.rs:1676](/work/crate/src/client.rs:1676).

A response must contain the mandatory `:status` pseudo-header. When it is absent, this conversion leaves `http::Response::builder()` at its default status.

**Executed evidence:** a response header block containing only `content-length: 0` and END_STREAM returned a successful HTTP 200 response. Callers can therefore mistake a malformed response for success.

Fix: require `pseudo.status` and return a stream protocol error when absent.

Regression: [status.rs](/work/tmp/probes/tests/status.rs).

### 4. Large capacity reservations wrap to zero

Location: [src/share.rs:287](/work/crate/src/share.rs:287).

`reserve_capacity` accepts `usize` but casts directly to the `u32` window-size type. Its documentation specifies no corresponding argument limit.

**Executed evidence:** reservations of 1,024 and 4,294,967,295 bytes received capacity; 4,294,967,296 received zero, and 4,294,967,297 received one byte. Reserving a large remaining file size requires no large allocation, so an ordinary streaming caller can encounter this boundary and wait indefinitely for capacity.

Fix: retain the demand as `usize`, or clamp it before narrowing.

Regression: [capacity.rs](/work/tmp/probes/tests/capacity.rs).

### 5. Header lists exactly at the advertised limit are rejected

Locations: [src/frame/headers.rs:869](/work/crate/src/frame/headers.rs:869), [src/frame/headers.rs:911](/work/crate/src/frame/headers.rs:911).

Both comparisons use `< max_header_list_size`, rejecting equality despite the documented maximum.

**Executed evidence:** `:status: 200` occupies 42 decoded header-list bytes. It succeeded with limit 43 and failed with `PROTOCOL_ERROR` at limit 42. Adding `x: a` produced the same result at 77 versus its exact size of 76.

Consequently, a peer respecting the advertised maximum can have a valid response rejected.

Fix: accept equality in both comparisons and add boundary regressions.

Regression: [headers.rs](/work/tmp/probes/tests/headers.rs).

## Minor

### 6. Connection errors discard underlying I/O details

Locations: [src/proto/error.rs:92](/work/crate/src/proto/error.rs:92), [src/error.rs:130](/work/crate/src/error.rs:130).

The internal conversion retains the I/O kind and optionally a string, then reconstructs an `io::Error`.

**Executed evidence:** a mock transport returning OS error 5 yielded an h2 error whose `get_io().raw_os_error()` was `None`. Source inspection also shows typed inner errors becoming strings, while `Error::source()` is not implemented.

Callers lose diagnostic details and cannot reliably classify underlying failures.

Fix: preserve OS codes and typed causes through the cloneable internal representation, and expose the underlying cause through `source()`.

Regression: [io_error.rs](/work/tmp/probes/tests/io_error.rs).

### 7. Documentation omits panic contracts and fails the private-item check

Locations include [src/client.rs:1388](/work/crate/src/client.rs:1388), [src/server.rs:481](/work/crate/src/server.rs:481), [src/hpack/decoder.rs:508](/work/crate/src/hpack/decoder.rs:508), and [src/frame/headers.rs:959](/work/crate/src/frame/headers.rs:959).

The audit reported **24 missing `# Errors` sections and seven missing `# Panics` sections**, including four stable connection window setters that assert a 31-bit size bound without documenting it. This makes configuration APIs harder to use correctly.

The strict private-item rustdoc command also failed on two bare URLs. Public documentation built successfully.

Fix: document the applicable error and panic conditions, especially window bounds, and mark the two URLs as links.

### 8. Strict Clippy fails on the supplied toolchain

Locations include [src/hpack/huffman/table.rs:265](/work/crate/src/hpack/huffman/table.rs:265), [src/hpack/table.rs:309](/work/crate/src/hpack/table.rs:309), and [src/proto/streams/recv.rs:160](/work/crate/src/proto/streams/recv.rs:160).

The warnings-denied Clippy check stopped on eight diagnostics: one large constant array, two `Option` replacements, one manual slice fill, two question-mark suggestions, one large error variant, and one collapsible match.

These are pre-existing strict-check failures, not demonstrated runtime defects. Fix the diagnostics while preserving the declared MSRV. No historical toolchain was available to establish which diagnostics represent toolchain drift.

## Verification

Commands ran offline with `RUSTUP_AUTO_INSTALL=0`, using Rust/Cargo **1.99.1** on aarch64 Linux.

From `/work/crate`:

- `cargo build --locked` — **passed**.
- `cargo test --locked -p h2 --lib` — **419 passed, one ignored**.
- `cargo test --locked -p h2-tests` — **173 passed, 12 ignored**, including the stress test.
- `bash /work/skill/scripts/check.sh --audit /work/crate` — formatting and all-feature workspace tests **passed**; strict Clippy and private-item rustdoc **failed** as described above.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — **failed**, eight h2 diagnostics.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features` — **failed**, two bare URLs.
- `cargo test --workspace --all-features` — **passed**, including 39 passing doctests.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p h2 --no-deps --all-features` — **passed**.
- `cargo build --locked --no-default-features` — **passed**.
- `cargo check --locked -p h2 --features stream` and `cargo check --locked -p h2 --features unstable` — **passed**.
- `RUSTFLAGS="-D warnings" cargo test --locked -p h2-tests --no-run` — **failed before tests ran**, on four pre-existing deprecation diagnostics in `tests/h2-support/src/mock.rs`. This reproduces the repository CI’s warnings-as-errors configuration.
- `cargo +nightly-2026-10-01 miri test --locked -p h2 --lib hpack::decoder::test::test_decode_continuation_header_with_non_huff_encoded_name` — **one targeted test passed**.

The heuristic scan completed; style-only hits without a concrete consequence were not promoted to findings.

From `/work/tmp/probes`, `cargo test --locked --no-fail-fast -- --nocapture` ran all six regression targets; **all six failed as described above**. Harness formatting and warnings-denied Clippy checks passed.

**Finding count: 8 — five Major, three Minor.** Coverage was focused rather than exhaustive across protocol interleavings. Full Miri, Rust 1.63/MSRV, cross-platform, and h2spec checks were not run. No dedicated security testing was performed. Last `/work/clock` reading: **19.35205 of 45 minutes used**.