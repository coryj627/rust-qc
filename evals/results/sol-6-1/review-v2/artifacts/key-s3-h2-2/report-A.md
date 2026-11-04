Found **7 findings: 1 High, 4 Medium, and 2 Low**. The requested baseline checks passed, but five local regression tests exposed behavior the existing suite does not catch. No release source files were changed.

1. **High — Early Hints on a pushed response causes a library panic**

   [src/proto/streams/state.rs:162](/work/crate/src/proto/streams/state.rs:162), [src/proto/streams/recv.rs:171](/work/crate/src/proto/streams/recv.rs:171).

   Expected: a pushed response may send `103 Early Hints` before its final response.

   Observed: receiving 103 leaves the stream in `ReservedRemote`, although it has already been counted. The final response counts it again and panics at `src/proto/streams/counts.rs:111`: `assertion failed: !stream.is_counted`.

   Consequence: an ordinary client connection task panics while processing a valid response sequence.

   Evidence: executed `pushed_get_accepts_early_hints_before_final_response`, using a promised GET followed by 103 and 200. Source inspection confirms the assertion is unconditional.

   transition out of the reserved state on the first response HEADERS while continuing to await final headers; count the stream once.

2. **Medium — Discarded DATA padding is omitted from flow control**

   [src/proto/streams/recv.rs:580](/work/crate/src/proto/streams/recv.rs:580).

   Expected: DATA flow control accounts for the entire frame payload, including padding and its length byte. Discarded padding capacity must become reusable.

   Observed: accounting uses only the body length after padding has been stripped.

   Consequence: a conforming peer can exhaust its window and stall, even after the application consumes and releases all body data.

   Evidence: executed `padded_data_returns_discarded_padding_capacity`. A 16-byte stream window was filled with one body byte, one padding-length byte, and fourteen padding bytes. After releasing the body byte, no WINDOW_UPDATE arrived within 100 ms. The equivalent unpadded control passed. Source inspection confirms padding never reaches the receive accounting.

   account for all flow-controlled octets and automatically release discarded padding at both stream and connection levels.

3. **Medium — A valid 304 response fails when ended with empty DATA**

   [src/proto/streams/recv.rs:187](/work/crate/src/proto/streams/recv.rs:187), [src/proto/streams/recv.rs:641](/work/crate/src/proto/streams/recv.rs:641).

   Expected: a 304 response may carry a representation `Content-Length` without sending that content, including when an empty DATA frame ends the stream.

   Observed: the length is retained as outstanding body data; the empty final DATA frame produces `PROTOCOL_ERROR`.

   Consequence: an ordinary conditional GET can fail depending on how the server terminates its response.

   Evidence: executed `not_modified_accepts_empty_end_data` with status 304, `Content-Length: 100`, and empty final DATA. It failed. Ending the same response in HEADERS passed.

   represent 304’s no-content semantics throughout body validation, rather than exempting only END_STREAM HEADERS.

4. **Medium — A 4 GiB capacity request silently becomes zero**

   [src/share.rs:287](/work/crate/src/share.rs:287).

   Expected: `reserve_capacity(usize)` preserves a positive request or applies a documented limit.

   Observed: casting to `WindowSize` truncates the argument. On this 64-bit host, `reserve_capacity(1usize << 32)` becomes `reserve_capacity(0)`.

   Consequence: an uploader reserving its remaining file size can cancel its reservation and wait indefinitely for capacity.

   Evidence: executed `reserve_large_body_does_not_cancel_reservation`. An existing positive reservation became zero. Controls requesting 65,535 bytes, 2 GiB, and 4 GiB minus one retained available capacity.

   clamp before conversion or use a checked conversion with a documented policy for large requests.

