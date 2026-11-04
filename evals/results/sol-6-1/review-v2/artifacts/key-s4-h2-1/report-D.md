Found **4 correctness issues**, reproduced with ordinary local tests. No supplied source files were edited.

1. **Rejected push requests retain stream entries** — [src/proto/streams/streams.rs:1209](/work/crate/src/proto/streams/streams.rs:1209).  
   Expected: validation errors clean up the newly reserved stream. Actual: `convert_push_message(...)?` returns before the cleanup block. Two rejected POST pushes left three stored streams instead of the original one. Caller errors therefore retain unnecessary per-connection state.

2. **Large capacity reservations silently wrap** — [src/share.rs:287](/work/crate/src/share.rs:287).  
   Expected: a large positive reservation remains positive, through deliberate limiting or documented validation. Actual: casting `usize` to `u32` makes `reserve_capacity(1usize << 32)` reserve zero on this 64-bit platform. The test observed capacity fall from 1 to 0, potentially stalling a sender despite available connection capacity.

3. **Header lists exactly at the configured limit are rejected** — [src/frame/headers.rs:869](/work/crate/src/frame/headers.rs:869), also line 911.  
   Expected: the configured maximum includes equality. Actual: both comparisons use `<`. A response containing only `:status: 200` occupies exactly 42 decoded bytes; limit 43 succeeded, while limit 42 produced a library `PROTOCOL_ERROR` reset. Valid boundary-sized messages consequently fail.

4. **Outgoing trailers bypass header validation** — [src/proto/streams/send.rs:277](/work/crate/src/proto/streams/send.rs:277).  
   Expected: prohibited connection headers receive the same validation as initial headers, before closing the send side. Actual: trailers containing `connection: close` return `Ok` and close the stream. A second test between two h2 endpoints confirmed that the receiver then reports `PROTOCOL_ERROR`. Callers receive delayed failure and cannot correct the trailers on that stream.

Executed checks from `/work/crate`, with `RUSTUP_AUTO_INSTALL=0`:

| Command | Outcome |
|---|---|
| `cargo build --locked` | Passed |
| `cargo test --locked -p h2 --lib` | 419 passed, 1 ignored |
| `cargo test --locked -p h2-tests` | 173 passed, 12 ignored; stress test passed |
| `cargo clippy --locked -p h2 --all-features --lib` | Completed; 8 warnings |
| `cargo test --locked -p h2 --doc --all-features` | 39 passed, 1 ignored |
| `cargo doc --locked --no-deps -p h2 --all-features` | Passed |
| `cargo fmt --all -- --check` | Passed |

The scratch tests are preserved in [correctness.rs](/work/tmp/h2-review/tests/correctness.rs). Reproduce with:

```sh
cd /work/tmp/h2-review
RUSTUP_AUTO_INSTALL=0 cargo test --offline --locked --test correctness -- --nocapture
```

Result: **4 failing regression assertions and 1 passing corroborating test**, exit 101. [Recorded output](/work/tmp/h2-review-results.log).

Source inspection focused on public API documentation, header conversion, framing, flow control, stream state, and error cleanup. This was not exhaustive state-space coverage. Ignored tests, Miri, MSRV, and other architectures were unrun; no required check remained blocked. Dedicated security testing and inquiries requiring it were outside scope and were not pursued.

Last clock reading: **8.344583333333334 minutes used of 15.0**.