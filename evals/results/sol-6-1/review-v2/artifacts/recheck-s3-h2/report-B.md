Found **8 issues: 6 behavioral defects, 1 documentation defect, and 1 lint failure**. The release source was left unchanged.

1. **Queued uploads can deadlock active requests — [src/proto/streams/prioritize.rs:287](/work/crate/src/proto/streams/prioritize.rs:287).**  
   Capacity reservation should allow active streams to progress while another stream waits for the concurrency limit. Instead, `reserve_capacity` assigns connection capacity to a stream in `pending_open`. With `MAX_CONCURRENT_STREAMS=1`, a queued upload that reserves and buffers 65,535 bytes consumes the entire connection window. The active request cannot finish, so the queued request cannot open.  
   **Executed evidence:** `queued_reservation_must_not_block_active_request` timed out after one second. The otherwise identical upload without explicit reservation passed. Both calls to `send_request` followed successful readiness checks.

2. **Padded DATA can stall a conforming peer — [src/proto/streams/recv.rs:580](/work/crate/src/proto/streams/recv.rs:580).**  
   HTTP/2 flow control counts the complete DATA payload, including padding. This code counts only the application payload after padding has been stripped. With a five-byte receive window, a valid frame containing one data byte, the padding-length byte, and three padding bytes exhausts the peer’s window. Releasing the application byte produces no window update, leaving both sides waiting.  
   **Executed evidence:** `padded_receive_window_replenishes` timed out; its unpadded control passed. **Source inspection:** the existing `data_padding` test checks payload stripping on an ended stream, so it cannot detect this replenishment failure.

3. **Zero-byte writes can cause an endless polling loop — [src/codec/framed_write.rs:137](/work/crate/src/codec/framed_write.rs:137).**  
   An `AsyncWrite` returning `Ok(0)` for nonempty input should terminate the write with `WriteZero`. Here, the returned byte count is discarded and the unchanged buffer is retried immediately. A permanently exhausted transport therefore prevents the poll from returning and can block its executor thread.  
   **Executed evidence:** a bounded writer returned `Ok(0)` once, then a distinct error on any retry. Server handshake called it twice and returned the second error instead of `WriteZero`. The endless-loop consequence follows from source inspection; an unbounded spinning test was not run.

4. **A 4 GiB capacity reservation silently becomes zero — [src/share.rs:287](/work/crate/src/share.rs:287).**  
   `reserve_capacity` accepts `usize` without documenting a size restriction, but casts it directly to `u32`. On this 64-bit platform, `reserve_capacity(1_usize << 32)` becomes a cancellation of the reservation. Callers reserving from a large file’s remaining length can consequently wait indefinitely for capacity.  
   **Executed evidence:** `reservation_larger_than_u32_must_not_become_zero` observed capacity fall from 65,535 to zero without sending data. Large requests should be bounded internally or explicitly rejected, rather than silently wrapping.

5. **Valid 304 responses fail when terminated by empty DATA — [src/proto/streams/recv.rs:187](/work/crate/src/proto/streams/recv.rs:187).**  
   A 304 response may advertise the selected representation’s nonzero Content-Length while carrying no content. The implementation exempts a 304 ending in HEADERS from length matching, but retains `ContentLength::Remaining` when END_STREAM arrives later. The empty DATA frame then fails the length check at [recv.rs:641](/work/crate/src/proto/streams/recv.rs:641).  
   **Executed evidence:** a conditional GET receiving `304`, `content-length: 123`, and empty DATA with END_STREAM yielded a body error containing `PROTOCOL_ERROR`. The same response ending in HEADERS passed.

6. **Pushed HEAD responses lose HEAD semantics — [src/proto/streams/recv.rs:749](/work/crate/src/proto/streams/recv.rs:749).**  
   HEAD responses must permit nonzero Content-Length without requiring corresponding body bytes. Ordinary HEAD requests set `ContentLength::Head`; received HEAD push promises never set that state. Consequently, a valid pushed HEAD response is rejected as truncated.  
   **Executed evidence:** `pushed_head_with_content_length` received a library-generated `PROTOCOL_ERROR` for a headers-only response with `content-length: 123`. The ordinary HEAD control passed.

7. **The server guide names a removed connection-driving method — [src/server.rs:40](/work/crate/src/server.rs:40).**  
   The guide repeatedly instructs callers to use `Connection::poll_close`, while the public method is `poll_closed`. Following this guidance when driving a connection without accepting requests fails to compile. The same guide also incorrectly says graceful shutdown is unimplemented at [server.rs:57](/work/crate/src/server.rs:57), despite the public `graceful_shutdown` method.  
   **Executed evidence:** a scratch example using `poll_close` failed with E0599; changing it to `poll_closed` compiled successfully.

8. **Clippy cannot complete the library test target on the supplied toolchain — [src/lib.rs:88](/work/crate/src/lib.rs:88).**  
   Under Clippy 0.1.99, `cfg_attr(test, deny(warnings))` turns eight existing Clippy warnings into errors. These include the large constant array at `src/hpack/huffman/table.rs:265` and `mem_replace_option_with_some` at `src/hpack/table.rs:309`. This blocks the full lint check even though compilation and tests pass.  
   **Executed evidence:** `cargo clippy --locked -p h2 --all-targets --all-features` exited 101 on the unmodified release. This is a toolchain-specific lint failure, not evidence of a runtime defect.

All commands used `RUSTUP_AUTO_INSTALL=0`, Rust **1.99.1**, and prefetched locked dependencies on **aarch64 Linux**. Checks ran from `/work/crate` unless noted.

| Command | Outcome |
|---|---|
| `cargo build --locked` | Passed |
| `cargo test --locked -p h2 --lib` | 419 passed, 1 ignored |
| `cargo test --locked -p h2-tests` | 173 passed, 12 ignored; stress test passed in 204.18 seconds |
| `cargo test --locked -p h2 --lib --all-features` | 419 passed, 1 ignored |
| `cargo test --locked -p h2 --doc --all-features` | 39 passed, 1 ignored |
| `cargo doc --locked -p h2 --no-deps --features stream` | Passed |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --locked -p h2 --all-targets --all-features` | Failed as described above |
| `RUSTFLAGS=-Dwarnings cargo test --locked -p h2-tests --no-run` | Pre-existing failure: four deprecated `std::usize` usages in `tests/h2-support/src/mock.rs` |

The scratch copy is `/work/tmp/review`. Its only additions are [review_regressions.rs](/work/tmp/review/tests/h2-tests/tests/review_regressions.rs) and two documentation-check examples. No implementation or existing test was edited.

From that copy, `cargo test --locked -p h2-tests --test review_regressions -- --nocapture --test-threads=1` produced **6 expected regression failures and 4 passing controls**. `cargo check --locked -p h2 --example review_doc_api` failed with E0599; the corrected `review_doc_api_current` example passed.

**Total: 8 findings.** Coverage was source inspection plus existing and targeted local tests, without exhaustive path coverage. MSRV 1.63 was unavailable and untested. Release-profile tests, h2spec, Miri, fuzzing, and dedicated security testing were not run. Last clock reading: **16.001866666666665 minutes used of 45.0**.