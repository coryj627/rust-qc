# Audit report: h2 0.4.12 (`crate/src/`)

## How this was established

I read all of `src/` except the HPACK static tables. I then wrote a throwaway crate, `probe/`, in the workspace, which drives h2 through the `h2-support` mock transport with raw frames. `crate/` was not modified. Nothing in the repository or its output addressed me.

- **Reproducers:** `probe/tests/p.rs` (tests `tNN`) and `probe/tests/alloc.rs` (tests `a1`, `a2`, `e1`, `e2`, `f1`, using a counting global allocator).
- **To run one:** `./run sh -c 'cd /work/probe && CARGO_TARGET_DIR=/target/probe cargo test --offline --test p tNN -- --nocapture --test-threads=1'`. Use `--test alloc` for the allocator tests. Run them one at a time, because t06 appears to hang and t22 aborts the process.
- **Baseline:** `cargo test --locked -p h2 --lib` gives 419 passed. I did not run `h2-tests`.
- **Builds:** all probes used debug builds. Panics from `assert!` or `unwrap` also apply to release builds. The one that relies on `debug_assert!` says so.

"Ran" means I observed the stated result. "Read only" means I did not execute it.

## High severity

**1. A peer can panic the connection task with many distinct header names, under default settings**
`src/frame/headers.rs:914` (`self.fields.append`)

`HeaderMap::append` panics once the map exceeds its internal size limit. The default `max_header_list_size` is 16 MB (`src/codec/framed_read.rs:22`), far above what a `HeaderMap` can hold, and nothing else limits the count.

- **What a caller sees:** the task driving the `Connection` panics. By the same code path this applies to a client receiving a response; I only ran the server case.
- **Ran t05:** 40,000 distinct names with empty values, 360 KB across 22 frames. Result: `panicked at http-1.5.0/src/header/map.rs:1433: size overflows MAX_SIZE: MaxSizeReached`. The server task's `JoinError` reported `is_panic()`.
- **Mitigation:** a small `max_header_list_size` avoids this.

**2. A malicious server panics the client with a 1xx on a pushed stream**
`src/proto/streams/state.rs:157-168`, `src/proto/streams/recv.rs:164-171`, `src/proto/streams/counts.rs:111`

`recv_open` returns `initial = true` for every HEADERS on a `ReservedRemote` stream. An informational response leaves the state at `ReservedRemote`, so the next HEADERS is also "initial". `inc_num_recv_streams` then hits `assert!(!stream.is_counted)` while the streams mutex is held, which poisons it (see finding 5). Push is enabled by default on clients.

- **Ran t06, t06b:** PUSH_PROMISE, HEADERS 103, HEADERS 200 on the promised stream. Result: `panicked at counts.rs:111: assertion failed: !stream.is_counted`.
- **Follow-on effect:** in t06b, the user's `ResponseFuture` panicked at `streams.rs:1330` on the poisoned lock when its timer re-polled it. In t06, which has no timer, output stopped at this test and the run went on to the 300 s limit, so I infer the future was never woken.

**3. Pushed streams bypass the local concurrency limit and hit a panic**
`src/proto/streams/recv.rs:171`, `src/proto/streams/counts.rs:110`

`can_inc_num_recv_streams` is only checked in `Recv::open`, when the promise is reserved, and reserved streams are not counted. When HEADERS arrive on promised streams, `inc_num_recv_streams` asserts the limit.

- **What a caller sees:** a client built with `max_concurrent_streams(N)` panics if the server promises N+1 streams and starts them.
- **Ran t07 (N=1):** `panicked at counts.rs:110: assertion failed: self.can_inc_num_recv_streams()`, followed by `StreamRef::drop; mutex poisoned` in the user's task.

**4. A GOAWAY cancels streams it says nothing about**
`src/proto/streams/streams.rs:731`

`recv_go_away` errors every stream with `id > last_stream_id`, whichever side initiated it. RFC 9113 §6.8 defines the last stream ID in terms of streams initiated by the receiver of the GOAWAY. A client's own odd-numbered requests are therefore not covered by the GOAWAY it sends.

