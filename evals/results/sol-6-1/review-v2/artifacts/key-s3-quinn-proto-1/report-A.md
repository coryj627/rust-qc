The existing tests pass, but the review found 21 issues: 4 High, 12 Medium and 5 Low. Fifteen behavioral issues were reproduced with local regression probes; the seeded-RNG finding rests on source inspection. Production files were left unchanged.

Locations below are relative to `/work/crate`.

## High

### 1. Disabling server configuration panics with pending connections

`quinn-proto/src/endpoint.rs:211`, `:540`.

`set_server_config(None)` should allow the endpoint to stop accepting new connections without making ordinary processing panic. With an `Incoming` pending, receiving a retransmitted Initial panics at the configuration unwrap. Accepting that pending connection without an explicit override also panics.

Evidence: `review_disable_server_with_pending_incoming` and `review_accept_pending_after_disable_server` both reproduced the panics. This can interrupt an application’s shutdown or configuration-change flow.

retain the configuration needed by pending incoming connections, or handle its absence explicitly.

### 2. A rejected ordered read destroys the receive stream

`quinn-proto/src/connection/streams/recv.rs:269`, `:272`.

An ordered read after an unordered read should return `IllegalOrderedRead` while preserving the stream. Instead, `Chunks::new` removes the receive state before checking ordering; returning the error drops that state and any unread data.

Evidence: `review_ordered_error_preserves_unordered_stream` read `abc` from `abcdef`, received the expected ordering error, then received `ClosedStream` when attempting to resume unordered reading. The corresponding control read successfully returned `def`.

validate ordering before removing the map entry, or restore the entry on failure.

### 3. Legal peer ACK-delay parameters can trigger a panic

`quinn-proto/src/connection/ack_frequency.rs:49`.

With ACK-frequency control enabled, the peer’s minimum delay must remain usable even when it exceeds the RTT-based preferred upper bound. The code instead calls `clamp` with inverted bounds.

Evidence: `review_peer_min_ack_delay_above_rtt_is_supported` successfully decoded parameters specifying a 100 ms minimum and maximum ACK delay, then reproduced `assertion failed: min <= max` with a 10 ms RTT. The helper is called when constructing ACK-frequency requests.

make the upper bound at least the peer’s minimum delay.

### 4. Optional transport-parameter integers panic instead of returning an error

`quinn-proto/src/transport_parameters.rs:472`, `:479`.

Malformed parameter values should produce the existing decoding error. These branches instead unwrap a failed integer decode.

Evidence: the public-API probe decoded `20 00`—a DATAGRAM-size parameter with an empty value—and panicked with `UnexpectedEnd`. An empty minimum-ACK-delay value reproduced the same failure in the other branch.

Consequently, callers cannot handle these ordinary decoder errors through `Result`.

decode within the declared parameter length, propagate errors with `?`, and verify complete value consumption.

## Medium

### 5. Stop followed by reset returns receive credit twice

`quinn-proto/src/connection/streams/state.rs:356`.

Stopping a receive stream already credits its discarded unread bytes. A subsequent reset credits those bytes again because it subtracts `bytes_read` rather than accounting for the earlier stop.

Evidence: with a 100-byte receive window, `review_stop_then_reset_returns_credit_once` received and stopped 50 bytes, processed the peer’s reset, then allowed 150 additional unread bytes. The final write returned `Ok(50)` instead of `Blocked`.

This exceeds the receiver’s configured buffering budget. Existing `stopped_reset` coverage checks cleanup but not this accounting.

for stopped streams, credit only any increase beyond the previously accounted end offset.

### 6. Restoring a receive window grants additional unintended credit

`quinn-proto/src/connection/streams/state.rs:874`.

Increasing a previously reduced window should first cancel outstanding shrink debt. Instead, the implementation immediately increases `local_max_data` while retaining that debt.

Evidence: `review_receive_window_restore_does_not_expand_credit` changed the window from 100 to 50 and back to 100 without reading data. The sender could then write 150 bytes. The unchanged-window control allowed 100.

consume shrink debt before granting additional advertised credit.

### 7. Zero MTU minimum change prevents discovery from finishing

`quinn-proto/src/connection/mtud.rs:333`.

`minimum_change(0)` is accepted without a documented restriction. It makes the stopping comparison permanently false, so the search repeatedly probes an unchanged size.

Evidence: `review_zero_mtu_minimum_change_eventually_finishes`, using a 10 ms RTT, exhausted 100 simulation steps after sending 49 successful probes with no losses. Discovery remained at 1451 bytes.

Applications selecting this boundary value incur continuing unnecessary traffic and wakeups.

reject or clamp zero, or terminate explicitly when the search cannot advance.

### 8. Failed connection attempts retain allocated CID routing state

`quinn-proto/src/endpoint.rs:344`, `:355`.

A connection attempt that returns an error should release its provisional state. `new_cid` inserts routing state before the fallible crypto-session creation, and the error path does not remove it.

