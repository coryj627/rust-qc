# Audit: h2 0.4.12

## Summary

The crate builds and its test suite passes, but a remote peer can panic it, and on default settings a single header block is enough. Several ordinary API sequences kill the connection or stall it permanently. Most of the serious defects are in push handling, flow-control accounting and validation of malformed messages.

All probes use the public API over the crate's mock transport or `tokio::io::duplex`. Probe IDs below refer to `probes/tests/a.rs`, `b.rs` and `c.rs`, and the fuzzer is `probes/src/bin/fuzz.rs`. `crate/` was not changed.

## High

**Too many header fields panic the connection task**
src/frame/headers.rs:914 (`self.fields.append`).
`HeaderMap::append` panics at its capacity limit, but the default header list limit is 16 MiB. A default server panicked with `size overflows MAX_SIZE: MaxSizeReached` at 24,577 distinct field names. 24,576 was accepted. The 40,000-field test block was 315 KB in 20 frames.
ran p26 and p26b against a server. The client uses the same decode path, but I only read that side.

**Two 1xx responses on a pushed stream panic the client**
src/proto/streams/state.rs:157-165, recv.rs:164-171, counts.rs:111.
an informational HEADERS frame leaves the stream in `ReservedRemote` and still reports `initial = true`. The second one fails `assert!(!stream.is_counted)` while the lock is held. That poisons the mutex, so user handles panic too. This happens with the default client configuration.
ran p25. PUSH_PROMISE, then 100, then 103 gave the panic at counts.rs:111, then `PoisonError` at streams.rs:1330.

**Pushed responses can exceed the client's concurrency limit and panic**
src/proto/streams/counts.rs:110, recv.rs:171.
the concurrency check runs only when a stream is first opened. Reserved streams are not counted, so with `max_concurrent_streams(1)`, two PUSH_PROMISEs followed by HEADERS on both trip the assertion.
ran p24, which panicked with `assertion failed: self.can_inc_num_recv_streams()`.

**PUSH_PROMISE flush panics if the pushed stream was already released**
src/proto/streams/prioritize.rs:828, stream.rs:254.
`is_released` ignores `is_pending_push`. If the pushed handle is dropped before the flush and the reset cannot be queued, the stream is removed, and `find_mut(..).unwrap()` then panics.
ran p02b with `max_concurrent_reset_streams(0)`, which panicked at prioritize.rs:828. Reaching this through an exhausted default reset cap (50) is inferred from the code, not tested.

**RecvStream drop can abort the process after a poisoned lock**
src/proto/streams/streams.rs:1415 (`clear_recv_buffer`), share.rs:487.
`drop_stream_ref` handles poisoning, but this `lock().unwrap()` runs inside `Drop` with no such check. During unwinding, that is a panic in a destructor, which aborts the process. Together with the 1xx panic above, a remote server can take down the whole process.
the fuzzer aborted with `panic in a destructor during cleanup`, after counts.rs:111 and then streams.rs:1415, on seed 5287897698250948783.

## Medium

**Explicit reset before flush puts RST_STREAM on an idle stream**
src/proto/streams/send.rs:226.
`send_reset` clears the queue, including the unsent HEADERS, and then queues RST_STREAM. The peer sees a reset for a stream that was never opened and closes the connection, taking sibling requests with it.
ran p35a, where the wire shows only RST_STREAM(1). In p35b a real h2 server answered with GOAWAY PROTOCOL_ERROR.

**Padding is not counted in flow control**
src/proto/streams/recv.rs:580, streams.rs:584.
accounting uses the payload length after padding is stripped. Sender and receiver windows diverge permanently, so padded senders eventually stall, and overruns go undetected.
ran p01. 300 frames totalling 76,800 flow-controlled octets were accepted against a 65,535 window, and no WINDOW_UPDATE was sent.

**Implicit reset leaks connection send capacity**
src/proto/streams/send.rs:250, prioritize.rs:849-853.
a scheduled reset keeps `requested_send_capacity`, so a later window update assigns capacity to the dying stream, and it is never reclaimed. The debug assertion at prioritize.rs:448 also fired under the fuzzer.
ran p04. After the leak, the next stream delivered 5,535 of 20,000 bytes, then stalled.

**reserve_capacity on a pending-open stream can deadlock the connection**
src/proto/streams/prioritize.rs:287 (`send_data` has the guard at lines 193-195, this path does not).
a stream that cannot open yet takes all connection capacity, and the open stream it is waiting on then cannot send.
ran p34 with a limit of 1. The pending stream held 65,535 and the server received 0 of 1,000 bytes.

