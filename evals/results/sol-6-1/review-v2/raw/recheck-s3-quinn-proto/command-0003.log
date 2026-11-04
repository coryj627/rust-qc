The existing test suites pass, but targeted regression tests confirmed runtime panics, lost receive state, stalled streams, and incorrect flow-control accounting. I found **4 Blocker, 7 Major, and 5 Minor findings**, using the skill’s severity definitions. No production files were changed.

## Blocker

### 1. Disabling incoming connections panics on pending connections

Location: [endpoint.rs:211](/work/crate/quinn-proto/src/endpoint.rs:211), also line 540.

`set_server_config` is documented as affecting new incoming connections only. After receiving an `Incoming`, calling `set_server_config(None)` makes both a subsequent packet for that pending connection and `accept(incoming, …, None)` unwrap the absent configuration.

An application temporarily disabling new connections can therefore panic while processing an already pending connection.

Evidence: executed `review_disable_server_handles_pending_retransmission` and `review_disable_server_preserves_pending_accept`; both panicked at the locations above.

Fix: retain the configuration associated with pending incoming connections, or handle its absence through an explicit error path.

### 2. A rejected ordered read destroys buffered stream data

Location: [connection/streams/recv.rs:269](/work/crate/quinn-proto/src/connection/streams/recv.rs:269).

`Chunks::new` removes the receive state before checking whether ordered reading is allowed. When `ensure_ordering` returns `IllegalOrderedRead`, the removed state is dropped instead of restored.

Expected: reject the ordered read while allowing subsequent unordered reads. Observed: subsequent reads return `ClosedStream`, losing remaining buffered data.

Evidence: executed `review_illegal_ordered_read_preserves_stream`: send `hello`, read `h` unordered, attempt an ordered read, then resume unordered reading. The final read returned `ClosedStream` instead of `ello`.

Fix: validate ordering before removing the entry, or restore it on error.

### 3. Truncated optional transport parameters panic instead of returning errors

Location: [transport_parameters.rs:472](/work/crate/quinn-proto/src/transport_parameters.rs:472), also line 479.

The `max_datagram_frame_size` and `min_ack_delay` branches unwrap integer decoding results. Empty or truncated values therefore panic instead of returning the existing malformed-parameter error.

Ordinary callers handling malformed handshake parameters cannot rely on the fallible parsing API.

Evidence: executed public-API probes confirmed panics for `20 00`, `20 01 40`, and a zero-length `min_ack_delay` parameter. Source inspection also establishes that the rustls adapter calls this parser.

Fix: decode within the advertised parameter length, propagate decoding errors, and require complete consumption of that value.

### 4. CID validation panics on short connection IDs

Location: [cid_generator.rs:147](/work/crate/quinn-proto/src/cid_generator.rs:147).

`ConnectionIdGenerator::validate` returns `Result<(), InvalidCid>` and is intended to determine whether a CID could have been generated. `HashedConnectionIdGenerator` instead unconditionally splits at byte three.

Evidence: executed public-API probes with valid `ConnectionId` objects of lengths 0, 1, and 2; all panicked with `mid > len`.

Callers checking arbitrary CIDs receive a panic instead of `InvalidCid`.

Fix: reject lengths other than the generator’s eight-byte CID length before splitting.

## Major

### 5. Rejected 0-RTT data permanently consumes the send window

Location: [connection/streams/state.rs:244](/work/crate/quinn-proto/src/connection/streams/state.rs:244).

`zero_rtt_rejected` clears outgoing streams and resets `data_sent`, but leaves `unacked_data` charged for the discarded early data. Those bytes can never subsequently be acknowledged.

Expected: the application can resend after rejection. Observed: filling the send window with early data leaves later writes blocked after the handshake succeeds.

Evidence: executed `review_zero_rtt_rejection_restores_send_window`, adapted from the existing rejection test. With a 13-byte send window and 13 early bytes, the subsequent write returned `Err(Blocked)` instead of `Ok(13)`.

Fix: release the discarded early data’s send-window accounting during rejection.

### 6. Stopping a reset receive stream can strand its concurrency slot

Location: [connection/streams/mod.rs:154](/work/crate/quinn-proto/src/connection/streams/mod.rs:154).

`RecvStream::stop` frees a completed/reset receive stream without queuing the resulting `MAX_STREAMS` update. Unlike the read-finalization and `received_reset` paths, it does not call `queue_max_stream_id`.

