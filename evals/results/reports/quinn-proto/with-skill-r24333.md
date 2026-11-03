Audit of quinn-proto 0.11.13 (`crate/quinn-proto/src/`)

Verdict: the crate builds and its own suite passes (263 unit tests, 3 doctests on default features). It still has remote panics, a stream-limit bypass, flow-control accounting errors and 0-RTT state bugs. I found 31 defects: 3 Blocker, 17 Major, 11 Minor, 0 Note.

`crate/` was not modified. Probes ran through `./run`, in a copy of the workspace at `tmp/q` (with test helpers and env-var hooks added to that copy) and in a throwaway crate at `tmp/probe`. Each finding says whether it was run or only read. Nothing in the repository or its output addressed me.

## Blocker

### Transport parameter parser panics on peer bytes
Location: `transport_parameters.rs:472` and `:479`.
- **Problem:** `max_datagram_frame_size` and `min_ack_delay` call `r.get().unwrap()` without checking `len` or the bytes remaining.
- **Minimal inputs:** `20 00` and `c0 00 00 00 ff 04 de 1b 00`.
- **Effect:** the `TransportParameters::read` call is reached from the client's Initial (ClientHello) before any peer authentication, through `Endpoint::accept`, so one crafted Initial kills the server task. A malicious server can do the same to a client. Non-empty lengths are also ignored: `20 02 05 0c 00` parses as `Ok`, with the second value byte read as a new parameter.
- **Established by:** running it. `TransportParameters::read` panicked on 642 of the 1-to-3-byte inputs enumerated. A full pair test with a client sending those bytes in its ClientHello panicked the server at both lines.
- **Fix:** check `len` and decode from `r.take(len)`, requiring full consumption, as the generic parameter path does.

### ACK frequency `clamp` panics on the peer's `min_ack_delay`
Location: `connection/ack_frequency.rs:49`.
- **Problem:** `.clamp(min_ack_delay, rtt.max(25ms))` asserts min <= max. `min_ack_delay` comes from the peer and is only validated against `max_ack_delay`.
- **Effect:** with `ack_frequency_config` set locally, any peer advertising `min_ack_delay` above max(RTT, 25 ms) panics `poll_transmit`. An honest peer can trigger this too.
- **Established by:** running it. A full pair test with a server advertising `max_ack_delay` of 2000 ms and `min_ack_delay` of 1 s panicked the client with "assertion failed: min <= max".
- **Fix:** use `.max(min).min(upper)` or otherwise tolerate min above max.

### MAX_STREAM_DATA bypasses the peer stream limit
Location: `connection/streams/state.rs:768`.
- **Problem:** `received_max_stream_data` calls `on_stream_frame` even when the id has no slot. A remote-initiated bidi id above `max_remote` therefore advances `next_remote`, with no STREAM_LIMIT_ERROR.
- **Effect:** one frame makes `Streams::accept` hand out an effectively endless series of unpermitted ids and raises `send_streams`. `max_concurrent_bidi_streams` no longer protects the application.
- **Established by:** running it on `StreamsState` with a limit of 1 and stream index 1,000,000,000. The call returned `Ok`, an `Opened` event fired, and `accept` returned 5,000,000 ids before I stopped the loop. `send_streams` was 5,000,000. I read the frame dispatch at `connection/mod.rs:2822` but did not send this over the wire.
- **Fix:** validate with `validate_receive_id`-style limits for remote ids, and only call `on_stream_frame` for slots that exist.

## Major

### Panic on a retransmitted Initial after `set_server_config(None)`
Location: `endpoint.rs:211`; the same unwrap pattern is at `:540` and `:729`, which I read but did not run.
- **Problem:** `self.server_config.as_ref().unwrap()` assumes a config is still set while an `Incoming` is pending.
- **Effect:** after an application stops accepting, the client's next Initial retransmit panics the endpoint. `accept(.., None)` and `retry()` have the same unwrap, undocumented.
- **Established by:** running it. Pair test: `Wait`, `set_server_config(None)`, advance past the PTO; panicked at `endpoint.rs:211`.
- **Fix:** handle `None` by dropping the datagram or storing the config in `Incoming`.

### Endpoint spins forever in `new_cid` with short CIDs
Location: `endpoint.rs:402`, with the check at `:921`.
- **Problem:** `cids_exhausted` is only checked at accept, but each connection then issues up to 7 more CIDs. A burst of Initials fills the CID space and `new_cid` loops without end.
- **Effect:** the endpoint hangs. This needs a short CID length; I tested 1 byte.
- **Established by:** running it. With a 1-byte generator, 120 connections were accepted, then `drive_outgoing` did not return within 20 s. I inferred from the code that it was spinning in `new_cid`; I did not sample a stack.
- **Fix:** cap issuance in `send_new_identifiers` and return fewer ids when the space is near full.

