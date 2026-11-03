44 findings, clock at 23 of 45.

**`report.md` was not written.** A harness hook blocked the Write ("Subagents should return findings as text, not write report files"), and I did not route around it with Bash. The full report is below instead, which also breaks the brief's 40-word limit. Please save it to `<workspace>/report.md`, or tell me to.

# Audit: h2 0.4.12

**Method:**
- Probes are in `<workspace>/probe/tests/` (`p.rs`, `q.rs`, `m.rs`), named p*/q*/m* below. They use the `h2-support` mock transport and ran through `./run`.
- "Ran" means I executed the probe and saw the stated result. "Read" means I only read the code.
- `crate/` was not changed. No repository text addressed me.
- Baseline passes: `-p h2 --lib` (419 tests), plus the `push_promise`, `stream_states`, `flow_control`, `server` and `client_request` test files.
- Not run: the stress test, Miri, and the remaining h2-tests files.
- RFC references are from memory.

## High

1. **Remote panic on client from a pushed 1xx response.** `proto/streams/state.rs:157-168` keeps `ReservedRemote` and returns `initial = true` for a 1xx, so `recv.rs:171` counts the stream twice. PUSH_PROMISE, then HEADERS 103, then HEADERS 200 hits `counts.rs:111` `assert!(!stream.is_counted)`. The panic poisons the mutex and kills the connection. Push is on by default. Ran p1.

2. **Remote panic on client over the concurrency limit.** With `max_concurrent_streams(1)`, two PUSH_PROMISEs then HEADERS on both hit `counts.rs:110`. Reserved streams are checked at `Recv::open` but not counted, so the later increment's assert fails. Ran p2.

3. **Unbounded memory from empty DATA frames.** `recv.rs:675-679` queues zero-length DATA regardless of flow control. 500,000 frames (4.5 MB on the wire) on an unread body grew the heap by 133 MB, or 266 bytes per frame. Ran m40.

4. **A peer's GOAWAY kills streams that peer initiated.** `streams.rs:730-737` applies `last_stream_id` to every stream, but it only covers streams initiated by the GOAWAY's receiver.
   - A client's graceful GOAWAY(0, NO_ERROR) aborts all in-flight requests on the server. `accept` returns None and `send_response` fails. Ran q28.
   - A server's GOAWAY(1) kills pushed stream 2 on the client. Ran q28b.

5. **`send_reset` before the stream opens kills the connection.** `send.rs:222-233` clears the queued HEADERS and queues RST_STREAM, so the RST goes out on an idle stream. Against a real h2 server over a duplex pipe, the server sends GOAWAY PROTOCOL_ERROR and the other request on the connection fails. Ran p4, q33.

6. **PUSH_PROMISE crossing a client cancel kills the connection.** The parent checks are at `streams.rs:753-778`, and the RFC expects this race. Three outcomes, all ran:
   - Handles dropped, reset still scheduled: `state.rs:437-439` becomes GOAWAY(CANCEL) and the healthy request fails (q17b).
   - Parent forgotten: GOAWAY PROTOCOL_ERROR at `streams.rs:775` (q17c).
   - After user `send_reset`: debug panic at `connection.rs:446`. In release the promised ID is untracked, so the next HEADERS draws GOAWAY PROTOCOL_ERROR (q17a in both profiles).

7. **Server panic when a push handle is dropped.** `prioritize.rs:828` does `find_mut(promised_id).unwrap()`. `SendPushedResponse` can be dropped and the child reaped before the PUSH_PROMISE is written. Ran p3a with reset limit 0. Ran p3b with default config: parent data blocked on flow control for over 1 s, and a client WINDOW_UPDATE then triggers the panic.

8. **Any h2 panic can abort the process.** `share.rs:487` calls `streams.rs:1415` `lock().unwrap()` inside `RecvStream::drop`. On a poisoned mutex this panics, and during unwinding it double-panics. `drop_stream_ref` guards this case; this path does not. Ran p3a and p3b: `Aborted`.

9. **Padding is not counted in flow control.** `recv.rs:580` uses the payload length after padding is stripped.
   - 65,312 flow-controlled octets produced a WINDOW_UPDATE of only 52,000, so a conformant peer's window shrinks until it stalls.
   - 512,000 octets of padding-only frames were accepted against a 65,535 window, with no error and no credit.
   - Ran p5.