Evidence: executed `review_stop_reset_stream_replenishes_stream_credit`. With a one-stream limit, the client resets an empty stream, the server accepts and stops it, and both endpoints are driven. Opening the next client stream still returns `None`.

An application discarding reset streams can stop accepting further streams until unrelated incoming traffic triggers an update.

Fix: queue stream-limit updates when `stop` frees receive state.

### 7. Stop followed by reset releases receive credit twice

Location: [connection/streams/state.rs:356](/work/crate/quinn-proto/src/connection/streams/state.rs:356).

Stopping an unread stream already releases credit for its discarded bytes. Processing the subsequent reset releases `final_offset - bytes_read` again, including those same bytes.

Evidence: both an internal accounting regression and an end-to-end regression failed. With a 16-byte receive window, stopping and resetting eight unread bytes allowed the peer to write another **24 bytes**, rather than 16.

This defeats the configured buffering bound during normal stream cancellation.

Fix: for stopped streams, release only credit attributable to newly learned final-offset bytes.

### 8. Shrinking then expanding the receive window grants excess credit

Location: [connection/streams/state.rs:873](/work/crate/quinn-proto/src/connection/streams/state.rs:873).

Expansion increases `local_max_data` without first cancelling outstanding `receive_window_shrink_debt`.

Evidence: executed internal and end-to-end regressions. Changing an unused receive window from 100 to 50 and back to 100 allowed a peer write of **150 bytes**, although no data had been consumed.

Repeated adjustments can advertise credit beyond the intended window.

Fix: cancel outstanding shrink debt before adding any net increase to advertised credit.

### 9. Datagram eviction ignores the incoming datagram’s size

Location: [connection/datagrams.rs:39](/work/crate/quinn-proto/src/connection/datagrams.rs:39).

The eviction loop checks only the bytes already queued. It then appends the new datagram, exceeding `datagram_send_buffer_size` rather than making space as documented.

Evidence: executed `review_datagram_drop_makes_room_for_new_data`. With an eight-byte buffer, queuing `12345678` followed by `x` with dropping enabled transmitted the older datagram instead of evicting it.

Callers receive weaker memory bounds and stale-data eviction than configured.

Fix: include the incoming length in capacity accounting and explicitly handle individual datagrams larger than the buffer.

### 10. Valid wider integer encodings are rejected

Location: [transport_parameters.rs:487](/work/crate/quinn-proto/src/transport_parameters.rs:487).

The parser compares the advertised value length against `value.size()`, which computes the minimum encoding length. QUIC variable-length integers may use wider encodings.

Evidence: an executed public-API probe accepted `04 01 01` but rejected the equivalent `04 02 40 01` with `Malformed`.

A conforming peer using a wider encoding can therefore fail transport-parameter negotiation.

Fix: validate bytes actually consumed within the parameter’s length, rather than comparing against the decoded value’s minimum size.

### 11. BBR’s bandwidth estimate never ages out a higher sample

Location: [congestion/bbr/bw_estimation.rs:65](/work/crate/quinn-proto/src/congestion/bbr/bw_estimation.rs:65).

The rolling maximum receives updates only when a sample exceeds its current maximum. Lower samples never advance the filter, so an old peak cannot expire.

Evidence: executed `review_bandwidth_estimate_ages_out_old_peak`. After establishing 1,000,000 bytes/s and providing 25 subsequent rounds at 100,000 bytes/s, the ten-round filter still reported 1,000,000 bytes/s.

Source inspection shows that BBR uses this estimate for its target congestion window and reported pacing rate. Callers selecting the experimental BBR controller can retain an inappropriate model after available bandwidth falls.

Fix: feed all non-application-limited samples into the rolling filter; also permit higher application-limited samples.

## Minor

### 12. `max_size` reports capability when datagrams are disabled locally

Location: [connection/datagrams.rs:69](/work/crate/quinn-proto/src/connection/datagrams.rs:69).

The documentation promises `None` when disabled locally, but the implementation checks only peer support.

Evidence: executed `review_datagram_max_size_respects_local_disabled`; it returned `Some(1414)` after local datagrams were disabled. `send` independently returns `Disabled`.

Fix: make the capability query check local enablement too.

### 13. Strict Clippy fails on the supplied toolchain

Locations: [connection/assembler.rs:333](/work/crate/quinn-proto/src/connection/assembler.rs:333) and [connection/packet_crypto.rs:91](/work/crate/quinn-proto/src/connection/packet_crypto.rs:91).

On Rust/Clippy 1.99.1, strict package checks fail on `derivable_impls` and `manual_filter`, with both default and extended features. These failures predate the review edits.