### Connection flow-control credit is issued twice
Location: `connection/streams/state.rs:351-356` together with `recv.rs:95` and `streams/mod.rs:159`.
- **Problem:** `stop()` credits `end - bytes_read`, and `received_reset` credits `final - bytes_read`, but `Assembler::clear` leaves `bytes_read` unchanged.
- **Effect:** stop-then-reset and reset-then-stop both double the credit, so the peer can exceed `receive_window`.
- **Established by:** running it on `StreamsState`. Both orders with 2048 bytes gave a credit of 4096, where 2048 is correct.
- **Fix:** issue credit once, based on what has already been credited.

### Duplicate RESET_STREAM kills the connection with FLOW_CONTROL_ERROR
Location: `connection/streams/recv.rs:184` and `:218`.
- **Problem:** `reset()` does not record `final_offset` in `self.end`, and it runs the flow-control check before detecting a redundant reset. The bytes are counted again.
- **Effect:** a normal retransmitted RESET_STREAM can close a healthy connection.
- **Established by:** running it. Window 1000: reset A at 500, then 1000 bytes on B, then the same reset again returned FLOW_CONTROL_ERROR.
- **Fix:** return the redundant case before the credit check, or update `end`.

### FIN below the received high-water mark is accepted, and the stream never ends
Location: `connection/streams/recv.rs:60-75`.
- **Problem:** with no known final size, a FIN whose end is below `self.end` is stored as the size. RFC 9000 requires FINAL_SIZE_ERROR.
- **Effect:** `size == Some(rs.end)` can never hold, so reads return `Blocked` forever and the stream slot leaks.
- **Established by:** running it. 100 bytes, then FIN at offset 50: `Ok(())`, then `Err(Blocked)` after all data was read.
- **Fix:** reject when `frame.fin && end < self.end`.

### `IllegalOrderedRead` destroys the stream
Location: `connection/streams/recv.rs:269-272`.
- **Problem:** the `Recv` is removed from the map before `ensure_ordering(ordered)?`, so the error path drops it without freeing it.
- **Effect:** the next read returns `ClosedStream`, buffered data is lost, its connection credit is never returned, and the stream slot is never released.
- **Established by:** running it. Unordered read, then ordered read (error), then unordered read returned `ClosedStream`. The entry was gone and `allocated_remote_count` stayed at 128.
- **Fix:** call `ensure_ordering` before `entry.remove()`.

### 0-RTT rejection permanently consumes the send window
Location: `connection/streams/state.rs:223-248`.
- **Problem:** `zero_rtt_rejected` resets `data_sent` but not `unacked_data`, and no ACK can ever release it because the streams are gone.
- **Effect:** after rejection, writes on new streams can block for the life of the connection.
- **Established by:** running it. With `send_window` 10,000 and 10,000 bytes written in 0-RTT, a write to a new stream after rejection returned `Err(Blocked)`.
- **Fix:** zero `unacked_data` there.

### 0-RTT rejection keeps the remembered connection flow-control limit
Location: `connection/streams/state.rs:201` and `:728`.
- **Problem:** `received_max_data` takes the maximum of the cached and new `initial_max_data`, so a smaller value from the server is ignored.
- **Effect:** the client overruns the real limit and the server closes the connection.
- **Established by:** running it. Server window 2000 after rejection: the client write returned `Ok(100000)` and the server lost the connection with FLOW_CONTROL_ERROR.
- **Fix:** reset `max_data` on rejection before applying the new parameters.

### Streams opened in accepted 0-RTT keep the cached stream limit and can stall
Location: `connection/streams/state.rs:202-207`.
- **Problem:** `set_params` only updates remote-initiated bidi streams, so local 0-RTT streams keep the remembered limit. The server thinks the limit is larger and sends no MAX_STREAM_DATA until window/8 has been read.
- **Effect:** the stream deadlocks when the server has raised its window by a large factor since the ticket was issued.
- **Established by:** running it. Ticket window 1000, new window 100,000, 0-RTT accepted: 1000 bytes were written, the server drained them, and the next client write returned `Err(Blocked)` after driving to idle.
- **Fix:** update `max_data` on existing local streams, keeping the maximum.

### Forged Retry and Version Negotiation packets count as authenticated
Location: `connection/mod.rs:2339-2346`, with the checks at `:2455` and `:2641`.
- **Problem:** `on_packet_authenticated` runs for unprotected packets before they are validated.
- **Effect:** an observer who knows a CID can keep a dead connection's idle timer alive, or send two junk Retries so the genuine Retry is discarded and the handshake never completes.
- **Established by:** running it. A forged Retry on an established client moved the idle deadline by 5 s (`total_authed` went from 11 to 12). With 2 forged Retries the client was still handshaking after 2000 steps; the control with 0 connected.
- **Fix:** call it only after decryption, or after Retry/VN validation.