**Dropping RecvStream does not release its connection window**
src/proto/streams/streams.rs:1414-1421, recv.rs:431.
buffered DATA is discarded, but its capacity is released only when all stream references go. A long response can therefore starve other streams.
ran p30. 64,000 buffered bytes were dropped while the SendStream stayed alive, and no connection WINDOW_UPDATE was sent.

**One invalid header field kills the whole connection**
src/hpack/header.rs:65-103, src/codec/framed_read.rs:178-181.
a newline in a value, an uppercase name, an unknown pseudo-header or a bad method all become HPACK decode errors, so GOAWAY is sent instead of a stream reset. Valid sibling streams die, which is serious for proxies. RFC 9113 treats these as malformed messages.
ran p08a, c, d, e on the server and p08b on the client. All gave GOAWAY. The control case (`connection` header, p08f) gave RST_STREAM and stream 3 was served.

**Malformed marker is lost across CONTINUATION**
src/frame/headers.rs:854, 930-938.
`malformed` is a local variable, and a NeedMore result returns before it is checked. If the frame ends mid-literal, requests with `connection`, `transfer-encoding`, a duplicate `:method` or a pseudo-header after a field are accepted, with the offending field silently dropped. Validation depends on where the frame was split.
ran p09a, c, d, e. All were accepted.

**Malformed block without END_HEADERS turns a stream error into a connection error**
src/codec/framed_read.rs:173-177.
returning a stream reset discards the partial state, so the following CONTINUATION is "unexpected" and triggers GOAWAY.
ran p09b.

**Response without :status is reported as 200 OK**
src/client.rs:1676.
the builder default is used when `:status` is missing, and request pseudo-headers in a response are ignored. Both are malformed messages, and the first hands callers a false success.
ran p06 (status 200 with `foo: bar`) and p06b (`:path` accepted in a response).

**Server accepts requests missing :path or :authority**
src/server.rs:1593 (the check covers extended CONNECT only), src/server.rs:1558.
GET without `:path` or `:authority` is delivered with an empty URI, as is CONNECT without `:authority`. `GET *` is also accepted, and its URI displays as `https://a.test*`.
ran p07a, c, h.

**Over-limit trailers are silently truncated**
src/proto/streams/recv.rs:348-370.
`recv_trailers` never checks `is_over_size`. Callers receive a partial trailer map with no error, so a dropped `grpc-status` looks like absence.
ran p15, which returned only `keep`, with no error and no reset.

**RST_STREAM(NO_ERROR) after a complete response turns the body into an error**
src/proto/streams/state.rs:277-288, 433-436.
the reset overwrites a state that had already seen END_STREAM. RFC 9113 §8.1 says clients must not discard the response in this case. This is the normal early-response pattern while a request body is still uploading.
ran p32. `data()` returned "hi", then `Err(Reset(NO_ERROR))`, and `trailers()` also returned an error.

**Content-length on a 1xx response applies to the final response**
src/proto/streams/recv.rs:174-187.
the value is recorded before the informational check, so a valid final response is reset.
ran p05a, where a 10-byte body gave PROTOCOL_ERROR.

**304 with content-length and a separate END_STREAM is reset**
src/proto/streams/recv.rs:187, 641.
the 204/304 exemption covers only END_STREAM on the HEADERS frame.
ran p05b, which gave PROTOCOL_ERROR.

**Promised HEAD requests are not treated as HEAD**
src/proto/streams/streams.rs:804-810, recv.rs:718.
`ContentLength::Head` is set only for local requests, so a pushed HEAD response with a content-length is treated as malformed.
ran p05c, which gave a reset.

**Conflicting content-length values are not detected**
src/proto/streams/recv.rs:178.
only the first value is checked, and both are passed to the application. Different handling further down the chain is a request-smuggling risk.
ran p12a. `1` and `100` were accepted with a 1-byte body, and both headers were delivered.

**PUSH_PROMISE on a stream the client reset kills the connection**
src/proto/streams/streams.rs:767, 774-777, state.rs:437-439.
RFC 9113 §6.6 requires handling these frames because they can cross with the reset in flight. Instead the result is GOAWAY with reason CANCEL (implicit reset pending) or PROTOCOL_ERROR (stream forgotten). With an explicit reset, debug builds panic at connection.rs:446.
ran p22, p23b and p23.

**Trailers on a forgotten stream bring down the server connection**
src/proto/streams/streams.rs:448-458.
the "forgotten stream" check is client-only, although DATA in the same situation gets a stream-level STREAM_CLOSED. The default forget time is 1 second, so a client still uploading can hit this.
ran p11 with `max_concurrent_reset_streams(0)`. DATA gave STREAM_CLOSED and trailers gave GOAWAY. The default-timing path is inferred from the code.