- **Ran t43:** the client sends a request, then GOAWAY(last=0, NO_ERROR), meaning "no more pushes". The server's `send_response` fails with `user error: unexpected frame type` and the server closes with GOAWAY. The response is lost.
- **Ran t43b:** the server sends GOAWAY(last=1) while its pushed stream 2 is active. The client's pushed body fails with `GoAway(NO_ERROR, Remote)`.
- **Related, read only:** `recv.max_stream_id` has the same problem at `streams.rs:431, 613`. After we send GOAWAY(N), RST_STREAM for our own pushed stream above N is ignored.

**5. An internal panic spreads to user tasks and can abort the process**
`src/proto/streams/streams.rs:1415` (and every `lock().unwrap()`, including 1330), `src/proto/streams/streams.rs:1481`

After the mutex is poisoned, user API calls panic instead of returning errors. `RecvStream::drop` calls `clear_recv_buffer`, which does `lock().unwrap()`. If a `RecvStream` is dropped while its task is already unwinding, that is a panic in a destructor during cleanup, which aborts the whole process. `drop_stream_ref` guards against this case; `clear_recv_buffer` does not.

- **Ran t22:** `panic in a destructor during cleanup` / `thread caused non-unwinding panic. aborting.`, triggered through finding 6 with the connection driven in the same task as a held `Request<RecvStream>`. Findings 2 and 3 poison the same mutex, so I expect the same escalation, but I did not run those combinations.
- **Ran t06b, t07:** panics in user tasks.

**6. `Prioritize::pop_frame` panics when a pushed stream is dropped before its PUSH_PROMISE is written**
`src/proto/streams/prioritize.rs:828` (`find_mut(&pp.promised_id()).unwrap()`)

`is_released` (`stream.rs:254`) ignores `is_pending_push`. Dropping `SendPushedResponse` closes the child stream, which is removed if no reset-expiry slot is available or once the expiry passes. The parent's queued PUSH_PROMISE then looks up a stream that no longer exists.

- **Conditions (read only, beyond the run below):** reset-expiry slots exhausted (default 50), or the socket stalled for longer than `reset_stream_duration`.
- **Ran t22:** with `max_concurrent_reset_streams(0)` and `drop(resp.push_request(..).unwrap())`, it panicked at `prioritize.rs:828`.

**7. Cancelling a request before it is flushed sends RST_STREAM on an idle stream, killing the connection**
`src/proto/streams/send.rs:226-232`

`Send::send_reset` clears the queue, including the unsent HEADERS, and then queues RST_STREAM. The peer never saw the stream open, and RFC 9113 §6.4 makes this a connection error. It happens when `send_reset` is called right after `send_request`, or on a stream still waiting in `pending_open`.

- **What a caller sees:** one cancelled request takes down every other request on the connection.
- **Ran t42 (real h2 client against h2 server over `tokio::io::duplex`):** the server reports `GoAway(PROTOCOL_ERROR, Library)` and the client connection gets `GoAway(PROTOCOL_ERROR, Remote)`. An unrelated in-flight request fails with BrokenPipe.
- **Ran t42b:** a stream waiting in `pending_open` puts `RST_STREAM(3)` on the wire with no HEADERS(3).

**8. Padding is left out of inbound flow control, so senders stall**
`src/proto/streams/recv.rs:580`, `src/proto/streams/streams.rs:564`

Accounting uses `frame.payload().len()`, which excludes the pad-length octet and the padding. RFC 9113 §6.1 counts the whole frame payload. Padding bytes are never returned by WINDOW_UPDATE, so a conforming sender's connection and stream windows shrink permanently. Inbound limits are not enforced against padded bytes either.

- **Ran t01:** 300 zero-payload frames with 255 bytes of padding, plus two 16,201-byte frames carrying 16,000 data bytes each. That is 109,202 flow-controlled bytes on a 65,535 window. The server accepted all of it without FLOW_CONTROL_ERROR, and after full `release_capacity` it sent only `WindowUpdate { stream 0, 32000 }`.

