31 findings; clock read 30 of 45 at my last check. The report is NOT in report.md: the harness refused my Write ("Subagents should return findings as text"), and I did not route around that through Bash. So the full report is below, over the 40-word limit, and needs saving to report.md if it is to count. `report.md` currently holds only a stale in-progress stub I wrote through Bash before I knew about the refusal; replace it.

Process notes:
- A stray `cat > /tmp/null` of mine hung one command and cost about 10 clock minutes. It also created an empty file outside the workspace, which I removed. I killed that one process.
- I also ran `ps` to find that process, which lists processes outside the workspace.
- `crate/` was not changed.
- No text in the repository or its output addressed me as an instruction.

# Audit of h2 0.4.12

Verdict: do not expose this release to untrusted peers without mitigation. A remote peer can panic a default server with one header block, and a server can panic a default client with four ordinary frames. Padded DATA and dropped request bodies break connection flow control, some malformed messages are accepted, and some recoverable ones kill the whole connection. The existing test suite passes and does not cover these paths.

Counts: 3 Blocker, 17 Major, 9 Minor, 2 Note.

Probes are in the workspace at `probe/tests/p.rs`, `probe/tests/q.rs` and `probe/tests/fuzz.rs`. They drive the public API through the `tests/h2-support` mock transport. A name in brackets, such as [p20], is the test that shows the result. "Read only" means I did not run it.

## Blocker

### Too many distinct header names panics the connection task
Location: src/frame/headers.rs:914 (HeaderBlock::load, `self.fields.append`).
Problem: `HeaderMap::append` panics when the map cannot grow. A block with 24,600 distinct names (about 221 KB in HEADERS plus CONTINUATION) panicked in http-1.5.0 `header/map.rs:1433` with "size overflows MAX_SIZE: MaxSizeReached"; 24,000 names were accepted. The 16 MiB default limit does not stop it, and neither did `max_header_list_size(1 << 20)`. No authentication or prior state is needed. From reading, the same line handles client responses, trailers and PUSH_PROMISE.
Established: ran [p20] on a default server (40,000 names panicked, 20,000 passed) and [q08] with the 1 MiB limit.
Fix: use `try_append`, and on `MaxSizeReached` set `is_over_size` so the request gets the normal oversize handling.

### Push promise, then a 1xx, then a final response panics the client
Location: src/proto/streams/counts.rs:111, reached from src/proto/streams/recv.rs:171, because src/proto/streams/state.rs:157-165 returns `initial = true` each time while the stream stays ReservedRemote.
Problem: the legal sequence HEADERS 200, PUSH_PROMISE, HEADERS 100 and HEADERS 200 on the promised stream counts the stream twice and hits `assert!(!stream.is_counted)`. Push is on by default for clients, so any server can do this. From reading, the panic happens with the streams mutex held, so it also poisons it.
Established: ran [p01] on a default client.
Fix: skip the increment when `stream.is_counted`, or track "reserved, 1xx seen" as its own state.

### More pushed responses than max_concurrent_streams panics the client
Location: src/proto/streams/counts.rs:110, reached from src/proto/streams/recv.rs:171.
Problem: `Recv::open` checks the limit at PUSH_PROMISE, but a reservation does not count. With `max_concurrent_streams(1)`, two promises pass, then HEADERS on both hits `assert!(self.can_inc_num_recv_streams())`.
Established: ran [p02].
Fix: check the limit in `recv_headers` for reserved streams and refuse with REFUSED_STREAM.

## Major

### RecvStream drop panics on a poisoned lock, and aborts the process if unwinding
Location: src/proto/streams/streams.rs:1415 (`clear_recv_buffer`, `lock().unwrap()`), called from src/share.rs:487.
Problem: after any panic under the streams lock, dropping a `RecvStream` panics on `PoisonError`. If that drop happens during unwinding, the process aborts, taking every connection with it. `drop_stream_ref` at streams.rs:1477 guards against this with `thread::panicking()`; this function does not. A `RecvStream` owned by another task gets an ordinary panic instead (from reading). The abort needs the stream to be dropped on the unwinding thread, for example when a handler and the connection share one task.
Established: ran [q03]: "panic in a destructor during cleanup", SIGABRT.
Fix: return early on a poisoned lock, as `drop_stream_ref` does.