10. **A cancelled queued stream permanently loses the connection send window.** `reclaim_reserved_capacity` (`prioritize.rs:349-364`) leaves `requested_send_capacity` set. When the dead stream leaves `pending_open`, `prioritize.rs:538` assigns it capacity that never returns. In release, later DATA is never sent and `capacity()` stays 0. In debug it panics at `prioritize.rs:448`. Ran q41.

11. **Late frames from a benign peer exhaust a lifetime cap.** Each STREAM_CLOSED reply for a forgotten stream counts toward `DEFAULT_LOCAL_RESET_COUNT_MAX` (`streams.rs:1575-1577`, `counts.rs:80-93`), which never decays. After 1,024 late DATA frames the client sent GOAWAY ENHANCE_YOUR_CALM and a healthy request failed. Ran q14 with reset limit 0. Read: default config forgets a stream after 1 s or past 50 concurrent.

12. **Trailers on a forgotten stream cause a connection error.** `streams.rs:448` gives only clients the forgotten-stream path, so a server reaches `recv.rs:137-139` and sends GOAWAY PROTOCOL_ERROR. DATA in the same situation gets STREAM_CLOSED. Ran q13, and q13b with default config.

13. **Header validation is bypassed by splitting the block.** `malformed` is local to each `HeaderBlock::load` call (`frame/headers.rs:854`), and a NeedMore error returns before checking it (`:930-933`). A `connection: close` request ending its HEADERS frame mid-way through the next header is accepted with the field silently removed. In one frame it is RST. Ran p7.

14. **HPACK state desync.** `framed_read.rs:154-160` returns on a self-dependent HEADERS before decoding its block. Dynamic table inserts are lost, and the next request referencing index 62 draws GOAWAY. The control with a valid dependency works. Ran q34.

15. **A malformed block with CONTINUATION escalates to GOAWAY.** `framed_read.rs:173-177` returns a stream reset without storing `partial`, so the next CONTINUATION fails at `:286`. Ran p8.

## Medium

16. **Queued streams hoard connection capacity, which can deadlock.** `send_data` guards `is_pending_open` (`prioritize.rs:193`) but `reserve_capacity` does not (`:287`). Unopened stream 3 took 65,535 of capacity. Open stream 1's `poll_capacity` timed out, and it must finish before 3 can open. Ran q39.

17. **`SendRequest::ready()` can hang.** `send_task` is one waker slot (`stream.rs:369`), shared by `poll_pending_open` (`streams.rs:988`) and `SendStream::poll_reset`/`poll_capacity`. With `poll_reset` registered afterwards, the stream opened but `ready()` never woke. The control completes. Ran q38.

18. **Dropping `RecvStream` keeps its window.** `streams.rs:1414-1421` clears buffered data without releasing `in_flight_recv_data`. With `SendResponse` still held, 65,535 unread bytes produced no WINDOW_UPDATE, starving the whole connection. Dropping both gives an immediate update. Ran q15, q15b.

19. **Buffer slab leak.** `recv.rs:434-436` returns early when `in_flight_recv_data == 0`, so queued Headers events stay in the slab after the stream is removed. Measured with an 8 KB header:
   - Dropping an unpolled `ResponseFuture` leaks 9,113 bytes per request; the control leaks 0.
   - The unpolled-push case grew by 37,585 bytes per request, with no separate control.
   - Read: late trailers after `RecvStream` is dropped have the same cause.
   - Ran m18, m18b.

20. **Reset counter leak.** `clear_expired_reset_streams` (`recv.rs:963`) passes `is_reset_counted = true`, but `counts.rs:224-230` only decrements if the stream is closed. A stream that still has queued frames at expiry leaks a slot, and later cancels are forgotten immediately. Ran q36, which got STREAM_CLOSED versus the control's silent ignore.

21. **Parent reset orphans a queued push.** `clear_queue` drops the PUSH_PROMISE, but the child keeps `is_pending_push`. `send_response` returns Ok, nothing is ever sent, and `poll_reset` hangs. Ran q31.

22. **DATA on ignored streams after GOAWAY is not counted.** `streams.rs:552-558` skips connection flow control. 65,535 bytes on ignored stream 3 got no WINDOW_UPDATE, so stream 1 can no longer send. Ran q25.

23. **Unbounded PUSH_PROMISE reservations.** See the TODO at `streams.rs:780-782`. 20,000 frames produced 20,001 wired streams and 19 MB on a default client. Ran m41.

24. **Encoder table follows the peer's size with no cap.** `proto/settings.rs:139-141`, `hpack/encoder.rs:30`. Over 10,000 responses with distinct header values, heap growth was 5.0 MB versus 58 KB at table size 4096, and time was 1.75 s versus 0.69 s. Ran m26.

