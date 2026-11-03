# Audit report: quinn-proto 0.11.13

I did not change `crate/`. Baseline `cargo test --locked -p quinn-proto` passes (263 unit tests, 3 doc tests), so none of the defects below are caught by the existing suite.

Probes ran through `./run` in a throwaway copy at `tmp/probe`, with test modules added and three test-only edits:
- test helpers widened to `pub(crate)`
- `RttEstimator::new` widened to `pub(crate)`
- a thread-local hook in `to_vec` that appends forged transport-parameter bytes

"Ran" below means I executed a probe and saw the stated result. Several probes feed frames straight into `Connection::process_payload` or call `StreamsState` directly, and those are marked. "Read only" means I did not execute it.

## High

**1. Remote panic while parsing transport parameters: `transport_parameters.rs:472`, `:479`**
- **What's wrong:** `Some(r.get().unwrap())` for `max_datagram_frame_size` and `min_ack_delay` never checks that bytes remain. `min_ack_delay` has no length check at all.
- **What a caller sees:** a client's ClientHello can panic the server inside `Endpoint::accept` before any authentication. A server can do the same to a client.
- **Trigger:** the parameter bytes `20 00` at the end of the block, when the sender has not already sent `max_datagram_frame_size`; otherwise `c0 00 00 00 ff 04 de 1b 00`.
- **Ran:** all four unit cases panicked with `unwrap() on Err: UnexpectedEnd`. End to end, a real handshake with forged parameters panicked at `:479` on both the server and the client.

**2. MAX_STREAM_DATA bypasses the stream limit and creates phantom streams: `connection/streams/state.rs:768`, reached through `:731-770`**
- **What's wrong:** for a remote bidirectional stream above `max_remote`, no entry exists and no error is raised, but `on_stream_frame(false, id)` still sets `next_remote = index + 1`. The RFC requires STREAM_LIMIT_ERROR. STOP_SENDING (`:366-373`) also ignores the limit, though it creates no streams.
- **What a caller sees:** one frame makes the application receive `Opened`, then `accept()` returns IDs up to the attacker's index, each incrementing `send_streams`. Every returned stream is dead. An accept loop can be made to spin or spawn tasks almost without limit.
- **Ran (direct `StreamsState` call):** index 1,000,000 against a limit of 4 returned `Ok`. `accept()` yielded 1,000,001 streams, `send_streams` was 1,000,001, and reading the last one gave `ClosedStream`.

**3. Peer-triggered panic when ACK frequency is enabled: `connection/ack_frequency.rs:46-49`**
- **What's wrong:** `.clamp(min_ack_delay, rtt.max(25ms))` panics when the peer's `min_ack_delay` exceeds that maximum. Parameter validation only requires `min_ack_delay <= max_ack_delay`, and `max_ack_delay` can be up to about 16 s.
- **What a caller sees:** any endpoint with `ack_frequency_config` set can be panicked by its peer.
- **Ran:** a real handshake with a forged server `max_ack_delay=2000`, `min_ack_delay=1000000` parsed fine, then panicked the client with `assertion failed: min <= max`.

## Medium

**4. `set_server_config(None)` panics on later traffic: `endpoint.rs:211`, `:540`, `:729`**
- **What's wrong:** the code unwraps `self.server_config` while an `Incoming` is still outstanding, even though `None` is an accepted value.
- **What a caller sees:** a retransmitted client Initial panics the endpoint, and so does `accept(incoming, …, None)`.
- **Ran:** both panicked (`:211`, `:540`). `:729` (`retry`) is read only.

**5. Connection flow-control credit is issued twice: `connection/streams/state.rs:351-356`, `connection/streams/recv.rs:95`**
- **What's wrong:** `stop()` credits `end - bytes_read`, and RESET_STREAM credits `final - bytes_read`. Both credit the same bytes, whichever arrives first.
- **What a caller sees:** a peer can inflate `MAX_DATA` beyond the configured `receive_window` and make the receiver buffer more than intended.
- **Ran (direct `StreamsState` calls):** 1000 bytes raised `local_max_data` by 2000, for both stop-then-reset and reset-then-stop.