5. **Medium — Pushed HEAD responses reject valid representation lengths**

   [src/proto/streams/recv.rs:749](/work/crate/src/proto/streams/recv.rs:749).

   Expected: pushed HEAD responses receive the same no-body treatment as directly requested HEAD responses. The crate explicitly permits HEAD promises.

   Observed: promise processing never sets `ContentLength::Head`. A 200 response with `Content-Length: 100` and END_STREAM is rejected with `PROTOCOL_ERROR`.

   Consequence: clients cannot receive otherwise valid pushed HEAD metadata.

   Evidence: executed `pushed_head_accepts_representation_content_length`. Controls using a directly requested HEAD, or a pushed HEAD with length zero, passed.

   initialize the promised stream’s content-length state from its request method.

6. **Low — Strict Clippy fails on the supplied toolchain**

   Locations include [src/hpack/huffman/table.rs:265](/work/crate/src/hpack/huffman/table.rs:265), [src/hpack/table.rs:309](/work/crate/src/hpack/table.rs:309), and [src/proto/streams/recv.rs:160](/work/crate/src/proto/streams/recv.rs:160).

   Expected: the strict lint gate completes successfully.

   Observed: Rust 1.99.1 reports eight errors: a large const array, two Option replacements, manual slice filling, two simplifiable matches, a large error variant, and a collapsible match.

   Consequence: maintainers cannot use the requested strict gate successfully. These are pre-existing lint failures, not evidence of a runtime defect. The repository’s Clippy CI job is commented out; older-toolchain behavior was not tested.

   address the diagnostics or document justified exceptions consistent with the MSRV.

7. **Low — Documentation checks fail, and panic preconditions are incomplete**

   Locations: [src/hpack/decoder.rs:508](/work/crate/src/hpack/decoder.rs:508), [src/frame/headers.rs:959](/work/crate/src/frame/headers.rs:959), and [src/client.rs:1388](/work/crate/src/client.rs:1388).

   Observed: strict documentation including private items fails on two bare URLs. The advisory pass also reports 24 missing `# Errors` sections and seven missing `# Panics` sections across the all-feature API.

   A concrete caller-facing omission is that client and server window setters panic above `2^31 − 1`, without documenting that bound or panic. This includes setters returning `Result`.

   Consequence: strict internal documentation builds fail, and callers lack complete configuration preconditions. Default public documentation passed.

   format the URLs and document actual error conditions, bounds, and panics.

Verification used offline dependencies and `RUSTUP_AUTO_INSTALL=0`, on aarch64 Linux with Rust 1.99.1 unless specified:

- `cargo build --locked` — passed.
- `cargo test --locked -p h2 --lib` — 419 passed, one ignored.
- `cargo test --locked -p h2-tests` — 173 passed, twelve ignored; stress test completed in about 199 seconds.
- `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` — exited 1:
  - `cargo fmt --all --check` — passed.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — failed with the eight diagnostics above.
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features` — failed on two bare URLs.
  - `cargo test --workspace --all-features` — 631 passed, fourteen ignored, including doctests.
- `cargo build --locked --no-default-features` — passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p h2 --no-deps` — passed.
- `python3 -I /work/[tooling]/scripts/[tooling] /work/crate` — completed; heuristic hits were inspected rather than treated as automatic defects.
- `cargo +nightly-2026-10-01 miri test --locked -p h2 --lib hpack::encoder::test::test_encode_method` — three passed.
- The selected HPACK fixture initially encountered Miri’s filesystem-isolation restriction. Re-running with `MIRIFLAGS=-Zmiri-disable-isolation` passed.

Regression tests remain in [the scratch harness](/work/tmp/review-harness/tests/contracts.rs). From `/work/tmp/review-harness`, `cargo test --locked --test contracts` produced five expected failures and five passing controls. Harness formatting and strict Clippy passed. Its existing `h2-support` dependency enables `stream` and `unstable`.

**Final count: 7 findings.** Coverage combined source inspection with ordinary local regression tests; no dedicated security assessment was performed. Rust 1.63 compatibility, other platforms, full-suite Miri, and h2spec were not run. Last clock reading: **1,070.3 of 2,700 seconds — 17.835 minutes**.