**9. Payload-free DATA frames grow memory without limit**
`src/proto/streams/recv.rs:675-678`

Empty DATA frames, and padded frames as in finding 8, consume no flow control but each pushes an `Event::Data` onto `pending_recv`. If the application is not reading the body right now, a client grows server memory without limit.

- **Ran f1:** 200,000 nine-byte empty DATA frames grew the server's live heap by 64.7 MB (323 B per frame) on an idle stream.

**10. HPACK encoder table size is uncapped and set by the peer**
`src/proto/settings.rs:139-141`, `src/hpack/encoder.rs:30`, `src/hpack/table.rs:350`

The encoder adopts whatever SETTINGS_HEADER_TABLE_SIZE the peer sends, up to 4 GiB. Every new header value the local side sends then stays in the table for the life of the connection.

- **Ran e1, e2:** with a peer table size of `u32::MAX`, sending 3,000 responses with a unique 1,000-byte header left the server's live heap growing by 1,215 B per response. With 4,096 the growth was 0 B per response.

## Medium severity

**11. `reserve_capacity` on a pending-open stream starves open streams**
`src/proto/streams/prioritize.rs:287`

`send_data` guards `try_assign_capacity` with `!is_pending_open`; its comment explains that this avoids starving the connection. `reserve_capacity` has no such guard.

- **Ran t13:** with `max_concurrent_streams=1`, pending stream 3 calls `reserve_capacity(65535)`. Its capacity becomes 65,535 and open stream 1 gets 0. Stream 1's DATA did not leave the client during the test.
- **Inferred:** if stream 1 has to finish before stream 3 can open, the connection deadlocks.

**12. Dropping `RecvStream` while keeping the send half withholds connection window**
`src/proto/streams/streams.rs:1414-1421`, `src/proto/streams/recv.rs:431-448`, `src/share.rs:480`

`clear_recv_buffer` discards the unread data but leaves `in_flight_recv_data` unreleased until every reference is gone. Nobody can release it in the meantime.

- **What a caller sees:** a handler that drops the request body and streams a long response starves all other streams on the connection for that time.
- **Ran t15:** 49,152 unread bytes, RecvStream dropped, SendStream held. No WINDOW_UPDATE appeared. Control t15c, where all references were dropped: `WindowUpdate { stream 0, 49152 }`.

**13. Connection window leaks for pushed data when the parent is dropped**
`src/proto/streams/streams.rs:1518-1523`

Promises still unclaimed are cancelled with `maybe_cancel` only. `release_closed_capacity` runs only for the stream whose ref count reached zero through a handle, so a promise's `in_flight_recv_data` is never returned.

- **Ran t15b:** 49,152 bytes on promised stream 2, parent dropped. RST frames were sent but no WINDOW_UPDATE appeared during the test (about 350 ms). That the leak is permanent comes from reading the code, since I found no later release path. By that reasoning, a server can shrink a client's connection window to zero.

**14. Unread events leak in the receive buffer when a stream is released**
`src/proto/streams/recv.rs:434-436`

`release_closed_capacity` returns early when `in_flight_recv_data == 0`, without clearing `pending_recv`. Queued `Event::Headers` stay in the per-connection `Buffer` slab until the connection ends. This is the typical request-timeout race: the response arrived, then the unpolled `ResponseFuture` was dropped.

- **Ran a1, a2:** 2,000 requests with 4 KB response headers and dropped futures leaked 5,000 B per request, while `num_wired_streams` was 0. The polled control leaked 0 B per request.

**15. Streams leak when evicted from `pending_capacity`**
`src/proto/streams/prioritize.rs:402-404`

The `continue` skips `counts.transition`. A stream that is closed, has no handles and was only retained by `is_pending_send_capacity` is never released until the connection ends.

- **Ran t14:** streams 3 and 5 queued for capacity, RST by the peer, handles dropped, then a connection WINDOW_UPDATE. `num_wired_streams` stays at 3 with `num_active_streams` at 1.

**16. `num_local_reset_streams` leaks if a reset stream still has queued data at expiry**
`src/proto/streams/recv.rs:963`, `src/proto/streams/counts.rs:224-228`