**6. Rejected 0-RTT permanently consumes the send window: `connection/streams/state.rs:223-248`**
- **What's wrong:** `zero_rtt_rejected` resets `data_sent` but not `unacked_data`, and no ACK can ever clear it.
- **What a caller sees:** writes block forever if 0-RTT writes reached `send_window`. Smaller writes shrink the window permanently.
- **Ran:** end to end with `send_window=1000`, after writing 1000 bytes and being rejected, a new stream's write returned `Err(Blocked)` on an idle connection.

**7. Rejected 0-RTT keeps the old connection credit: `connection/streams/state.rs:223-248`, `:201`, `:728`**
- **What's wrong:** `max_data` is not reset, and `received_max_data` only ever raises it, so a lower limit from the new handshake is ignored.
- **What a caller sees:** the client overruns the server's limit and the connection dies.
- **Ran:** end to end, the old limit was 1,000,000 and the new one 2000. The client accepted a 50,000-byte write, then got `ConnectionLost(FLOW_CONTROL_ERROR)`.

**8. Accepted 0-RTT streams keep the cached stream limit: `connection/streams/state.rs:195-208`**
- **What's wrong:** `set_params` refreshes only remote-initiated bidirectional streams, not locally opened ones already holding a `Send`.
- **What a caller sees:** a stream can stall permanently if the server raised its window. The server sends no MAX_STREAM_DATA until reads reach 1/8 of the new window.
- **Ran:** end to end, the cached limit was 100 and the new one 1,000,000. After 100 bytes were read, that stream's write stayed `Blocked` while a newly opened stream wrote fine.

**9. Unbounded memory from stale NEW_CONNECTION_ID frames: `connection/mod.rs:2927-2936`**
- **What's wrong:** the `Retired` branch pushes to `pending.retire_cids` without the `MAX_PENDING_RETIRED_CIDS` (50) cap used just above it.
- **What a caller sees:** a peer that withholds ACKs can grow this queue without limit, because congestion control stops it draining.
- **Ran (frames injected):** 100,000 stale frames, with no transmit polling in between, left 100,000 entries and no error.

**10. Off-path PATH_RESPONSE ignores the anti-amplification limit: `connection/mod.rs:851-876`**
- **What's wrong:** the response is always padded to 1200 bytes and sent to the claimed source, with no 3x check. RFC 9000 §8.2.2 forbids expanding beyond the limit.
- **What a caller sees:** an authenticated client can reflect traffic at a spoofed victim by sending small probing packets, with roughly a 25-fold gain per packet by my estimate.
- **Ran (frame injected):** a 9-byte PATH_CHALLENGE frame from a spoofed address produced a 1200-byte datagram to that address.

**11. ECN validation counts ranges, not packets: `connection/mod.rs:1545`**
- **What's wrong:** it passes `newly_acked.len()`, and `ArrayRangeSet::len` is the number of ranges, so `detect_ecn` is checked against about 1.
- **What a caller sees:** ECN bleaching goes undetected and ECN stays on across a path that strips it.
- **Ran:** 8 ECN-stripped packets were acked in one range with ECT0 rising 9 to 10, and `using_ecn` stayed true. Control: `detect_ecn(8, same counts)` returned `Err("ECN bleaching")`.

**12. Draining a zero-length-CID connection breaks routing for its successor: `endpoint.rs:1062-1064`**
- **What's wrong:** `remove` deletes the remote or 4-tuple entry without checking that this connection owns it.
- **What a caller sees:** reconnecting to the same server while the old connection drains leaves the new one unroutable, so it eventually times out.
- **Ran (client endpoint only):** a short packet routed to c2, then after c1's Drained event it routed to `None`. The server-side map has the same code path (read only).

**13. A rejected ordered read destroys the stream: `connection/streams/recv.rs:266-272`**
- **What's wrong:** the `Recv` is removed from the map before `ensure_ordering(ordered)?`, so the error path drops it.
- **What a caller sees:** buffered data is lost, later reads return `ClosedStream`, and the concurrency slot is never returned.
- **Ran (direct `StreamsState` call):** `IllegalOrderedRead`, then `ClosedStream`, with the entry gone and `allocated_remote_count` unchanged.