25. **Response without `:status` becomes 200 OK.** `client.rs:1676-1678` falls back to the builder default. `:path`/`:method` in a response are also ignored rather than rejected. Ran q9.

26. **Requests missing required pseudo-headers are accepted.** `server.rs:1558-1609`. Ran p10:
   - GET with no `:path` and no `:authority`.
   - CONNECT with no `:authority`.
   - GET with `:path *`.

27. **Trailers are not validated.** `recv.rs:348-370` discards pseudo-headers instead of rejecting them (p11). It also ignores `is_over_size`: oversize trailers were silently truncated to `{a}`, dropping `b` and `c` (q32).

28. **Content-length checks are incomplete.** `recv.rs:178` uses `.get()`, so only the first value is checked. Values 5 and 100 with a 5-byte body were accepted and both forwarded, a smuggling risk behind a proxy. An empty value parses as 0 (`frame/headers.rs:326-344`). Ran p22.

29. **Pushed HEAD responses with content-length are reset.** `ContentLength::Head` is only set for local requests (`streams.rs:271`), not in `recv_push_promise`. The pushed response fails with a stream error. Ran q12.

30. **304 with content-length then empty END_STREAM DATA is reset.** The exemption at `recv.rs:190-196` does not cover `:641-648`. Ran q23.

31. **`RecvStream::trailers()` hangs on unread data.** `recv.rs:1146-1150` returns Pending without registering a waker. Timed out with DATA and trailers already received. Ran q16.

## Low

32. **Writer returning Ok(0) spins forever.** The loop at `framed_write.rs:137-153` never yields, so tokio's timeout never fires. Ran q20.

33. **`reserve_capacity` truncates.** `share.rs:287`: 2^32+5 reserves 5. Ran q21.

34. **Rejected `push_request` leaks a stream.** The `?` at `streams.rs:1209` skips child cleanup. Wired streams went from 1 to 101 after 100 rejected calls. Ran q19.

35. **`push_request` succeeds on a closed parent.** Returns Ok and emits PUSH_PROMISE after END_STREAM (`send.rs:99-123`). Ran q27.

36. **Outgoing connection-specific headers are under-checked.** `send_trailers` skips `check_headers` (`send.rs:269-292`). `te: trailers` plus `te: gzip` also passes (`:90`). Both are emitted malformed. Ran q30, q42.

37. **Credentials go on the wire in `:authority`.** `frame/headers.rs:596-598` sent `user:secret@a.b`. Ran q37.

38. **Client cancel is delayed.** `SendRequest.pending` holds a stream ref (`client.rs:521-525`), so no RST is sent until the next `poll_ready` or until that handle is dropped. Ran the earlier version of q36: no RST with the ref held, RST once it was released.

39. **Header list limit is off by one.** `frame/headers.rs:869,911` use `<`, so a list exactly at the limit is rejected. Ran q32.

40. **Valid PUSH_PROMISE rejected.** `frame/headers.rs:435` refuses an empty first fragment followed by CONTINUATION, with GOAWAY. Ran q35.

41. **HPACK block-split inconsistencies.** Both are accepted when split but rejected whole. Ran p29, p30.
   - An empty-name literal returns NeedMore (`hpack/header.rs:65-67`) and is silently dropped, including its table insert.
   - `can_resize` resets per fragment (`hpack/decoder.rs:186`), allowing a mid-block size update.

42. **Wrong error codes or scope.** All PROTOCOL_ERROR GOAWAY (p32, p31):
   - HPACK failures, which should be COMPRESSION_ERROR (`framed_read.rs:179-180`).
   - Bad frame sizes, which should be FRAME_SIZE_ERROR.
   - Oversized initial window, which should be FLOW_CONTROL_ERROR.
   - Zero stream WINDOW_UPDATE and DATA after END_STREAM, which should be stream errors.
   - Uppercase header names, which are malformed messages and should be stream errors.
   - Separately, GOAWAY on a non-zero stream is accepted as real (`framed_read.rs:248-249`).

43. **Lenient message handling.** Ran q42.
   - A 101 is treated as informational, so the response future hangs (`frame/headers.rs:638-641`).
   - Field values with leading or trailing whitespace are accepted (`hpack/header.rs:99`).

44. **Possible lost wakeup for user pings.** `ping_pong.rs:181,193` checks state before registering the waker, the opposite of `poll_pong`. A ping sent in that window waits for the next connection poll. Read only.