`transition_after(stream, true)` only decrements when `stream.is_closed()`. A stream with queued data is not closed, but `reset_at` has been cleared, so later transitions pass `false` and the count never drops. Once the count reaches `max_concurrent_reset_streams`, resets are forgotten immediately and stray frames draw RST_STREAM(STREAM_CLOSED).

- **Ran t35:** with the limit at 1, a stream dropped with data blocked on flow control expired, then a later reset stream 3 received `Reset { 3, STREAM_CLOSED }` for a late DATA frame. Control t35b ignored the late frame as designed.
- **Read only:** those resets count toward the lifetime `max_local_error_reset_streams` limit, which ends in GOAWAY.

**17. Lost wakeups from a single shared `send_task` slot**
`src/proto/streams/stream.rs:369-371`, with callers at `src/proto/streams/streams.rs:988` and `src/proto/streams/send.rs:330, 353`

`SendRequest::poll_ready`, `SendStream::poll_capacity` and `poll_reset` all overwrite one waker. If the body task and the requester are different tasks, the earlier waiter is never woken.

- **Ran t21:** with `max_concurrent_streams=1`, task B waits in `ready()` and task A polls `poll_reset` on the same pending stream. B returned only when its own 800 ms timer fired (801 ms), and `ready()` was then already satisfied. Control t21b, without A: 150 ms.

**18. Library-initiated stream resets do not wake the send task**
`src/proto/streams/send.rs:171-234`, `src/proto/streams/streams.rs:1600, 1624`, `src/proto/streams/stream.rs:387-391`

`set_reset` and its callers notify recv and push waiters but never `notify_send`. A task parked in `poll_reset` hangs when h2 itself resets the stream, for example on a content-length violation. `poll_capacity` shares the same waker, but I only ran `poll_reset`.

- **Ran t41:** `poll_reset` was still pending after the RST_STREAM went out (timed out at 600 ms). Control t41b, a remote reset: `Ok(CANCEL)` immediately.

**19. `RecvStream::trailers()` hangs while DATA is unread**
`src/proto/streams/recv.rs:1146-1151`

`poll_trailers` returns `Pending` without registering a waker when the next event is DATA.

- **Ran t40:** awaiting `trailers()` with unread DATA and trailers already received was still pending at the 400 ms timeout. Since no waker is stored, a caller that only awaits it should never complete.

**20. A fully received response is turned into an error by a later RST_STREAM or EOF**
`src/proto/streams/state.rs:277-288, 303-316`

`recv_reset` and `recv_eof` overwrite `HalfClosedRemote`, which forgets that END_STREAM was received. RFC 9113 §8.1 says clients must not discard a response because of RST_STREAM(NO_ERROR) sent after an early, complete response.

- **Ran t38:** during an upload, HEADERS, DATA with END_STREAM, then RST_STREAM(NO_ERROR). The client reads the 8 data bytes, then gets `Err(Reset(1, NO_ERROR, Remote))` instead of `None`, `is_end_stream()` is false, and `trailers()` errors.
- **Ran t39:** the same response followed by close gives BrokenPipe.

**21. Graceful shutdown turns into a connection error on a benign race**
`src/proto/streams/streams.rs:685-687`, `src/proto/streams/recv.rs:780-789`

`recv_window_update` has no `id > max_stream_id` check, unlike HEADERS, DATA and RST_STREAM. Ignored HEADERS do not advance `next_stream_id`, so a WINDOW_UPDATE for the ignored stream looks like one for an idle stream.

- **Ran t45:** after `graceful_shutdown` reached GOAWAY(1), the client raced HEADERS(3) and WINDOW_UPDATE(3). The server reported `GoAway(PROTOCOL_ERROR)` and the response to in-flight stream 1 could no longer be sent.

**22. PUSH_PROMISE on a stream we just reset kills the connection**
`src/proto/streams/streams.rs:767`, `src/proto/connection.rs:446`

RFC 9113 §6.6 requires handling a PUSH_PROMISE that crossed our RST_STREAM. Here `ensure_recv_open()?` returns the stream's stored error and the promised stream is never registered, so its HEADERS look illegal.