### Dropping pushed responses past the reset limit panics
Location: src/proto/streams/prioritize.rs:828 (`find_mut(&pp.promised_id()).unwrap()`).
Problem: 60 `push_request` calls with each `SendPushedResponse` dropped panics on the `unwrap`. 20 did not. Past the default limit of 50 stored resets, the promised stream is released while its PUSH_PROMISE is still queued.
Established: ran [q03] and [p19_push_dropped_many].
Fix: skip the frame if the stream is gone, or keep the stream until the PUSH_PROMISE is written.

### Padded DATA is not counted in flow control
Location: src/proto/streams/recv.rs:580 and src/proto/streams/streams.rs:584 (`frame.payload().len()`), after src/frame/util.rs:19 strips padding.
Problem: RFC 9113 section 6.1 counts the whole payload, including padding. A peer sent 300 frames of 1 data byte and 254 padding bytes, 76,800 wire bytes on a 65,535 window. h2 raised no FLOW_CONTROL_ERROR and, after the app released all 300 data bytes, sent no WINDOW_UPDATE. A conforming padded sender therefore runs out of window and hangs (inferred from that accounting). A hostile sender is not limited by the window.
Established: ran [p03]. The existing `data_padding` test checks only payload length.
Fix: keep the full frame payload length on `frame::Data`, use it for accounting, and release the padding share automatically.

### Dropping a request body keeps buffered bytes out of the connection window
Location: src/proto/streams/streams.rs:1414 (`clear_recv_buffer`).
Problem: buffered data is discarded, but `in_flight_recv_data` is not released until the stream closes and all references go. With 49,152 bytes buffered, the body dropped and the response stream kept open, no WINDOW_UPDATE was sent. A handler that ignores the body and streams its response (long polling, server-sent events) starves other streams; about 64 KB of unread bodies would stall the connection (inferred).
Established: ran [q01].
Fix: release `in_flight_recv_data` to the connection there and zero it.

### DATA above the GOAWAY limit never returns connection window
Location: src/proto/streams/streams.rs:551-559 (`recv_data`).
Problem: RFC 9113 section 6.8 requires this DATA to count toward the connection window. This branch returns `Ok(())` without `ignore_data`, unlike the branch below it (line 570). After a final GOAWAY with last stream 1, 48 KB raced on stream 3 got no WINDOW_UPDATE, so an upload on stream 1 can stall during graceful shutdown.
Established: ran [q04].
Fix: call `ignore_data(sz)` in this branch.

### A malformed block followed by CONTINUATION kills the connection
Location: src/codec/framed_read.rs:173-177 and 345-349.
Problem: `MalformedMessage` returns a stream reset without storing `Partial`, so the next CONTINUATION is "unexpected" and becomes GOAWAY PROTOCOL_ERROR. The rest of the block is never HPACK decoded, despite the comment at src/frame/headers.rs:881 saying it must be.
Established: ran [p04] (`connection: close` in a split block gave GOAWAY, not RST_STREAM).
Fix: record malformed state on the frame, decode through END_HEADERS, then reset the stream.

### Malformed state is forgotten when a block is split across frames
Location: src/frame/headers.rs:854 and 930-938.
Problem: `malformed` is a local variable, and a `NeedMore` error returns before it is checked. A request with `connection: close` or a duplicate `:path`, plus a header cut at the frame boundary, reached the app as valid with the offending field silently removed. Whether a message is rejected depends on where the sender cut the frame.
Established: ran [p05] and [p05b].
Fix: store the flag in `HeaderBlock` and check it after the final fragment.

### One invalid header field closes the whole connection
Location: src/codec/framed_read.rs:178-181, with errors from src/hpack/header.rs:72-100.
Problem: RFC 9113 section 8.2.1 makes these malformed messages, which are stream errors. A newline in a value, an uppercase name, `:foo` or a bad `:method` on stream 3 sent GOAWAY PROTOCOL_ERROR, ending stream 1 too.
Established: ran [p12a] to [p12d].
Fix: keep HPACK state correct, report the field as malformed, and keep GOAWAY for real HPACK failures.

### A response without :status is reported as 200 OK
Location: src/client.rs:1676.
Problem: the builder default applies when `:status` is missing. HEADERS with only `x: y` gave the client `Ok(200)`, so a broken upstream looks like success.
Established: ran [p06].
Fix: reset the stream with PROTOCOL_ERROR when `pseudo.status` is `None`.