Fix: apply the corresponding small transformations. Older-toolchain behavior was not tested, so I cannot establish whether these are exclusively toolchain drift.

### 14. Public error and panic documentation needs consolidation

Representative locations: [coding.rs:21](/work/crate/quinn-proto/src/coding.rs:21), [connection/mod.rs:421](/work/crate/quinn-proto/src/connection/mod.rs:421).

The advisory pass reported **38 missing `# Errors` sections and 12 missing `# Panics` sections**. Several panic warnings concern internal invariants and are not independent runtime defects. Confirmed public documentation gaps include the direction restrictions on `send_stream`/`recv_stream`.

Fix: document recoverable outcomes and caller preconditions explicitly, prioritizing the public entry points.

Evidence: advisory Clippy output and source inspection.

### 15. Twenty-three reachable public types lack `Debug`

Representative locations: [endpoint.rs:1142](/work/crate/quinn-proto/src/endpoint.rs:1142) and [token.rs:67](/work/crate/quinn-proto/src/token.rs:67).

The advisory pass flagged 26 types; three are inaccessible internal range-set types, leaving **23 publicly reachable types**. Missing implementations hinder diagnostics; for example, `TokenReuseError` also prevents using standard `Result::expect`.

Fix: provide appropriate implementations, redacting sensitive state where necessary.

Evidence: advisory output and public-export/source inspection.

### 16. Four unchecked integer calls lack safety explanations

Locations: [connection/send_buffer.rs:99](/work/crate/quinn-proto/src/connection/send_buffer.rs:99), also line 118; [connection/mod.rs:3261](/work/crate/quinn-proto/src/connection/mod.rs:3261); [lib.rs:280](/work/crate/quinn-proto/src/lib.rs:280).

All four calls to `VarInt::from_u64_unchecked` lack adjacent safety explanations. Future changes must reconstruct the required range invariants.

Fix: document those invariants or use checked construction where appropriate. This is a maintainability finding, not a demonstrated undefined-behavior finding.

Evidence: advisory output and source inspection.

## Verification and artifacts

Commands used `RUSTUP_AUTO_INSTALL=0`, offline Cargo operation, and two build jobs.

From `/work/crate`:

- `cargo test --locked -p quinn-proto` — passed: **263 unit tests, 3 doc tests**.
- `cargo test --locked -p quinn-proto --no-default-features` — passed: **154 unit tests, 3 doc tests**.
- `cargo test --locked -p quinn-proto --features qlog,arbitrary,platform-verifier,rustls-log` — passed: **263 unit tests, 3 doc tests**.
- `cargo test --locked -p quinn-proto --no-default-features --features rustls-aws-lc-rs` — passed: **256 unit tests, 3 doc tests**.
- `cargo fmt --all --check` — passed through the skill gate.
- `cargo clippy --locked -p quinn-proto --all-targets -- -D warnings` — failed with the two diagnostics above. The extended-feature equivalent failed identically.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p quinn-proto --no-deps --document-private-items --features qlog,arbitrary,platform-verifier,rustls-log` — passed.
- `cargo +nightly-2026-10-01 miri test --locked -p quinn-proto --no-default-features --lib connection::send_buffer::tests` — passed: **7 tests**.

`bash /work/skill/scripts/check.sh --audit /work/crate` passed formatting, but its workspace-wide all-feature Clippy, documentation, test, and advisory builds were blocked by **missing Go, required by AWS-LC FIPS**. This is an environment block, not a demonstrated crate defect. The package-level advisory pass completed separately with 216 warnings, triaged above. The heuristic scan was also run.

Regression artifacts:

- `/work/tmp/review`: scratch workspace with test-only additions in `tests/mod.rs`, `connection/streams/state.rs`, and `congestion/bbr/bw_estimation.rs`. `cargo test --locked -p quinn-proto review_ -- --nocapture` produced **12 expected regression failures**.
- `/work/tmp/probes`: separate public-API harness referencing the original crate. `cargo test --offline -- --nocapture` produced **4 failures and 1 passing VarInt boundary test**.
- Both scratch locations pass formatting checks. A directory comparison confirmed that only the three test-bearing scratch files differ from the original workspace.

**Finding count: 16 — 4 Blocker, 7 Major, 5 Minor.** Coverage was selective on aarch64 Linux. MSRV 1.74.1, cross-platform targets, full Miri, and the feature powerset were not tested. No dedicated security assessment or exploit investigation was performed. Last `/work/clock` reading: **10.6071 minutes used of 45.0**.