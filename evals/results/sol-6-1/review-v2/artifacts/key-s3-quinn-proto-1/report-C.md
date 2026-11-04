Found **16 issues: 14 reproduced behavior defects and two pre-existing strict-check failures**. The original baseline passes all 263 unit tests and 3 documentation tests.

Source links below refer to the unmodified release. Behavior findings were reproduced with local tests; root causes were established by source inspection.

1. **Disabling the server can panic on a pending connection’s retransmission** — [endpoint.rs:211](/work/crate/quinn-proto/src/endpoint.rs:211). After `set_server_config(None)`, packets for an existing `Incoming` should be handled or ignored safely. Replaying that client’s valid Initial instead panics at `server_config.as_ref().unwrap()`. An ordinary transition to client-only operation can therefore crash the endpoint driver.

2. **Connection flow control hides terminal stream errors** — [streams/mod.rs:254](/work/crate/quinn-proto/src/connection/streams/mod.rs:254). A stopped stream should report `WriteError::Stopped`, irrespective of connection credit. With a 100-byte connection window filled across two streams, stopping the one-byte stream produces `stopped() == Some(42)` but `write()` returns `Blocked`. Callers may wait for writability instead of receiving the terminal error.

3. **Rejected 0-RTT data permanently consumes the send window** — [streams/state.rs:244](/work/crate/quinn-proto/src/connection/streams/state.rs:244). Rejection discards outgoing streams and resets `data_sent`, but leaves `unacked_data` unchanged. A regression based on the existing rejection test fills a 13-byte send window; after successful handshaking and rejection, a new stream’s 13-byte write returns `Blocked`. The discarded data can never be acknowledged to release that budget.

4. **Valid ACK-frequency parameters can cause a panic** — [ack_frequency.rs:49](/work/crate/quinn-proto/src/connection/ack_frequency.rs:49). Calculating a requested delay should accommodate the peer’s valid minimum without panicking. Parameters with minimum delay 50 ms and maximum delay 100 ms successfully encode and decode, but calculating the request at RTT 10 ms panics because `clamp` receives minimum 50 ms and maximum 25 ms. This affects interoperability when ACK-frequency configuration is enabled.

5. **A zero idle timeout prevents servers from accepting connections** — [endpoint.rs:545](/work/crate/quinn-proto/src/endpoint.rs:545). Existing timeout-negotiation code and tests establish that zero disables the timeout. The acceptance staleness check instead treats it as immediate expiry. Configuring `Some(IdleTimeout::from(VarInt::from_u32(0)))` makes a normal connection attempt fail with `TimedOut`.

6. **Stopping an already reset stream fails to advertise replacement stream credit** — [streams/mod.rs:154](/work/crate/quinn-proto/src/connection/streams/mod.rs:154). Freeing a receive stream should replenish the peer’s concurrency allowance. With a one-stream limit, receiving an empty reset and then calling `stop()` frees local state but queues no `MAX_STREAMS` update; the peer still cannot open another stream. The corresponding finished-stream control test passes.

7. **Stopping an already reset stream releases receive credit twice** — [streams/recv.rs:95](/work/crate/quinn-proto/src/connection/streams/recv.rs:95). Reset processing already credits discarded unread data at [streams/state.rs:356](/work/crate/quinn-proto/src/connection/streams/state.rs:356). A subsequent `stop()` credits it again. With an eight-byte receive window, receiving six bytes, resetting, then stopping permits a new 14-byte write instead of eight bytes. The configured connection buffering bound is no longer maintained.

8. **An MTU reset can permanently stall queued datagrams** — [connection/mod.rs:1399](/work/crate/quinn-proto/src/connection/mod.rs:1399). `path_changed()` lowers the MTU without removing queued datagrams that no longer fit. Transmission readiness examines only the queue’s first datagram at [connection/mod.rs:3681](/work/crate/quinn-proto/src/connection/mod.rs:3681). In the regression, the payload limit falls from 1414 to 1162 bytes; a previously queued 1414-byte datagram prevents a subsequent five-byte datagram from being transmitted. Without the oversized queued datagram, the control transmits successfully.

9. **An illegal ordered read destroys the receive stream** — [streams/recv.rs:269](/work/crate/quinn-proto/src/connection/streams/recv.rs:269). Rejecting ordered reads after unordered reads should preserve the stream for legal unordered operations. `Chunks::new` removes the stream before checking ordering, then drops it on error. After reading three bytes of `abcdef`, an ordered attempt returns `IllegalOrderedRead`; retrying unordered returns `ClosedStream`, losing access to `def`.

10. **Failed connection creation leaks registered connection IDs** — [endpoint.rs:344](/work/crate/quinn-proto/src/endpoint.rs:344). A failed `connect()` should release provisional endpoint state. `new_cid` registers an ID before the fallible `start_session` call, with no rollback. Three attempts using an invalid hostname leave three registered CIDs despite zero open connections. Repeated application errors accumulate endpoint state and consume CID space.