- **Ran t08:** drop the response, the server's PUSH_PROMISE and HEADERS(2) arrive, and the client sends GOAWAY(PROTOCOL_ERROR).
- **Ran t08b:** with an explicit `send_reset`, the stored `Initiator::User` trips `debug_assert_eq!(initiator, Library)` at `connection.rs:446`, a panic in debug builds only.

**23. Stream errors on one request escalate to connection errors, or are missed**
- **`src/codec/framed_read.rs:154-160`:** HEADERS with a self-dependency returns before HPACK decoding, so the dynamic table desynchronises. **Ran t02:** the next stream, referencing index 62, gets GOAWAY instead of being served.
- **`src/frame/headers.rs:930-938`:** `malformed` is local to one `load` call and is dropped when `decode` returns `NeedMore`. **Ran t03:** `te: gzip` and `connection: close` followed by a literal split across a CONTINUATION boundary. The request was accepted with those fields silently removed, where it should get RST_STREAM(PROTOCOL_ERROR).
- **`src/codec/framed_read.rs:173-177, 345-349`:** a malformed block without END_HEADERS discards the partial state, so the following CONTINUATION is "unexpected". **Ran t04:** GOAWAY instead of a stream error.
- **`src/hpack/header.rs:98-99`, `src/codec/framed_read.rs:178-181`:** an uppercase header name or a bad field value is a connection error, though RFC 9113 §8.2.1 calls it a malformed message, which is a stream error. **Ran t17, t17b:** GOAWAY.
- **`src/hpack/header.rs:65-67`:** an empty header name returns `NeedMore`. **Ran t18:** when not in the final frame it is silently dropped.

**24. Required pseudo-headers are not enforced**
- **`src/server.rs:1607`:** a non-CONNECT request without `:path` and `:authority` is accepted, as is CONNECT without `:authority`. **Ran t28:** `GET uri=` and `CONNECT uri=` (empty URIs) were accepted, as was `GET :path *` (`https://a*`) and a field value with surrounding spaces (` v `).
- **`src/client.rs:1676`:** a response without `:status` becomes 200 OK, and request pseudo-headers in a response are ignored. **Ran t27:** HEADERS with `:path` and `x-a` only gave `Ok(200, {"x-a": "1"})`.

**25. Differing `content-length` values are accepted**
`src/proto/streams/recv.rs:178`, `src/frame/headers.rs:326-344`

Only the first value is enforced, while all values reach the application. That is a request-smuggling risk for proxies forwarding to HTTP/1.1. `parse_u64` also accepts the empty string as 0.

- **Ran t11:** `content-length: 5` and `content-length: 100` with 5 bytes was accepted with both headers exposed. An empty `content-length` was accepted too.

**26. `content-length` is applied from the wrong responses**
`src/proto/streams/recv.rs:187`

- **Ran t12:** a 1xx with `content-length: 0` is remembered, so the following 200's body fails with `Reset(PROTOCOL_ERROR)`.
- **Ran t12b:** a 304 with `content-length: 100` and no END_STREAM on HEADERS, ended by an empty DATA frame, is reset. The 204/304 exception at `recv.rs:195` only covers END_STREAM on HEADERS.
- **Ran t24:** `recv_push_promise` never sets `ContentLength::Head` (only `streams.rs:271` does), so a pushed HEAD response with `content-length: 100` and END_STREAM is reset. Control t24b, a normal HEAD, gives 200.

**27. Trailers are not validated**
`src/proto/streams/streams.rs:532`, `src/proto/streams/recv.rs:348-370`

`is_over_size` is not checked, and pseudo-headers are discarded rather than treated as malformed.

- **Ran t10:** with `max_header_list_size(200)`, 20 trailer fields arrive as a silently truncated set of 5, with no error. `:path` in trailers was ignored and `x-ok` delivered.

