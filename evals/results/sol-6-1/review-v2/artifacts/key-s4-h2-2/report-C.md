Found **7 correctness issues**. Supplied source was unchanged; all added tests are under `/work/tmp/h2-review`.

1. **P1 — Padded DATA can stall valid streams.** [src/proto/streams/recv.rs:580](/work/crate/src/proto/streams/recv.rs:580) counts only application bytes after padding is stripped. Flow control must account for the entire DATA payload, including padding, and reclaim discarded padding. With an eight-byte window, one body byte plus seven padding-related bytes exhausts the peer’s window, but releasing the body byte produces no `WINDOW_UPDATE`. Ordinary padded transfers can stop progressing. [Reproduction](/work/tmp/h2-review/tests/padding.rs:37) timed out waiting for an update; the equivalent unpadded control passed.

2. **P1 — Flushing retries zero-byte writes without making progress.** [src/codec/framed_write.rs:142](/work/crate/src/codec/framed_write.rs:142) discards the returned write count. A zero-byte write with pending data should return `WriteZero`; instead, flushing immediately retries. A transport that repeatedly returns zero can keep the connection future from yielding. The [bounded transport test](/work/tmp/h2-review/tests/transport.rs:21) returned zero once, then a sentinel error: h2 made the extra write and returned that error instead of `WriteZero`. The indefinite-loop consequence follows from source inspection; no unbounded test was run.

3. **P2 — Responses missing `:status` become successful responses.** [src/client.rs:1676](/work/crate/src/client.rs:1676) leaves `Response::builder()`’s default status when `:status` is absent. Expected behavior is a stream protocol error; actual behavior is `200 OK`. Applications can therefore treat an invalid response as successful. The [test](/work/tmp/h2-review/tests/contracts.rs:54) received a header block containing only `content-length: 0` and observed a successful response with status 200.

4. **P2 — Header lists exactly at the configured maximum are rejected.** [src/frame/headers.rs:869](/work/crate/src/frame/headers.rs:869) and [line 911](/work/crate/src/frame/headers.rs:911) use `<` instead of `<=`. A sole `:status: 200` field occupies exactly 42 decoded octets. With `max_header_list_size(42)`, the [test](/work/tmp/h2-review/tests/contracts.rs:35) received `PROTOCOL_ERROR`; the same response passed with limit 43. Valid responses at the advertised boundary fail.

5. **P2 — Large capacity reservations silently truncate.** [src/share.rs:287](/work/crate/src/share.rs:287) casts `usize` to `u32`. On this 64-bit platform, `reserve_capacity(1usize << 32)` becomes a zero-capacity cancellation. Expected behavior is meaningful reservation handling or an explicit documented limit. Callers reserving from a large remaining body length can wait for capacity that was never requested. The [test](/work/tmp/h2-review/tests/contracts.rs:4) observed capacity zero; a 65,535-byte control immediately received capacity.

6. **P2 — Trailers bypass outbound header validation.** [src/proto/streams/send.rs:276](/work/crate/src/proto/streams/send.rs:276) closes and queues trailers without calling `check_headers`. Expected behavior is rejection of connection-specific fields, consistent with initial headers. Actual behavior: `send_trailers` accepts `connection: close`. Applications receive success locally and encounter a later protocol failure. The [contract test](/work/tmp/h2-review/tests/contracts.rs:13) observed `Ok(())`; an [additional test](/work/tmp/h2-review/tests/outbound.rs:5) confirmed the emitted trailer fails decoding with `PROTOCOL_ERROR`.

7. **P2 — Non-CONNECT authority-form requests are accepted without a scheme.** [src/client.rs:1636](/work/crate/src/client.rs:1636) contains an empty error branch. An HTTP/2 GET targeting `example.com:443` should fail local validation because it lacks `:scheme`; `send_request` instead succeeds. Callers discover the problem asynchronously through peer rejection. The [contract test](/work/tmp/h2-review/tests/contracts.rs:24) demonstrated acceptance, and the [wire observation](/work/tmp/h2-review/tests/outbound.rs:26) confirmed emission without `:scheme`.

Executed from `/work/crate`, with `RUSTUP_AUTO_INSTALL=0`:

| Command | Outcome |
|---|---|
| `cargo build --locked` | Passed |
| `cargo test --locked -p h2 --lib` | 419 passed, 1 ignored |
| `cargo test --locked -p h2-tests` | 273 passed, 12 ignored; stress test passed |
| `cargo test --locked -p h2 --doc --features stream` | 39 passed, 1 ignored |
| `cargo clippy --locked -p h2 --lib --all-features` | Completed with 8 warnings |
| `cargo fmt --all -- --check` | Passed |

Clippy’s warnings concern maintainability and representation choices, including a large constant array, large error variant, and simplification suggestions. They were not counted as correctness findings. The passing suites lack regression coverage for the cases above.

Reproduce all focused tests:

```sh
cd /work/tmp/h2-review
RUSTUP_AUTO_INSTALL=0 cargo test --locked --offline --no-fail-fast --tests -- --nocapture
```

Outcome: **7 failing contract assertions and 7 passing controls/observations**, exit 101. Full output: [results.txt](/work/tmp/h2-review/results.txt).

This was a time-bounded source review with ordinary local tests, not exhaustive verification. No required check was blocked. MSRV, cross-platform, and Miri checks were unrun. Dedicated security testing and inquiries requiring it were outside scope; none were pursued.

Last `/work/clock` reading: **9.518833333333333 of 15.0 minutes used**.