### Requests missing :path or :authority are accepted
Location: src/server.rs:1558-1567 and 1593-1609.
Problem: `GET` with `:scheme` and no `:path` or `:authority` reached the app with an empty URI. So did `CONNECT` with no `:authority`. RFC 9113 sections 8.3.1 and 8.5 require these fields.
Established: ran [p07a] and [p07d].
Fix: treat both as malformed.

### Oversized trailers are silently truncated
Location: src/proto/streams/recv.rs:348-370.
Problem: `recv_trailers` does not check `is_over_size`. With a 300-byte limit, `grpc-keep`, `x-big1`, `x-big2`, `grpc-status: 13` reached the app as `Ok(Some({grpc-keep, x-big1}))`. Losing `grpc-status` changes what a caller concludes about an RPC, with no signal.
Established: ran [p08].
Fix: reset the stream when trailers are over size.

### A 304 with content-length and an empty final DATA frame is reset
Location: src/proto/streams/recv.rs:640-648.
Problem: the 204 and 304 exemption at recv.rs:195 covers only END_STREAM on HEADERS. `304, content-length: 100` then an empty DATA with END_STREAM gave a 304 whose body read failed with `Reset(PROTOCOL_ERROR)`. RFC 9110 section 8.6 allows that header on a 304.
Established: ran [p10].
Fix: treat a 304 like `ContentLength::Head`.

### Empty and conflicting content-length values reach the app
Location: src/proto/streams/recv.rs:178 (first value only) and src/frame/headers.rs:326-344 (empty parses as 0).
Problem: `content-length: 0` plus `content-length: 100` was accepted, and so was an empty value. From reading, both values stay in the `HeaderMap`; the probe printed names only. A proxy forwarding to HTTP/1.1 could send conflicting framing, which is a request smuggling setup.
Established: ran [p11a] and [p11b].
Fix: reject empty values and check all values with `get_all`.

### Credentials in the URI are sent in :authority
Location: src/frame/headers.rs:597.
Problem: `https://user:s3cret@x.com/` is sent as `:authority: user:s3cret@x.com`. RFC 9113 section 8.3.1 forbids userinfo there.
Established: ran [p14].
Fix: strip through the last `@`.

### Dropping a pushed response never resets the promised stream
Location: src/proto/streams/prioritize.rs:832 and src/proto/streams/send.rs:243-251.
Problem: the stream is not send-ready during `is_pending_push`, and after the PUSH_PROMISE it is rescheduled only if `pending_send` is non-empty. The peer got no RST_STREAM in 700 ms and, from reading, never would, so a client awaiting that push hangs.
Established: ran [q02].
Fix: also reschedule when `state.is_scheduled_reset()`.

### Unbounded memory from push reservations
Location: src/proto/streams/streams.rs:780.
Problem: reserved streams count toward no limit, as the code's own TODO says. A server can promise every even stream ID, and each stores a `Stream` and a request until the parent closes.
Established: read only.
Fix: limit reservations and refuse beyond the limit.

### The HPACK encoder table follows the peer's setting with no cap
Location: src/proto/settings.rs:140, src/codec/mod.rs:94, src/hpack/encoder.rs:30.
Problem: any SETTINGS_HEADER_TABLE_SIZE up to 4 GiB is adopted, though RFC 7541 section 4.2 lets an encoder use less. A client can grow per-connection memory if responses carry unique indexed headers.
Established: read only.
Fix: cap it locally and make the cap configurable.

## Minor

### PUSH_PROMISE on a locally reset stream trips a debug assertion
Location: src/proto/connection.rs:446, through src/proto/streams/streams.rs:767.
Problem: `ensure_recv_open` returns the stored `Reset(.., User)` as a new stream error, and `debug_assert_eq!(initiator, Library)` panics. This is remote-triggered but debug builds only. A remote reset instead of a user reset gave GOAWAY [q05, q09]; I did not work out why that path differs.
Established: ran [q10]; first found by the fuzz harness.
Fix: treat a closed parent as a PROTOCOL_ERROR connection error.

