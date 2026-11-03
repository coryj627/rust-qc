# Audit of h2 0.4.12

Verdict: do not expose this release to untrusted peers without mitigation. A remote peer can panic a default server with one header block, and a server can panic a default client with four ordinary frames. Padded DATA and dropped request bodies break connection flow control, some malformed messages are accepted, and some recoverable ones kill the whole connection. The existing test suite passes and does not cover these paths.

Probes are in the workspace at `probe/tests/p.rs`, `probe/tests/q.rs` and `probe/tests/fuzz.rs`. They drive the public API through the `tests/h2-support` mock transport. A name in brackets, such as [p20], is the test that shows the result. "Read only" means I did not run it.

## High

**Too many distinct header names panics the connection task**
src/frame/headers.rs:914 (HeaderBlock::load, `self.fields.append`).
`HeaderMap::append` panics when the map cannot grow. A block with 24,600 distinct names (about 221 KB in HEADERS plus CONTINUATION) panicked in http-1.5.0 `header/map.rs:1433` with "size overflows MAX_SIZE: MaxSizeReached"; 24,000 names were accepted. The 16 MiB default limit does not stop it, and neither did `max_header_list_size(1 << 20)`. No authentication or prior state is needed. From reading, the same line handles client responses, trailers and PUSH_PROMISE.
ran [p20] on a default server (40,000 names panicked, 20,000 passed) and [q08] with the 1 MiB limit.

**Push promise, then a 1xx, then a final response panics the client**
src/proto/streams/counts.rs:111, reached from src/proto/streams/recv.rs:171, because src/proto/streams/state.rs:157-165 returns `initial = true` each time while the stream stays ReservedRemote.
the legal sequence HEADERS 200, PUSH_PROMISE, HEADERS 100 and HEADERS 200 on the promised stream counts the stream twice and hits `assert!(!stream.is_counted)`. Push is on by default for clients, so any server can do this. From reading, the panic happens with the streams mutex held, so it also poisons it.
ran [p01] on a default client.

**More pushed responses than max_concurrent_streams panics the client**
src/proto/streams/counts.rs:110, reached from src/proto/streams/recv.rs:171.
`Recv::open` checks the limit at PUSH_PROMISE, but a reservation does not count. With `max_concurrent_streams(1)`, two promises pass, then HEADERS on both hits `assert!(self.can_inc_num_recv_streams())`.
ran [p02].

## Medium

**RecvStream drop panics on a poisoned lock, and aborts the process if unwinding**
src/proto/streams/streams.rs:1415 (`clear_recv_buffer`, `lock().unwrap()`), called from src/share.rs:487.
after any panic under the streams lock, dropping a `RecvStream` panics on `PoisonError`. If that drop happens during unwinding, the process aborts, taking every connection with it. `drop_stream_ref` at streams.rs:1477 guards against this with `thread::panicking()`; this function does not. A `RecvStream` owned by another task gets an ordinary panic instead (from reading). The abort needs the stream to be dropped on the unwinding thread, for example when a handler and the connection share one task.
ran [q03]: "panic in a destructor during cleanup", SIGABRT.

**Dropping pushed responses past the reset limit panics**
src/proto/streams/prioritize.rs:828 (`find_mut(&pp.promised_id()).unwrap()`).
60 `push_request` calls with each `SendPushedResponse` dropped panics on the `unwrap`. 20 did not. Past the default limit of 50 stored resets, the promised stream is released while its PUSH_PROMISE is still queued.
ran [q03] and [p19_push_dropped_many].

**Padded DATA is not counted in flow control**
src/proto/streams/recv.rs:580 and src/proto/streams/streams.rs:584 (`frame.payload().len()`), after src/frame/util.rs:19 strips padding.
RFC 9113 section 6.1 counts the whole payload, including padding. A peer sent 300 frames of 1 data byte and 254 padding bytes, 76,800 wire bytes on a 65,535 window. h2 raised no FLOW_CONTROL_ERROR and, after the app released all 300 data bytes, sent no WINDOW_UPDATE. A conforming padded sender therefore runs out of window and hangs (inferred from that accounting). A hostile sender is not limited by the window.
ran [p03]. The existing `data_padding` test checks only payload length.