**28. PUSH_PROMISE handling leaves counters and resources unbounded**
- **`src/proto/streams/recv.rs:288-317, 810-812`:** resets of unclaimed promises increment the pending-accept reset counter, and only `next_incoming` (server) decrements it. **Ran t09:** 21 promises reset, each accepted by the client, gave GOAWAY(ENHANCE_YOUR_CALM, "too_many_resets").
- **`src/proto/streams/streams.rs:780-782` (acknowledged TODO):** reserved streams have no limit. **Ran t37:** 5,000 promises left `num_wired_streams` at 5,001 with `max_concurrent_streams(10)`.

## Low severity

**29. Server push API misuse is not rejected**
- **`src/proto/streams/streams.rs:1209`:** when `convert_push_message` fails, the `?` skips removal of the child stream. **Ran t29:** three invalid pushes returned errors and left `num_wired_streams` 3 higher, even after the parent finished.
- **`src/proto/streams/send.rs:99-123`:** the parent's state is not checked. **Ran t30:** `push_request` after END_STREAM returned Ok and PUSH_PROMISE went out on a closed stream. Per finding 22's code path, an h2 client treats that as a connection error.
- **`src/proto/streams/prioritize.rs:673-675`:** resetting the parent drops the queued PUSH_PROMISE and leaves the child with `is_pending_push` set forever. **Ran t23:** `pushed.send_response` returned Ok, nothing was sent during the test, and the stream stayed wired.

**30. Client send-side validation gaps**
- **`src/share.rs:287`:** `capacity as WindowSize` truncates. **Ran t31:** `reserve_capacity(2^32 + 10)` gave capacity 10.
- **`src/client.rs:1637` (`TODO: Error`):** a non-CONNECT request with an authority-only URI is sent without `:scheme`. **Ran t32:** `send_request` returned Ok and the wire pseudo-headers had `scheme: None`.
- **`src/proto/streams/send.rs:269-292`:** `send_trailers` skips `check_headers`. **Ran t33:** `connection: close` in trailers returned Ok and went on the wire, where the mock's h2 decoder rejected it.

**31. Protocol conformance**
- **`src/frame/headers.rs:638`:** a client treats 101 as an ignorable 1xx, while RFC 9113 §8.6 says it is malformed. **Ran t36:** the future kept waiting, with no error, until the 150 ms timeout.
- **`src/frame/headers.rs:435`:** a 4-byte PUSH_PROMISE whose block is entirely in CONTINUATION is rejected. **Ran t20:** GOAWAY(PROTOCOL_ERROR).
- **`src/frame/headers.rs:869, 911`:** `<` makes a header list of exactly `max_header_list_size` count as oversize. **Ran t16:** exactly 200 bytes with limit 200 was answered with END_STREAM HEADERS and no request was accepted.
- **`src/hpack/decoder.rs:186`:** `can_resize` is reset for each `decode` call. **Ran t19:** a size update at the start of a CONTINUATION frame, mid-block, was accepted.
- **Wrong error codes, `src/codec/framed_read.rs:204-207` and siblings. Ran t44**, all answered with GOAWAY(PROTOCOL_ERROR):
  - WINDOW_UPDATE increment 0 on a stream, where the RFC says stream error (`frame/window_update.rs:41`).
  - Wrong-length PING, SETTINGS, SETTINGS ACK, RST_STREAM and WINDOW_UPDATE, where the RFC says FRAME_SIZE_ERROR.
  - Wrong-length PRIORITY, where the RFC says stream error FRAME_SIZE_ERROR.
  - INITIAL_WINDOW_SIZE of 2^31, where the RFC says FLOW_CONTROL_ERROR.
  - HPACK decode failures, where the RFC says COMPRESSION_ERROR.

**32. Read only, not executed**
- **`src/frame/headers.rs:975-980`:** `calculate_header_list_size` omits `:protocol`, so the size is undercounted when a block resumes across frames.
- **`src/hpack/decoder.rs:166-173`:** `queue_size_update` keeps the maximum of pending values, so if a lower setting is acknowledged last, the peer may still update to the older, larger size.
- **`src/frame/go_away.rs:43`:** `GoAway::load` never checks that the stream ID is 0, so GOAWAY on a non-zero stream is accepted.