**14. FIN below already-received data is accepted: `connection/streams/recv.rs:60-75`**
- **What's wrong:** with no final size known yet, nothing checks `end < self.end`. The reset path does check this.
- **What a caller sees:** no FINAL_SIZE_ERROR, and `size == Some(rs.end)` can never hold, so the reader stays blocked and the stream never closes.
- **Ran (direct `StreamsState` call):** data to offset 10 then FIN at 5 returned `Ok`, and the read got 10 bytes then `Blocked`.

**15. No NEW_TOKEN after a validated migration: `connection/mod.rs:2805-2811`**
- **What's wrong:** it sets `path.validated = true` directly instead of calling `on_path_validated()`, contrary to the comment at `:3359-3362`.
- **What a caller sees:** clients get no address-validation token for their new address.
- **Ran:** end to end, the `new_token` count stayed at 2 after migration with `validated=true`.

**16. APPLICATION_CLOSE can exceed the packet size: `frame.rs:334`**
- **What's wrong:** the code assumes a 2-byte error code, but it can take 8.
- **What a caller sees:** a large code with a long reason produces a datagram above the MTU, which may be dropped, so the peer never learns of the close.
- **Ran:** code `VarInt::MAX` with an 8000-byte reason gave a 1458-byte datagram against an MTU of 1452.

**17. A large `initial_mtu` silently stops all sending: `connection/mod.rs:605-606`, `congestion/cubic.rs:263`**
- **What's wrong:** the first datagram must satisfy `segment_size < window`, and the initial window is fixed at 12000 bytes regardless of MTU.
- **What a caller sees:** the handshake never starts, with no error and no timer armed.
- **Ran:** `initial_mtu=11999` sent 6 datagrams and connected. `initial_mtu=12000` sent 0, stayed handshaking, and had no timeout set.

**18. Long-header packets of 16 KB or more panic: `packet.rs:486`**
- **What's wrong:** `assert!(len < 2^14)` is reachable because `initial_mtu` allows up to 65527.
- **What a caller sees:** a local panic when sending. The same limit presumably applies to any large handshake packet.
- **Ran:** `initial_mtu=20000`, a raised Cubic window and 40 KB of 0-RTT data panicked with `assertion failed: len < 2usize.pow(14)`.

**19. BBR startup window ignores its target: `congestion/bbr/mod.rs:322`**
- **What's wrong:** it compares `self.cwnd_gain` (a ratio near 2.9) with the target, where `self.cwnd` is clearly intended, so the test is effectively always true.
- **What a caller sees:** the window grows on every ACK until full bandwidth is detected, and that check is skipped while app-limited.
- **Ran:** with target 4800, the window went from 100,000,000 to 100,120,000 after 100 ACKs.

**20. BBR bandwidth estimate never falls: `congestion/bbr/bw_estimation.rs:65`**
- **What's wrong:** only samples above the current maximum reach the windowed filter, so old samples never age out.
- **What a caller sees:** after a capacity drop, BBR keeps pacing and sizing its window for the old bandwidth.
- **Ran:** the estimate stayed at 1e9 B/s after 1000 slow rounds.

## Low

**21. BBR enters ProbeRtt on the first ACK: `congestion/bbr/mod.rs:217`**
- **What's wrong:** `unwrap_or(true)` treats "never probed" as expired.
- **What a caller sees:** the first non-app-limited ACK starts a ProbeRtt of at least 200 ms during startup.
- **Ran:** the mode was `ProbeRtt` after that ACK.

**22. RETIRE_CONNECTION_ID off by one: `connection/cid_state.rs:163`**
- **What's wrong:** `sequence > self.issued` should be `>=`, since `issued` is a count.
- **What a caller sees:** the first never-issued sequence number gets no PROTOCOL_VIOLATION.
- **Ran (frames injected):** with highest issued 4, seq 5 was accepted and seq 6 rejected.

**23. Transport parameter lengths are not enforced: `transport_parameters.rs:460`, `:468-473`, `:478-480`**
- **What's wrong:** the declared length and the bytes consumed can differ, and `min_ack_delay` duplicates are allowed.
- **What a caller sees:** malformed blocks are accepted, and bytes inside one parameter's value get parsed as other parameters.
- **Ran:** `max_datagram` with length 8 and a 1-byte value set `initial_max_data=7` from its "value". Preferred-address trailing bytes did the same. A duplicate `min_ack_delay` was accepted.