### Zero-length datagrams are buffered without limit
Location: `connection/datagrams.rs:132-139`.
- **Problem:** `recv_buffered` does not grow for empty datagrams, so the window never evicts them. `was_empty` also stays true, adding one event per frame.
- **Effect:** a peer gets about 32 bytes of memory per 2 wire bytes. By default this is reachable in applications that never read datagrams, since the default receive buffer is enabled.
- **Established by:** running it. With a 100-byte window, 50,000 empty datagrams were all retained, with 50,000 `DatagramReceived` events.
- **Fix:** count an overhead per datagram, or cap the queue length.

### Version Negotiation is an unauthenticated amplifier
Location: `endpoint.rs:164-195`.
- **Problem:** there is no minimum datagram size or rate limit, although RFC 9000 §5.2.2 says to respond only if the datagram is long enough to initiate a connection.
- **Effect:** spoofed-source reflection at more than the 3x limit.
- **Established by:** running it. A 7-byte datagram produced a 39-byte response.
- **Fix:** require `MIN_INITIAL_SIZE`.

### Off-path PATH_RESPONSE ignores the anti-amplification limit
Location: `connection/mod.rs:859`.
- **Problem:** it is always padded to 1200 bytes to an unvalidated address and is not counted in any path budget. RFC 9000 §8.2.2 forbids this.
- **Effect:** a peer that spoofs a source address in a small PATH_CHALLENGE gets 1200 bytes sent to a victim. The server's migration setting must be enabled, which is the default.
- **Established by:** partly running it. A response queued through a test helper went out as 1200 bytes to the other address. The inbound path was read, not sent over the wire.
- **Fix:** skip the padding when the budget does not allow it.

### ECN validation counts ACK ranges instead of packets
Location: `connection/mod.rs:1545`; `ArrayRangeSet::len` at `range_set/array_range_set.rs:55`.
- **Problem:** `newly_acked.len()` is the number of ranges, so a contiguous flight counts as 1 and bleaching is not detected.
- **Effect:** ECN stays enabled on paths that clear the marks.
- **Established by:** running it. In a 6-packet flight with marks cleared on all but one, `using_ecn` stayed true.
- **Fix:** count the packets, for example `newly_acked.elts().count()`.

### Long-header packets over 16383 bytes hit an assert
Location: `packet.rs:486`.
- **Problem:** `initial_mtu` can exceed the 2-byte length field reserved in long headers, and nothing caps long-header packets to it.
- **Effect:** `poll_transmit` panics.
- **Established by:** running it. `initial_mtu(30000)` with a large initial congestion window and a 100,000-byte 0-RTT write panicked with `len < 2usize.pow(14)`.
- **Fix:** cap long-header packet size at 16383 bytes.

### `initial_mtu` above the congestion window sends nothing and never times out
Location: `connection/mod.rs:605-606`, with the window set in `congestion/*::new`.
- **Problem:** the initial window ignores the MTU, so even the first Initial is reported as congestion blocked.
- **Effect:** the connection attempt hangs forever with no error.
- **Established by:** running it. With `initial_mtu(20000)` there were 0 datagrams, `poll_timeout` returned `None`, and there were no events.
- **Fix:** start the window at max(configured, 2 × MTU).

### Close frames can exceed the MTU
Location: `frame.rs:334`; the same arithmetic is at `:293` (read only).
- **Problem:** only 3 bytes are reserved for the type and error code, but an application code can take 8.
- **Effect:** the datagram is oversized and may be dropped, so the peer may never see the close.
- **Established by:** running it. `VarInt::MAX` with a 5000-byte reason gave 1458 bytes against an MTU of 1452; code 0 gave 1451.
- **Fix:** subtract the real code size.

## Minor

### Retired NEW_CONNECTION_ID frames queue without a limit
Location: `connection/mod.rs:2932`.
- **Problem and effect:** this path skips `MAX_PENDING_RETIRED_CIDS`, so repeating an old sequence number grows `retire_cids` while sending is blocked.
- **Established by:** reading only.
- **Fix:** apply the same limit.

### RETIRE_CONNECTION_ID off by one
Location: `connection/cid_state.rs:163`.
- **Problem and effect:** `sequence == issued` is accepted, although it was never issued and must be a PROTOCOL_VIOLATION.
- **Established by:** running it. `on_cid_retirement(1, 2)` with `issued` = 1 returned `Ok(true)`.
- **Fix:** use `>=`.