11. **BBR bandwidth estimates never expire old peaks** — [bbr/bw_estimation.rs:65](/work/crate/quinn-proto/src/congestion/bbr/bw_estimation.rs:65). The ten-round maximum filter must receive lower samples so previous maxima can expire. Updates currently occur only when a sample exceeds the existing maximum. After 10,000 bytes/s samples followed by 25 rounds at 1,000 bytes/s, the executed component test still reports 10,000 bytes/s. Experimental BBR consequently retains stale capacity estimates after bandwidth decreases.

12. **Datagram sends exceed the configured buffer limit** — [datagrams.rs:39](/work/crate/quinn-proto/src/connection/datagrams.rs:39). With dropping enabled, old datagrams should be discarded until the incoming datagram fits. The loop checks the existing total without adding the incoming length. With a 100-byte limit, two queued 60-byte datagrams are both delivered instead of dropping the oldest, violating the documented limit and stale-data policy.

13. **Connection-ID validation unexpectedly panics on short IDs** — [cid_generator.rs:147](/work/crate/quinn-proto/src/cid_generator.rs:147). `validate` should return `Err(InvalidCid)` for an ID this generator could not produce. Calling it with the valid `ConnectionId` value representing an empty ID instead panics in `split_at(3)`. Direct callers cannot rely on the public validation method’s error result for this boundary case.

14. **`Datagrams::max_size()` ignores local disablement** — [datagrams.rs:69](/work/crate/quinn-proto/src/connection/datagrams.rs:69). Its documentation promises `None` when datagrams are disabled locally. With local datagrams disabled, the regression obtains `Some(1414)`, while `send()` returns `Disabled`. Callers using this query to determine availability receive contradictory answers.

15. **Strict documentation generation fails with default features** — [connection/qlog.rs:45](/work/crate/quinn-proto/src/connection/qlog.rs:45) and [crypto/rustls.rs:279](/work/crate/quinn-proto/src/crypto/rustls.rs:279). The executed rustdoc check reports unresolved links to feature-gated `QlogStream` and `ClientConfig::with_platform_verifier`. This prevents the strict documentation build used by CI. Enabling `qlog` and `platform-verifier` resolves these failures.

16. **Strict Clippy fails on Rust 1.99.1** — [assembler.rs:333](/work/crate/quinn-proto/src/connection/assembler.rs:333) and [packet_crypto.rs:91](/work/crate/quinn-proto/src/connection/packet_crypto.rs:91). The supplied source fails `-D warnings` with `derivable_impls` and `manual_filter`. These are lint compatibility failures, rather than demonstrated runtime defects, but block the strict lint recipe on the installed toolchain.

All commands used `RUSTUP_AUTO_INSTALL=0`. Checks ran from `/work/crate` on **Rust 1.99.1, aarch64 Linux**, with Clippy 0.1.99.

| Executed command | Outcome |
|---|---|
| `cargo test --locked -p quinn-proto` | Passed: 263 unit tests, 3 doctests |
| `cargo test --locked --offline -p quinn-proto --no-default-features` | Passed: 154 unit tests, 3 doctests |
| `cargo test --locked --offline -p quinn-proto --features qlog,platform-verifier,arbitrary,rustls-log` | Passed: 263 unit tests, 3 doctests |
| `cargo test --locked --offline -p quinn-proto --no-default-features --features rustls-aws-lc-rs` | Passed: 256 unit tests, 3 doctests |
| `cargo fmt -p quinn-proto -- --check` | Passed |
| `cargo clippy --locked --offline -p quinn-proto --all-targets -- -D warnings` | Failed: finding 16 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --offline -p quinn-proto --no-deps --document-private-items` | Failed: finding 15 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --offline -p quinn-proto --no-deps --document-private-items --features qlog,platform-verifier,rustls-aws-lc-rs,rustls-log` | Passed |
| `CARGO_BUILD_JOBS=2 cargo check --locked --offline -p quinn-proto --all-features` | Environment block: `aws-lc-fips-sys` requires unavailable Go |

Changes were confined to `/work/tmp/review`: tests were appended to `quinn-proto/src/tests/mod.rs`, `connection/ack_frequency.rs`, and `congestion/bbr/bw_estimation.rs`. Each original file remains a verbatim prefix of its scratch counterpart; no production fixes were made.

From that scratch workspace, `cargo test --locked --offline -p quinn-proto review_ -- --nocapture --test-threads=1` executed **15 tests: 14 failed as described above and one control passed**. The MTU case also contains a passing control. Detailed output is retained in [regressions.log](/work/tmp/regressions.log). ACK-frequency and BBR reproductions are component tests; the other cases exercise simulated connections or public helpers.

**Finding count: 16.** Coverage excludes exhaustive feature combinations, MSRV verification, other platforms, release-mode testing and Miri. FIPS compilation remains blocked by the missing build dependency. No dedicated security assessment was performed. Last `/work/clock` reading: **11.379533333333335 minutes used of 45.0**.