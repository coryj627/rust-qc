Found **15 behavior defects and 2 validation failures**. The original test suite passes; 16 added local regression tests reproduce the behavior defects. `/work/crate` was unchanged. All edits append tests in `/work/tmp/review`; no production fixes were made.

1. **Disabling server configuration can panic with pending connections.** [endpoint.rs:211](/work/crate/quinn-proto/src/endpoint.rs:211), [endpoint.rs:540](/work/crate/quinn-proto/src/endpoint.rs:540)  
   `set_server_config` documents that changes affect new incoming connections only. After receiving an `Incoming`, calling `set_server_config(None)` makes both handling a duplicate ordinary Initial and accepting the pending connection panic on `unwrap()`. This can terminate an application while it stops accepting new connections. Executed evidence: `review_disable_server_with_pending_initial` and `review_accept_pending_after_disabling_server`.

2. **Rejected 0-RTT data permanently consumes send-window capacity.** [connection/streams/state.rs:223](/work/crate/quinn-proto/src/connection/streams/state.rs:223)  
   Rejection should discard early streams and restore their local buffer allowance. The reset clears `data_sent` but leaves `unacked_data`. With a 32-byte send window filled before rejection, writing five bytes on the replacement stream returns `Blocked` after the handshake succeeds. No acknowledgment can release the discarded data’s accounting. Executed evidence: `review_zero_rtt_rejection_restores_send_window`.

3. **Stopping a reset receive stream can stall subsequent streams.** [connection/streams/mod.rs:154](/work/crate/quinn-proto/src/connection/streams/mod.rs:154)  
   Freeing the receive stream should make renewed concurrency credit available to the peer. `stop()` frees its state but does not queue `MAX_STREAMS`. With a one-stream limit, receiving a reset, accepting that stream and stopping it leaves the sender unable to open another stream after both endpoints are driven. Executed evidence: `review_stop_reset_stream_releases_stream_credit`.

4. **`IllegalOrderedRead` destroys the receive stream and unread data.** [connection/streams/recv.rs:266](/work/crate/quinn-proto/src/connection/streams/recv.rs:266)  
   The documented ordering error should reject the ordered read while leaving unordered access usable. Instead, `Chunks::new` removes the stream before checking ordering and drops it when validation fails. After reading `h` unordered from `hello`, attempting an ordered read returns `IllegalOrderedRead`; the next unordered read returns `ClosedStream`, losing `ello`. Executed evidence: `review_illegal_ordered_read_preserves_stream`.

5. **A zero idle timeout rejects every server connection immediately.** [endpoint.rs:542](/work/crate/quinn-proto/src/endpoint.rs:542)  
   Zero means disabled in the transport-parameter documentation and existing timeout-negotiation tests. The stale-Initial check instead interprets it as an immediate deadline. Configuring `max_idle_timeout(Some(VarInt::from_u32(0).into()))` makes an ordinary handshake fail with `TimedOut`, even when accepted immediately. Executed evidence: `review_zero_idle_timeout_is_disabled`.

6. **Failed connection setup leaks a reserved connection ID.** [endpoint.rs:344](/work/crate/quinn-proto/src/endpoint.rs:344)  
   A failed `connect()` should release provisional endpoint state. `new_cid` inserts into the routing index before fallible `start_session`, without rollback. Connecting with server name `"bad name"` returns `InvalidServerName` and leaves one tracked CID despite zero open connections. Repeated failures accumulate state; source inspection also shows these entries count toward short-CID exhaustion. Executed evidence: `review_failed_connect_releases_connection_id`.

7. **STOP followed by RESET credits received bytes twice.** [connection/streams/state.rs:350](/work/crate/quinn-proto/src/connection/streams/state.rs:350)  
   Connection flow-control credit should be restored once per consumed or discarded byte. Receiving 32 bytes and stopping the stream raises a 100-byte allowance to 132. Receiving its matching reset raises it again to 164, although total received data remains 32. Ordinary stream cancellation therefore advertises excess allowance beyond the configured window. Executed unit evidence: `review_stopped_reset_credits_once`.

8. **Shrinking and restoring the receive window creates extra credit.** [connection/streams/state.rs:870](/work/crate/quinn-proto/src/connection/streams/state.rs:870)  
   Restoring the original window before any reads should cancel outstanding shrink debt. Instead, expansion adds new credit while retaining the debt. Changing a 100-byte window to 50 and back to 100 produces `local_max_data == 150`. Callers adjusting buffering limits can temporarily permit more outstanding data than requested. Executed unit evidence: `review_receive_window_shrink_expand_does_not_add_credit`.

9. **Datagram eviction ignores the new datagram’s length.** [connection/datagrams.rs:39](/work/crate/quinn-proto/src/connection/datagrams.rs:39)  
   The configured send-buffer maximum should trigger eviction before appending data that exceeds it. The loop checks only the already-buffered total. With a ten-byte limit, two six-byte datagrams remain queued, and the receiver gets the older datagram that should have been evicted. This exceeds the documented buffer limit and sends stale data. Executed evidence: `review_datagram_drop_respects_buffer_limit`.

10. **BBR’s bandwidth estimate never expires an earlier maximum.** [congestion/bbr/bw_estimation.rs:65](/work/crate/quinn-proto/src/congestion/bbr/bw_estimation.rs:65)  
    The underlying maximum filter has a ten-round window, but it receives updates only when bandwidth increases. An estimate of 1,000,000 bytes/second remains unchanged after twenty non-application-limited rounds delivering 100,000 bytes/second. Source inspection shows BBR uses this stale estimate for congestion-window and pacing calculations, impairing adaptation after capacity decreases. Executed unit evidence: `review_bandwidth_estimate_expires_old_maximum`.