Evidence: `review_failed_connect_releases_cid` used an invalid server name. `connect` returned `InvalidServerName` and reported zero open connections, but retained one known CID.

Repeated failed attempts therefore retain memory and consume CID capacity.

roll back the provisional CID on error, or defer committing routing state until session creation succeeds.

### 9. Valid longer integer encodings are rejected

`quinn-proto/src/transport_parameters.rs:487`.

QUIC variable-length integers may use a longer permitted encoding. The decoder compares the declared length with the decoded value’s *minimum* encoding size, incorrectly requiring canonical encoding.

Evidence: the public probe supplied `04 02 40 01`, representing `initial_max_data = 1` using two bytes. It returned `Err(Malformed)`. A control confirmed that the primitive integer decoder accepts `40 01`.

This can reject otherwise valid peer transport parameters.

compare the declared length with bytes actually consumed, rather than `value.size()`.

### 10. Datagram eviction ignores the incoming datagram’s size

`quinn-proto/src/connection/datagrams.rs:39`.

The documented send-buffer limit should cause older queued datagrams to be evicted when necessary. The eviction condition checks only the bytes already queued.

Evidence: with a four-byte buffer, `review_datagram_drop_respects_buffer_limit` queued `aaaa` and `bbbb` with dropping enabled. Both were delivered; the expected retained queue contained only `bbbb`. The control correctly blocked the second send when dropping was disabled.

account for the incoming length during eviction, and define handling for a single datagram larger than the buffer.

### 11. TLS configuration conversion succeeds before session creation panics

`quinn-proto/src/crypto/rustls.rs:384`, `:366`, `:515`.

The documentation says TLS 1.3 must be enabled for conversion to succeed. Conversion checks the provider’s initial cipher suite but does not validate the enabled protocol versions.

Evidence: with `rustls/tls12` enabled, a TLS-1.2-only client configuration converted successfully. Calling `connect` then panicked on `General("TLS 1.3 support is required for QUIC")`. A server configuration with `max_early_data_size = 1024` likewise converted and later panicked during acceptance.

Callers receive delayed panics after a successful fallible conversion.

validate QUIC compatibility during conversion and preserve recoverable session-creation errors.

### 12. Qlog RTT values use seconds instead of milliseconds

`quinn-proto/src/connection/paths.rs:267`.

Qlog RTT metrics require milliseconds. All four RTT-related fields are populated using `as_secs_f32()`.

Evidence: `review_qlog_rtt_uses_milliseconds` supplied 100 ms and observed `Some(0.1)` instead of `Some(100.0)`.

This makes latency measurements in exported traces 1,000 times too small.

convert these durations to fractional milliseconds.

### 13. CID validation panics on short connection IDs

`quinn-proto/src/cid_generator.rs:147`.

`validate` should return `InvalidCid` for an ID that could not have been generated. Its unconditional `split_at(3)` instead panics for shorter IDs.

Evidence: public-API calls reproduced panics for CID lengths 0, 1 and 2. Longer tested lengths returned validation results.

Applications using this public validator must currently add an undocumented length check.

reject an incorrect total length before splitting.

### 14. Safe stream-ID construction can violate the VarInt invariant

`quinn-proto/src/lib.rs:257`, `:280`, `:301`.

`StreamId::new` accepts an unrestricted `u64` index, although only 60 bits fit. Conversion then bypasses the documented VarInt range requirement.

Evidence: the public probe constructed a client unidirectional stream with index `1 << 60`. Converting it produced `4611686018427387906`, exceeding `VarInt::MAX`. Source inspection shows encoding subsequently unwraps a failed checked conversion.

validate the index and provide a checked construction path; constrain every generation path to the representable range.

### 15. An explicit RNG seed still invokes the OS entropy source

`quinn-proto/src/endpoint.rs:77`; contract at `quinn-proto/src/config/mod.rs:154`.

The seed option is documented as supporting environments without an entropy source. However, `map_or` eagerly evaluates `StdRng::from_os_rng()` even when the seed is present.

Evidence: source inspection establishes the unconditional entropy call; the dependency documents that this call panics when entropy acquisition fails. An entropy failure was not injected locally.

use lazy default evaluation, such as `map_or_else`.

### 16. Default-feature documentation fails

`quinn-proto/src/crypto/rustls.rs:279`, `quinn-proto/src/connection/qlog.rs:45`.

Documentation should build under the default feature configuration. Unconditional links reference items that are compiled out.

Evidence: strict public rustdoc failed on `with_platform_verifier`; adding private-item documentation also exposed the missing `QlogStream` link. Documentation passed with both relevant features enabled.

conditionally document these links, or use references that remain valid when the features are disabled.

## Low

### 17. Datagram size reporting contradicts local disablement

`quinn-proto/src/connection/datagrams.rs:69`.

`max_size()` documents `None` when datagrams are disabled locally, but does not check local configuration.

Evidence: `review_datagram_max_size_disabled_locally` observed `Some(1414)` after local disablement, although `send` returns `Disabled`.

apply the same local-enable check in both methods.