**Pushed stream is never reset when its handle is dropped before flush**
src/proto/streams/prioritize.rs:832.
only `pending_send` is checked, so the scheduled reset is never sent. The client waits on the promised response indefinitely.
ran p02a. The wire shows PUSH_PROMISE and HEADERS(1), with no RST_STREAM(2).

**Parent reset before flush orphans the pushed stream silently**
src/proto/streams/prioritize.rs:673.
`clear_queue` drops the PUSH_PROMISE but `is_pending_push` stays set. `send_response` and `send_data` return Ok, nothing is sent, and `poll_reset` hangs.
ran p36.

**Failed push leaks a stream slot, and push after the final response is allowed**
src/proto/streams/streams.rs:1209 (the `?` skips cleanup), streams.rs:1177.
rejected pushes leave a slot behind for the life of the connection. Separately, there is no parent state check, so PUSH_PROMISE can go out after END_STREAM.
ran p14, where wired streams went from 1 to 101 after 100 failed pushes. Ran p13, where the wire shows PUSH_PROMISE after END_STREAM. How a peer reacts to that was not tested.

**Remote resets of pushes accumulate until the client sends GOAWAY**
src/proto/streams/recv.rs:810-823 (only `next_incoming`, streams.rs:145, decrements).
the counter rises for pending pushes, but `poll_pushed` never lowers it. A long-lived connection eventually fails even with nothing outstanding.
ran p10. GOAWAY ENHANCE_YOUR_CALM `too_many_resets` arrived on iteration 21, with one promise outstanding at a time.

**trailers() hangs while DATA is unread**
src/proto/streams/recv.rs:1146-1151.
it returns Pending without registering a waker, so a caller awaiting trailers without draining the body is never woken, even after END_STREAM.
ran p33, which timed out with trailers already sent.

**Literal header names are re-decoded on every CONTINUATION frame**
src/hpack/decoder.rs:287-291, src/codec/framed_read.rs:339-342.
the Huffman name is decoded again each time the value is incomplete, so work grows with name size times frame count.
ran p31 in release mode with a 1M-character name. 1 frame took 7 ms, 50 took 147 ms and 100 took 342 ms. Extrapolating to the 16 MiB limit gives roughly 20 seconds of task-blocking CPU per header block. That figure is an estimate, not a measurement.

**Client puts malformed requests on the wire**
src/client.rs:1637 (`TODO: Error`), src/client.rs:1634.
an authority-form GET omits `:scheme`. A relative CONNECT gets `:scheme` with no `:authority`. `Protocol` on a GET sends `:protocol`. All return Ok from `send_request`.
ran p27a, p27b and p37.

**reserve_capacity silently truncates sizes**
src/share.rs:285-287.
`capacity as WindowSize` wraps, so `1 << 32` becomes 0 and the caller waits forever. A 4 GiB upload that reserves its body length would hit this.
ran p28, where `poll_capacity` stayed pending and `capacity()` returned 0.

## Low

**Wrong error codes**
src/codec/framed_read.rs:178-181, 202-276.
HPACK failures give PROTOCOL_ERROR rather than COMPRESSION_ERROR. Bad frame lengths, a too-large INITIAL_WINDOW_SIZE and a zero WINDOW_UPDATE on a stream all give connection PROTOCOL_ERROR instead of their specified codes or scope.
ran p16 and p17a-g.

**Header decoding is too lenient at the edges**
src/hpack/decoder.rs:186, src/hpack/header.rs:66, src/frame/headers.rs:326-344.
a table size update is accepted at the start of a CONTINUATION frame, an empty literal name can be swallowed as NeedMore, and an empty content-length parses as 0.
ran p17l, p17p and p12b.

**Header list limit is off by one**
src/frame/headers.rs:869, 911.
a list exactly equal to the limit gets an automatic response, while limit + 1 is accepted. I did not inspect that response's status, but it matches the 431 path in the code.
ran p39 at limits 124 and 125.

**Send-side header checks are incomplete**
src/proto/streams/send.rs:90, 269-292.
only the first `te` value is checked, and trailers are not checked at all.
ran p27d and p28. `te: gzip` went on the wire, and `send_trailers` accepted `connection: close`.

**Lint, documentation and hygiene debt**
crate-wide.
clippy reports 8 errors and rustdoc fails on a bare URL, both on Rust 1.99, while the project declares `rust-version` 1.63. I treat these as toolchain drift. The scan also found ASCII diagrams (decoder.rs, state.rs), TODOs without owners, and asserts without `# Panics` docs such as `set_target_window_size`. Missing docs (103 warnings) and missing Debug (6) include the test support crate.