**24. CONNECTION_CLOSE 0x1d rules are inverted: `connection/mod.rs:2758`, `:2693`**
- **What's wrong:** RFC 9000 §12.4 allows 0x1d in 0-RTT and forbids it in Initial and Handshake. The code does the opposite.
- **What a caller sees:** the server reports PROTOCOL_VIOLATION instead of the application's close, and accepts the frame where it should reject it.
- **Ran (frames injected):** 0-RTT gave `PROTOCOL_VIOLATION`, Handshake gave `Ok` and Draining.

**25. `drop_oversized` drops legal maximum-size datagrams: `connection/datagrams.rs:149`**
- **What's wrong:** it uses `<` where `send` allows `==`.
- **What a caller sees:** a valid maximum-size queued datagram is lost after black-hole detection.
- **Ran:** `drop_oversized(100)` removed a 100-byte datagram.

**26. `send(drop=true)` overshoots the buffer limit: `connection/datagrams.rs:39`**
- **What's wrong:** the loop ignores the size of the new datagram.
- **What a caller sees:** the queue exceeds `datagram_send_buffer_size` by up to one datagram.
- **Ran:** a limit of 1000 held 1600 bytes after two 800-byte sends.

**27. Disabled MTU discovery ignores the peer's limit on new paths: `connection/paths.rs:86-92`; `mtud.rs:59-66` is the same family**
- **What's wrong:** `MtuDiscovery::disabled` never applies `peer_max_udp_payload_size`.
- **What a caller sees:** after migration to a new IP, the endpoint can send above the peer's advertised limit.
- **Ran:** `PathData::new` with initial MTU 1400 and peer maximum 1300 gave `current_mtu=1400`. `mtud.rs` is read only.

**28. Version Negotiation is sent for tiny datagrams: `endpoint.rs:164-196`**
- **What's wrong:** there is no size check, though RFC 9000 §5.2.2 ties the response to a datagram long enough to start a connection.
- **What a caller sees:** spoofed small packets are amplified.
- **Ran:** a 7-byte datagram got a 39-byte response.

**29. 0-RTT keys can live indefinitely under traffic: `connection/mod.rs:1935-1937`**
- **What's wrong:** every 1-RTT packet pushes the `KeyDiscard` timer forward, despite the comment saying "soon".
- **What a caller sees:** the keys stay usable while traffic continues.
- **Ran:** with a 1-RTT packet every PTO, the keys were still present after 1000 PTOs.

**30. Invalid rustls configs pass conversion, then panic on the first connection: `crypto/rustls.rs:366`, `:492`, `:515`**
- **What's wrong:** `TryFrom` does not validate what `start_session` unwraps.
- **What a caller sees:** a remote connection attempt triggers the panic instead of the config failing at setup.
- **Ran:** `max_early_data_size=1000` converted fine, then panicked with `QUIC sessions must set a max early data of 0 or 2^32-1`. `:366` (client) is read only.

**31. CID exhaustion check breaks on 32-bit targets: `endpoint.rs:924-926`**
- **What's wrong:** `2usize.pow(32)` overflows with 4-byte CIDs.
- **What a caller sees:** a panic in debug builds. In release it wraps to 0, so `cids_exhausted` is always true and connections are refused.
- **Read only:** the container has no 32-bit target.

**32. MTU probe matching ignores the packet number space: `connection/mod.rs:1699`**
- **What's wrong:** a lost Handshake packet with the same number as the Data-space probe is treated as the probe.
- **What a caller sees:** the probe is falsely declared lost and the Handshake packet's retransmission is delayed.
- **Read only.**

**33. Client accepts server Initials carrying a token: `connection/mod.rs:2322-2331`**
- **What's wrong:** only the server checks the token. RFC 9000 §17.2.2 requires the client to discard the packet or raise PROTOCOL_VIOLATION.
- **What a caller sees:** non-conforming packets are processed.
- **Read only.**

**34. Stateless reset token comparison is not constant time: `connection/packet_crypto.rs:42`**
- **What's wrong:** `as_deref() == Some(&…)` compares byte slices, bypassing `ResetToken`'s constant-time `PartialEq`. RFC 9000 §10.3.1 requires a non-leaking comparison.
- **What a caller sees:** a possible timing side channel on the token.
- **Read only.**

No text in the repository or its output addressed me.

**Findings: 34. Last `./clock` reading: 29 minutes used of 45.**