**Dropping a request body keeps buffered bytes out of the connection window**
src/proto/streams/streams.rs:1414 (`clear_recv_buffer`).
buffered data is discarded, but `in_flight_recv_data` is not released until the stream closes and all references go. With 49,152 bytes buffered, the body dropped and the response stream kept open, no WINDOW_UPDATE was sent. A handler that ignores the body and streams its response (long polling, server-sent events) starves other streams; about 64 KB of unread bodies would stall the connection (inferred).
ran [q01].

**DATA above the GOAWAY limit never returns connection window**
src/proto/streams/streams.rs:551-559 (`recv_data`).
RFC 9113 section 6.8 requires this DATA to count toward the connection window. This branch returns `Ok(())` without `ignore_data`, unlike the branch below it (line 570). After a final GOAWAY with last stream 1, 48 KB raced on stream 3 got no WINDOW_UPDATE, so an upload on stream 1 can stall during graceful shutdown.
ran [q04].

**A malformed block followed by CONTINUATION kills the connection**
src/codec/framed_read.rs:173-177 and 345-349.
`MalformedMessage` returns a stream reset without storing `Partial`, so the next CONTINUATION is "unexpected" and becomes GOAWAY PROTOCOL_ERROR. The rest of the block is never HPACK decoded, despite the comment at src/frame/headers.rs:881 saying it must be.
ran [p04] (`connection: close` in a split block gave GOAWAY, not RST_STREAM).

**Malformed state is forgotten when a block is split across frames**
src/frame/headers.rs:854 and 930-938.
`malformed` is a local variable, and a `NeedMore` error returns before it is checked. A request with `connection: close` or a duplicate `:path`, plus a header cut at the frame boundary, reached the app as valid with the offending field silently removed. Whether a message is rejected depends on where the sender cut the frame.
ran [p05] and [p05b].

**One invalid header field closes the whole connection**
src/codec/framed_read.rs:178-181, with errors from src/hpack/header.rs:72-100.
RFC 9113 section 8.2.1 makes these malformed messages, which are stream errors. A newline in a value, an uppercase name, `:foo` or a bad `:method` on stream 3 sent GOAWAY PROTOCOL_ERROR, ending stream 1 too.
ran [p12a] to [p12d].

**A response without :status is reported as 200 OK**
src/client.rs:1676.
the builder default applies when `:status` is missing. HEADERS with only `x: y` gave the client `Ok(200)`, so a broken upstream looks like success.
ran [p06].

**Requests missing :path or :authority are accepted**
src/server.rs:1558-1567 and 1593-1609.
`GET` with `:scheme` and no `:path` or `:authority` reached the app with an empty URI. So did `CONNECT` with no `:authority`. RFC 9113 sections 8.3.1 and 8.5 require these fields.
ran [p07a] and [p07d].

**Oversized trailers are silently truncated**
src/proto/streams/recv.rs:348-370.
`recv_trailers` does not check `is_over_size`. With a 300-byte limit, `grpc-keep`, `x-big1`, `x-big2`, `grpc-status: 13` reached the app as `Ok(Some({grpc-keep, x-big1}))`. Losing `grpc-status` changes what a caller concludes about an RPC, with no signal.
ran [p08].

**A 304 with content-length and an empty final DATA frame is reset**
src/proto/streams/recv.rs:640-648.
the 204 and 304 exemption at recv.rs:195 covers only END_STREAM on HEADERS. `304, content-length: 100` then an empty DATA with END_STREAM gave a 304 whose body read failed with `Reset(PROTOCOL_ERROR)`. RFC 9110 section 8.6 allows that header on a 304.
ran [p10].

**Empty and conflicting content-length values reach the app**
src/proto/streams/recv.rs:178 (first value only) and src/frame/headers.rs:326-344 (empty parses as 0).
`content-length: 0` plus `content-length: 100` was accepted, and so was an empty value. From reading, both values stay in the `HeaderMap`; the probe printed names only. A proxy forwarding to HTTP/1.1 could send conflicting framing, which is a request smuggling setup.
ran [p11a] and [p11b].

**Credentials in the URI are sent in :authority**
src/frame/headers.rs:597.
`https://user:s3cret@x.com/` is sent as `:authority: user:s3cret@x.com`. RFC 9113 section 8.3.1 forbids userinfo there.
ran [p14].

**Dropping a pushed response never resets the promised stream**
src/proto/streams/prioritize.rs:832 and src/proto/streams/send.rs:243-251.
the stream is not send-ready during `is_pending_push`, and after the PUSH_PROMISE it is rescheduled only if `pending_send` is non-empty. The peer got no RST_STREAM in 700 ms and, from reading, never would, so a client awaiting that push hangs.
ran [q02].