### 18. Stream-accessor panic preconditions are undocumented

`quinn-proto/src/connection/mod.rs:421`, `:432`.

The public documentation only says these methods provide stream control. Source inspection shows that requesting the wrong half of a unidirectional stream immediately asserts, before a caller can receive a `ClosedStream` error.

document the direction requirements and panics, or expose a fallible accessor.

The advisory pass flagged 37 missing `# Errors` sections and 11 missing `# Panics` sections. These two accessors have concrete undocumented caller preconditions; invariant-only panic warnings were not treated as additional defects.

### 19. Public types lack useful diagnostic implementations

`quinn-proto/src/token.rs:67`, `quinn-proto/src/connection/paths.rs:285`.

The default-feature advisory pass identified 21 reachable public types without `Debug`, excluding three inaccessible range-set implementation types.

A concrete consequence is that `Result<(), TokenReuseError>` cannot be used with ordinary `unwrap` or `expect`; the missing implementations also hinder diagnostics.

derive `Debug` where appropriate and provide selective manual implementations for stateful types.

### 20. Unchecked conversions lack invariant explanations

`quinn-proto/src/connection/send_buffer.rs:99`, `:118`; `quinn-proto/src/connection/mod.rs:3261`; `quinn-proto/src/lib.rs:280`.

Four unchecked VarInt conversions lack SAFETY comments explaining why their values fit. This obscures the bounds future changes must preserve; finding 14 demonstrates that one assumed bound already fails.

Evidence: advisory lint output and source inspection.

establish the bounds first, then document them beside each unchecked conversion.

### 21. Strict Clippy fails on the supplied toolchain

`quinn-proto/src/connection/assembler.rs:333`, `quinn-proto/src/connection/packet_crypto.rs:91`.

Both default and expanded-feature Clippy runs failed under Rust 1.99.1 because `-D warnings` promotes `derivable_impls` and `manual_filter` diagnostics to errors.

derive the enum default and use `Option::filter`. These failures predate the review edits; comparison against an older compiler was unavailable.

## Verification

Host: aarch64 Linux, Rust 1.99.1, Clippy 0.1.99, rustfmt 1.10.0. Checks used installed tools and prefetched dependencies; no tools or dependencies were installed.

Executed from `/work/crate`:

- `cargo test --locked -p quinn-proto` — passed: 263 unit tests, 3 doctests.
- `cargo test --locked -p quinn-proto --no-default-features` — passed: 154 unit tests, 3 doctests.
- `cargo test --locked -p quinn-proto --no-default-features --features rustls-aws-lc-rs` — passed: 256 unit tests, 3 doctests.
- `cargo test --locked -p quinn-proto --features arbitrary,platform-verifier,qlog,rustls-log,rustls-aws-lc-rs` — passed: 263 unit tests, 3 doctests.
- `cargo fmt --all --check` — passed.
- `cargo clippy --locked -p quinn-proto --all-targets -- -D warnings` — failed with the two diagnostics above. Adding the expanded feature list produced the same failures.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p quinn-proto --no-deps --document-private-items` — failed with the two links above. Without private items, the platform-verifier link still failed. With the expanded feature list, documentation passed.
- `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` — formatting passed; all-features Clippy, documentation, tests and advisory compilation were blocked by the FIPS dependency’s missing Go build requirement. This is an environment block.
- `python3 -I /work/[tooling]/scripts/[tooling] /work/crate` — completed; no additional actionable in-scope finding was established from its heuristic matches.
- `cargo +nightly-2026-10-01 miri test --locked -p quinn-proto --no-default-features` — incomplete: sandbox exit 137. This does not establish a defect.
- The same Miri command with filter `connection::send_buffer::tests::fragment_` — passed both selected tests.

The separate package advisory Clippy run completed with 210 warnings. Detailed records are in `/work/tmp/audit-package.log`.

Scratch verification:

- `/work/tmp/review`: `cargo test --locked -p quinn-proto --features qlog review_ -- --nocapture` — 3 controls passed; 12 regression assertions failed.
- `/work/tmp/public_probe`: `cargo test --offline -- --nocapture` — 1 control passed; 7 regression assertions failed.
- Scratch baseline with `-- --skip review_` — all 263 original unit tests and 3 doctests passed.
- Scratch formatting passed. Scratch Clippy passed with only the two pre-existing diagnostics explicitly allowed.

Edits consist solely of appended tests in scratch copies of `tests/mod.rs`, `connection/paths.rs` and `connection/ack_frequency.rs`, plus the standalone probe crate. Regression logs are `/work/tmp/regression-final.log` and `/work/tmp/public-final.log`.

Finding count: 21. Coverage concentrated on stream lifecycle, flow-control accounting, datagrams, endpoint configuration, transport parameters and configuration boundaries. FIPS, the complete feature powerset, MSRV 1.74.1 and other targets remain unverified; cargo-hack and additional toolchains/targets were unavailable. No dedicated security assessment or exploit work was performed. Last clock reading: 24.9905 of 45 minutes used, excluding slot waits.