### Protocol error codes are wrong for several frame errors
Location: src/codec/framed_read.rs:178-181 and 201-256, src/frame/go_away.rs:43.
Problem: wrong-size PING, SETTINGS, RST_STREAM, WINDOW_UPDATE and PRIORITY give PROTOCOL_ERROR, not FRAME_SIZE_ERROR. An oversized initial window should be FLOW_CONTROL_ERROR, and HPACK failures should be COMPRESSION_ERROR. A zero WINDOW_UPDATE on a stream closes the connection, and GOAWAY on a non-zero stream ID is accepted.
Established: ran [p17c], [p17d], [p18].
Fix: map `frame::Error` to the right `Reason`, and pass the head to `GoAway::load`.

### PUSH_PROMISE with an empty first fragment is rejected
Location: src/frame/headers.rs:435.
Problem: a promised ID with the block carried in CONTINUATION is legal but gets GOAWAY.
Established: ran [p13].
Fix: require 4 bytes, not 5.

### A header list exactly at the limit is rejected
Location: src/frame/headers.rs:869 and 911.
Problem: exactly 205 bytes under a limit of 205 got an immediate END_STREAM response rather than reaching the app (from the code, the 431); a limit of 206 was accepted.
Established: ran [p15].
Fix: use `<=`.

### Pseudo-headers in trailers are silently dropped
Location: src/proto/streams/recv.rs:361.
Problem: `:path` in trailers disappeared with no error; RFC 9113 section 8.1 makes that malformed.
Established: ran [p09].
Fix: reset the stream.

### HPACK size update is accepted mid-block after a frame boundary
Location: src/hpack/decoder.rs:186.
Problem: `can_resize` restarts on each `decode` call, so an update rejected within one frame is accepted at the start of a CONTINUATION.
Established: ran [p17a] and [p17b].
Fix: keep the flag across fragments.

### send_request emits malformed requests instead of returning an error
Location: src/client.rs:1626-1638.
Problem: `GET example.com` sends no `:scheme`, and `CONNECT /x` sends `:scheme http` with no `:authority`. The failure surfaces later as a peer reset.
Established: ran [q07a] and [q07b].
Fix: return a `UserError`.

### Header size recomputation omits :protocol
Location: src/frame/headers.rs:975-980.
Problem: `set_pseudo!` counts `:protocol`, and `calculate_header_list_size` does not, so the total drops when a block continues.
Established: read only.
Fix: add `pseudo_size!(protocol)`.

### Hygiene from the lint and rustdoc passes
Location: mainly src/proto/streams/ and src/frame/.
Problem:
- clippy with `-D warnings` fails on 8 errors, including src/hpack/huffman/table.rs:265. I treat these as toolchain drift but did not run the project's CI toolchain.
- rustdoc with private items fails on two bare URLs (src/hpack/decoder.rs:508, src/frame/headers.rs:959).
- Advisory counts in src/, some possibly in test modules: 110 asserts without messages, about 85 `unwrap` calls, 24 missing `# Errors`, 7 missing `# Panics`, 10 wildcard imports.
- Box-drawing diagrams appear in src/hpack/decoder.rs and src/proto/streams/state.rs, and TODOs have no issue references.
- The one `unsafe` block (src/hpack/header.rs:283) looks sound from reading its constructors.
Established: ran the gate and scan.
Fix: routine cleanup, starting with assert messages on frame-reachable paths.

## Note

### Regression coverage
Add these probes as tests. The fuzz harness is weak evidence of absence: it did not rediscover the Blockers.

### Debug-only assertions on drop
`Counts::drop` (src/proto/streams/counts.rs:282) and `Store::drop` (src/proto/streams/store.rs:231) fired in debug fuzz runs. I did not investigate them, so they may be harness shutdown artifacts rather than leaks.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail, as above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, 2 bare URLs.
- `cargo test --workspace --all-features`: pass, including the stress test.
- `cargo test --test p -- --nocapture --test-threads=1` in probe/: p01, p02 and p20 fail because h2 panics.
- `cargo test --test q <name> -- --nocapture` in probe/: q03 aborts, q08 panics at 24,600 names, q10 panics.
- `FUZZ_SECS=45 cargo test --test fuzz -- --nocapture` in probe/: 446,076 cases, debug assertions only.
- `FUZZ_SECS=90 cargo test --release --test fuzz -- --nocapture` in probe/: 4,624,952 cases, no panics.
- Not run: Miri, a build at rust-version 1.63 (toolchain not available), feature powerset.