### `Endpoint::connect` leaks a CID on failure
Location: `endpoint.rs:344` and `:353-355`.
- **Problem and effect:** `new_cid` inserts into the index before `start_session(..)?` can fail, leaving a stale route.
- **Established by:** running it. 10 invalid server names took `known_cids` from 0 to 10.
- **Fix:** remove the entry on error.

### Stateless reset comparison is not constant time
Location: `connection/packet_crypto.rs:42`.
- **Problem and effect:** `as_deref()` compares plain slices and bypasses `ResetToken`'s constant-time `PartialEq`. RFC 9000 §10.3.1 requires a comparison that does not leak the token value.
- **Established by:** reading only.
- **Fix:** use `constant_time::eq`.

### `drop_oversized` drops datagrams that fit
Location: `connection/datagrams.rs:149`.
- **Problem and effect:** it uses `<` where `send` allows `len <= max`.
- **Established by:** running it. A datagram of exactly the limit was dropped.
- **Fix:** use `<=`.

### Datagram send buffer overshoots its documented limit
Location: `connection/datagrams.rs:39`.
- **Problem and effect:** the loop ignores the new datagram's size, while the docs say older ones are dropped until there is space.
- **Established by:** partly running it. Two 900-byte sends into a 1000-byte buffer both succeeded with `send_buffer_space` at 0. From reading, nothing was dropped and 1800 bytes stayed queued; I did not print `outgoing_total`.
- **Fix:** include `data.len()` in the loop condition.

### Safe `StreamId` conversion breaks the `VarInt` invariant
Location: `lib.rs:280`.
- **Problem and effect:** `StreamId::new` does not range-check, and `From<StreamId>` then calls `from_u64_unchecked`, violating its `# Safety` contract. This is a panic rather than memory unsafety.
- **Established by:** running it. Index 2^61 gave a `VarInt` of 2^63, and encoding panicked at `varint.rs:190`.
- **Fix:** validate in `StreamId::new`.

### NEW_TOKEN is never reissued after migration
Location: `connection/mod.rs:2810`.
- **Problem and effect:** this sets `validated` directly instead of calling `on_path_validated`, although the comment at `:3359-3362` says tokens may be enqueued for the new path. Old-path tokens are discarded.
- **Established by:** running it. The `new_token` count stayed at 2 after the server migrated.
- **Fix:** call `on_path_validated`.

### Parser accepts duplicates and trailing bytes
Location: `transport_parameters.rs:460`, `:474`, `:478`.
- **Problem and effect:** duplicate `grease_quic_bit` and `min_ack_delay` are accepted, and bytes left inside `preferred_address` are parsed as new parameters.
- **Established by:** running the duplicate `grease_quic_bit` case (`Ok`). The other two were read only.
- **Fix:** reject them with TRANSPORT_PARAMETER_ERROR.

### `MtuDiscoveryConfig::minimum_change(0)` never finishes probing
Location: `connection/mtud.rs:333`.
- **Problem and effect:** `< 0` can never be true, so the stop condition cannot fire.
- **Established by:** running it, with an indirect result. The connection test did not reach idle and was killed at the 300 s `timeout` while inside `pair.drive()`. The probe count was not observed.
- **Fix:** clamp the value to at least 1.

### Undocumented `unsafe`
Location: `connection/send_buffer.rs:99` and `:118`, `connection/mod.rs:3261`, `lib.rs:280`.
- **Problem and effect:** none of the 4 blocks has a SAFETY comment. The three size calculations look sound.
- **Established by:** grep and reading.
- **Fix:** add comments, or use a checked size helper.

## Verification

- `cargo test --locked -p quinn-proto`: pass (263 unit tests, 3 doctests).
- `cargo clippy --locked -p quinn-proto --all-targets`: 2 warnings on the 1.99 toolchain (for example `manual_filter`), which I treat as toolchain drift, not findings.
- Skill gate `scripts/check.sh --audit` on `crate/quinn-proto`: format ran and did not fail. Clippy, rustdoc, tests and audit lints failed only because `--all-features` builds `aws-lc-fips-sys`, whose build script failed in this container.
- `audit_scan.py`: 1 weasel-word type name and 8 bare TODOs; none reported as defects.
- Random frame injection into established client and server connections: about 500,000 injected packets across 58,622 short sessions on the `tmp/q` copy, with no panics other than the seeded harness failures fixed along the way. It did not exercise `min_ack_delay`, the first Blocker's bytes or the other paths above, and many sessions ended early on protocol errors, so coverage is limited.
- I did not run Miri, a build at the declared MSRV, or a long fuzz of `PartialDecode::new`.

Findings: 31. Last `./clock` reading: 32 minutes used of 45.