11. **`minimum_change(0)` prevents MTU discovery from completing.** [connection/mtud.rs:333](/work/crate/quinn-proto/src/connection/mtud.rs:333)  
    The public setter accepts zero without documenting a restriction. With zero, the stopping comparison can never succeed. A bounded test acknowledged 32 successive probes; discovery settled into repeatedly probing 1451 bytes instead of finishing. Source inspection establishes that this continues, producing ongoing probe traffic. Executed unit evidence: `review_zero_minimum_change_completes`.

12. **CID validation panics on an empty, constructible CID.** [cid_generator.rs:147](/work/crate/quinn-proto/src/cid_generator.rs:147)  
    `validate()` returns `Result<(), InvalidCid>` and documents no minimum input length. Passing `ConnectionId::new(&[])` to `HashedConnectionIdGenerator::validate` panics at `split_at(3)` instead of returning `InvalidCid`. Callers cannot use the fallible API directly for all valid `ConnectionId` values. Executed evidence: `review_validate_short_connection_id`.

13. **Qlog RTT metrics use seconds instead of milliseconds.** [connection/paths.rs:267](/work/crate/quinn-proto/src/connection/paths.rs:267)  
    Qlog recovery durations require milliseconds. Conversion emits `0.025` for a 25 ms minimum RTT, making reported RTT 1,000 times too small. The same seconds conversion appears in all four RTT fields. This gives callers incorrect diagnostic measurements. Executed converter evidence: `review_qlog_rtt_uses_milliseconds`, with feature `qlog`.

14. **MTU reduction drops datagrams exactly at the allowed limit.** [connection/datagrams.rs:149](/work/crate/quinn-proto/src/connection/datagrams.rs:149)  
    `drop_oversized` promises to discard payloads larger than its limit, but retains only strictly smaller payloads. A five-byte queued datagram is removed when the limit is five. Source inspection shows this runs after detected MTU reductions, causing avoidable loss of valid queued datagrams. Executed unit evidence: `review_drop_oversized_keeps_limit_sized_datagram`.

15. **`max_size()` reports capacity when datagrams are disabled locally.** [connection/datagrams.rs:69](/work/crate/quinn-proto/src/connection/datagrams.rs:69)  
    Its contract explicitly requires `None` when locally disabled. With local datagram support disabled and a supporting peer, it returns `Some(1414)`, while `send()` returns `Disabled`. Applications using this method to discover datagram availability receive contradictory results. Executed evidence: `review_datagram_max_size_when_disabled`.

16. **Strict documentation builds fail under default features.** [crypto/rustls.rs:279](/work/crate/quinn-proto/src/crypto/rustls.rs:279), [connection/qlog.rs:45](/work/crate/quinn-proto/src/connection/qlog.rs:45)  
    Public documentation links to feature-gated `with_platform_verifier` while that feature is disabled. Including private items additionally exposes an unresolved `QlogStream` link. Both fail with `RUSTDOCFLAGS=-Dwarnings`; enabling `qlog,platform-verifier` makes the private-item build pass. This prevents warning-clean default documentation generation. These failures were executed on the unmodified release.

17. **Strict Clippy fails on the supplied toolchain.** [connection/assembler.rs:333](/work/crate/quinn-proto/src/connection/assembler.rs:333), [connection/packet_crypto.rs:91](/work/crate/quinn-proto/src/connection/packet_crypto.rs:91)  
    Clippy reports `derivable_impls` and `manual_filter`. With the project’s `-D warnings` convention, these prevent lint validation on Rust 1.99.1 / Clippy 0.1.99. This is a toolchain-dependent quality failure, not evidence of incorrect runtime behavior. Executed on the unmodified release.

All commands below ran with `RUSTUP_AUTO_INSTALL=0`, from `/work/crate` unless stated otherwise.

| Command | Outcome |
|---|---|
| `cargo test --locked -p quinn-proto` | Passed: 263 unit tests, 3 doctests |
| `cargo test --locked -p quinn-proto --no-default-features` | Passed: 154 unit tests, 3 doctests |
| `cargo test --locked -p quinn-proto --features qlog,platform-verifier,arbitrary` | Passed: 263 unit tests, 3 doctests |
| `CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2 cargo test --locked -p quinn-proto --no-default-features --features rustls-aws-lc-rs` | Passed: 256 unit tests, 3 doctests |
| `cargo fmt -p quinn-proto -- --check` | Passed |
| `cargo clippy --locked -p quinn-proto --all-targets -- -D warnings` | Failed: finding 17 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p quinn-proto --no-deps` | Failed: platform-verifier link |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p quinn-proto --no-deps --document-private-items` | Failed: both links in finding 16 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p quinn-proto --no-deps --document-private-items --features qlog,platform-verifier` | Passed |
| `CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2 cargo check --locked -p quinn-proto --all-features` | Environment-blocked: `aws-lc-fips-sys` requires unavailable Go |
| In `/work/tmp/review`: `cargo test --locked -p quinn-proto --features qlog review_ -- --test-threads=1` | 16 regression tests ran and failed as described above |

The final regression output is retained in [review-final.log](/work/tmp/review-final.log). An initial scratch-test compilation error caused by an assertion requiring `Chunks: Debug` was corrected before executing the regressions; it is not a crate finding.

**Finding count: 17 — 15 behavior defects and 2 validation failures.** Coverage was targeted to endpoint lifecycle, streams, datagrams, MTU discovery, congestion estimation, qlog and package checks on aarch64 Linux. MSRV, other platforms, the feature powerset, Miri and FIPS tests were not run. No dedicated security assessment or fuzzing was performed. Last clock reading: **10.952866666666667 minutes used of 45.0**.