**Unbounded memory from push reservations**
src/proto/streams/streams.rs:780.
reserved streams count toward no limit, as the code's own TODO says. A server can promise every even stream ID, and each stores a `Stream` and a request until the parent closes.
read only.

**The HPACK encoder table follows the peer's setting with no cap**
src/proto/settings.rs:140, src/codec/mod.rs:94, src/hpack/encoder.rs:30.
any SETTINGS_HEADER_TABLE_SIZE up to 4 GiB is adopted, though RFC 7541 section 4.2 lets an encoder use less. A client can grow per-connection memory if responses carry unique indexed headers.
read only.

## Low

**PUSH_PROMISE on a locally reset stream trips a debug assertion**
src/proto/connection.rs:446, through src/proto/streams/streams.rs:767.
`ensure_recv_open` returns the stored `Reset(.., User)` as a new stream error, and `debug_assert_eq!(initiator, Library)` panics. This is remote-triggered but debug builds only. A remote reset instead of a user reset gave GOAWAY [q05, q09]; I did not work out why that path differs.
ran [q10]; first found by the fuzz harness.

**Protocol error codes are wrong for several frame errors**
src/codec/framed_read.rs:178-181 and 201-256, src/frame/go_away.rs:43.
wrong-size PING, SETTINGS, RST_STREAM, WINDOW_UPDATE and PRIORITY give PROTOCOL_ERROR, not FRAME_SIZE_ERROR. An oversized initial window should be FLOW_CONTROL_ERROR, and HPACK failures should be COMPRESSION_ERROR. A zero WINDOW_UPDATE on a stream closes the connection, and GOAWAY on a non-zero stream ID is accepted.
ran [p17c], [p17d], [p18].

**PUSH_PROMISE with an empty first fragment is rejected**
src/frame/headers.rs:435.
a promised ID with the block carried in CONTINUATION is legal but gets GOAWAY.
ran [p13].

**A header list exactly at the limit is rejected**
src/frame/headers.rs:869 and 911.
exactly 205 bytes under a limit of 205 got an immediate END_STREAM response rather than reaching the app (from the code, the 431); a limit of 206 was accepted.
ran [p15].

**Pseudo-headers in trailers are silently dropped**
src/proto/streams/recv.rs:361.
`:path` in trailers disappeared with no error; RFC 9113 section 8.1 makes that malformed.
ran [p09].

**HPACK size update is accepted mid-block after a frame boundary**
src/hpack/decoder.rs:186.
`can_resize` restarts on each `decode` call, so an update rejected within one frame is accepted at the start of a CONTINUATION.
ran [p17a] and [p17b].

**send_request emits malformed requests instead of returning an error**
src/client.rs:1626-1638.
`GET example.com` sends no `:scheme`, and `CONNECT /x` sends `:scheme http` with no `:authority`. The failure surfaces later as a peer reset.
ran [q07a] and [q07b].

**Header size recomputation omits :protocol**
src/frame/headers.rs:975-980.
`set_pseudo!` counts `:protocol`, and `calculate_header_list_size` does not, so the total drops when a block continues.
read only.

**Hygiene from the lint and rustdoc passes**
mainly src/proto/streams/ and src/frame/.

- clippy with `-D warnings` fails on 8 errors, including src/hpack/huffman/table.rs:265. I treat these as toolchain drift but did not run the project's CI toolchain.
- rustdoc with private items fails on two bare URLs (src/hpack/decoder.rs:508, src/frame/headers.rs:959).
- Advisory counts in src/, some possibly in test modules: 110 asserts without messages, about 85 `unwrap` calls, 24 missing `# Errors`, 7 missing `# Panics`, 10 wildcard imports.
- Box-drawing diagrams appear in src/hpack/decoder.rs and src/proto/streams/state.rs, and TODOs have no issue references.
- The one `unsafe` block (src/hpack/header.rs:283) looks sound from reading its constructors.

## Low

**Regression coverage**
Add these probes as tests. The fuzz harness is weak evidence of absence: it did not rediscover the High.

**Debug-only assertions on drop**
`Counts::drop` (src/proto/streams/counts.rs:282) and `Store::drop` (src/proto/streams/store.rs:231) fired in debug fuzz runs. I did not investigate them, so they may be harness shutdown artifacts rather than leaks.
