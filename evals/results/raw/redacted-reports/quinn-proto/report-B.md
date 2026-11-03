# Audit report: quinn-proto 0.11.13

Baseline: `cargo test --locked -p quinn-proto` passes (263 unit tests, 3 doc tests).

**How the probes ran.** `crate/` was not changed. All probes ran in the container on a scratch copy at `tmp/p/crate`, as test code in `quinn-proto/src/tests/probe.rs`, with `#[cfg(test)]` hooks added to that copy:
- `probe_inject` feeds a decrypted frame payload to `Connection::process_payload`, as if an authenticated peer had sent it.
- `TP_HOOK` replaces the client's transport-parameter bytes in `crypto/rustls.rs::to_vec`.

"Debug" and "release" below mean `cargo test` and `cargo test --release`.

I also ran a structured frame and API fuzzer for 36,000 seeds; see the notes at the end.

## High

**1. Malformed transport parameters panic the endpoint, before authentication**
- **Where:** `transport_parameters.rs:472`, `:479`
- **Wrong:** `max_datagram_frame_size` and `min_ack_delay` use `r.get().unwrap()` with no check that a value is present.
- **Effect:** `[0x20, 0x00]` as the last parameter panics inside `TransportParameters::read`. It is called on servers from the client's first Initial flight (`connection/mod.rs:2621`) and on clients from the server's parameters (`:2548`). One unauthenticated Initial crashes a server thread or process.
- **Established:**
  - Unit calls with empty values for both parameters panicked with `UnexpectedEnd`, at `:472` and `:479`.
  - End to end, a client whose parameters end in `20 00` panicked the server during the handshake (debug).

**2. A long Initial token makes the client loop and allocate until it runs out of memory**
- **Where:** `connection/mod.rs:2512` and `:2959` (no token length limit), loop at `:525`, `:571`, `:3248`; `packet_builder.rs:155`
- **Wrong:** The header can outgrow the 1200-byte datagram. CRYPTO then never fits, the space stays sendable, `buf_capacity - buf_end` wraps, and each pass appends another oversized Initial with no progress.
- **Effect:**
  - Any server, or anyone who sees the client's Initial, can send a Retry with a token of about 1300 bytes. The Retry integrity key is public.
  - A server can also do this through NEW_TOKEN, which takes effect on the next connect.
  - In release, `poll_transmit` never returns and the process aborts on allocation failure. In debug it panics at `packet_builder.rs:156`.
- **Established:**
  - A 1300-byte token from a `TokenStore`, and separately a forged Retry with a valid tag, both gave `memory allocation of 1160 bytes failed` and `Aborted` under `ulimit -v 1200000` (release).
  - A 1100-byte token works (1200-byte datagram). I did not narrow the exact threshold.

**3. A peer's `min_ack_delay` can panic `candidate_max_ack_delay`**
- **Where:** `connection/ack_frequency.rs:49`
- **Wrong:** `.clamp(min_ack_delay, rtt.max(25ms))` panics when the minimum exceeds the maximum. The peer sets the minimum, up to its `max_ack_delay` (below 16 s), and `read` accepts it.
- **Effect:** On an endpoint with `ack_frequency_config` set (not the default), the peer panics `poll_transmit` during the handshake.
- **Established:** A client advertising `max_ack_delay=100` and `min_ack_delay=100000` panicked the server with `assertion failed: min <= max`, end to end.

**4. FIN below the highest received offset is accepted, leading to a stuck stream, a panic, or a flow-control bypass**
- **Where:** `connection/streams/recv.rs:60-65`, `:316`; `streams/state.rs:355`
- **Wrong:** The final-size check only runs once a final size is known, so data to offset 110 followed by FIN at 10 raises no `FINAL_SIZE_ERROR`.
- **Effect:**
  - Reads never finish, because `size != end`. The probe read 110 bytes, then got `Err(Blocked)`.
  - A following RESET_STREAM evaluates `final_offset - end`, which underflows. Debug builds panic with `attempt to subtract with overflow`.
  - In release, `data_recvd` saturates to `u64::MAX`. `received + new_bytes` at `recv.rs:218` then wraps, which disables connection flow control.
- **Established:** Probes in debug and release. In release, 17,100 new bytes were accepted over a 1000-byte connection window, and all 19 extra streams got through.

**5. MAX_STREAM_DATA beyond the stream limit creates phantom incoming streams**
- **Where:** `connection/streams/state.rs:745-768`; `streams/mod.rs:67-79`
- **Wrong:** A remote-initiated ID at or above `max_remote` is not in the map and raises no `STREAM_LIMIT_ERROR`. `on_stream_frame` still sets `next_remote = id + 1` and flags `Opened`.
- **Effect:** One frame makes `accept()` return up to 2^60 IDs that do not exist, past the concurrency limit. Reads on them give `ClosedStream`, and `send_streams` inflates. An accept loop spins and allocates per stream.
- **Established:** Injected index 1,000,000 with a limit of 100. The frame returned `Ok`, `accept` yielded at least 5000 streams (the probe stopped there), and `send_streams` was 5000.

## Medium

**6. Connection flow-control credit is issued twice**
- **Where:** `streams/state.rs:356`; `recv.rs:95`; `streams/mod.rs:159`
- **Wrong:** Stop followed by RESET credits `final - bytes_read` after `stop()` already credited `end - bytes_read`. RESET followed by stop does the same in the other order.
- **Effect:** A peer inflates `local_max_data` beyond what was consumed, each time the application stops a stream.
- **Established:** 1000 bytes on a stream produced 2000 bytes of credit in both orders.

**7. Off-path PATH_RESPONSE ignores the anti-amplification limit**
- **Where:** `connection/mod.rs:850-877`, pad at `:859`
- **Wrong:** The response is padded to 1200 bytes to any claimed source address, with no per-address budget. RFC 9000 §8.2.2 forbids padding past the limit.
- **Effect:** An authenticated peer can reflect traffic at a spoofed victim.
- **Established:** 50 injected challenges produced 9 datagrams and 10,800 bytes to an unvalidated address, at a single instant.

**8. Version Negotiation answers tiny datagrams with no rate limit**
- **Where:** `endpoint.rs:164-196`
- **Wrong:** There is no datagram-size check or rate limit, unlike stateless reset. RFC 9000 §5.2.2 ties the response to a datagram large enough to start a connection.
- **Effect:** Spoofed small packets are amplified about 5.6 times.
- **Established:** A 7-byte datagram drew a 39-byte response, and 100 of 100 repeats at the same instant were answered.

**9. The retired-CID queue is unbounded**
- **Where:** `connection/mod.rs:2927-2936`
- **Wrong:** The `InsertError::Retired` arm pushes to `pending.retire_cids` without the `MAX_PENDING_RETIRED_CIDS` check used just above.
- **Effect:** Repeating one NEW_CONNECTION_ID with a retired sequence number grows memory without bound. Each entry is also resent as a frame.
- **Established:** 10,000 frames gave 10,005 queued entries, against the code's own limit of 50.

**10. Zero-length DATAGRAM frames bypass the receive buffer limit**
- **Where:** `connection/datagrams.rs:132-139`
- **Wrong:** The limit counts payload bytes only, so empty datagrams never evict anything.
- **Effect:** Memory grows without bound if the application does not drain datagrams, and they are enabled by default. From reading: small non-empty datagrams also pin their whole packet buffer, which the limit does not count.
- **Established:** 600,000 entries queued with `recv_buffered=0` and a 100-byte window.

**11. 0-RTT rejection leaks `unacked_data`**
- **Where:** `streams/state.rs:223-248`
- **Wrong:** `data_sent` is reset but `unacked_data` is not, and the discarded packets never get acknowledged.
- **Effect:** The send window shrinks permanently by every byte written in rejected 0-RTT.
- **Established:** With `send_window=1000`, after writing 1000 bytes and being rejected, a new stream's write returned `Err(Blocked)` with `unacked_data=1000`.

**12. 0-RTT rejection keeps the cached `max_data`**
- **Where:** `streams/state.rs:223-248`, `:727-729`
- **Wrong:** `received_max_data` only raises the value, so a lower limit in the real handshake is ignored.
- **Effect:** The client overruns the server's limit and the connection dies.
- **Established:** With a server window of 1000, the client kept `max_data=2^62-1`, wrote 5000 bytes, and the server closed with `FLOW_CONTROL_ERROR`.

**13. An empty FIN sent in 0-RTT is lost across Retry**
- **Where:** `streams/state.rs:693`
- **Wrong:** `is_fully_acked() && !fin_pending` skips a stream with no data whose FIN was already sent, and the 0-RTT packets are discarded.
- **Effect:** The stream never reaches the server.
- **Established:** With a Retry, `open` plus `finish` never reached the server's `accept`. The control with one byte of data worked.

**14. A rejected ordered read destroys the stream**
- **Where:** `streams/recv.rs:266-272`; `streams/state.rs:232`
- **Wrong:** `entry.remove()` runs before `ensure_ordering(..)?`, so the error drops the `Recv` with no cleanup.
- **Effect:**
  - Further reads return `ClosedStream`, where unordered reads should still work.
  - Stream and flow-control credit are never released.
  - On a 0-RTT bidirectional stream, a later rejection panics at `self.recv.remove(&id).unwrap()`.
- **Established:** Both probes (`Err(ClosedStream)`, and the panic).

**15. An unauthenticated Retry closes a server connection during its handshake**
- **Where:** `connection/mod.rs:2451-2453`
- **Wrong:** It returns `PROTOCOL_VIOLATION` instead of discarding the packet. The same code discards spoofable Initials for this reason (`:2325-2329`).
- **Effect:** An attacker who knows the server's CID, for example an on-path observer, and spoofs the client's address can kill the handshake.
- **Established:** A garbage Retry through `Endpoint::handle` gave `ConnectionLost(PROTOCOL_VIOLATION "client sent Retry")`.

**16. ECN validation counts ACK ranges instead of packets**
- **Where:** `connection/mod.rs:1545`; `range_set/array_range_set.rs:55`
- **Wrong:** `newly_acked.len()` is the number of ranges.
- **Effect:** ECN bleaching and corruption go undetected, so congestion signals can be lost.
- **Established:** 9 packets acknowledged in one range with an ECT(0) increase of 1 left ECN enabled.

## Low

**17. Transport-parameter length and duplicate checks are missing**
- **Where:** `transport_parameters.rs:456-480`
- **Wrong:** The declared `len` is not enforced for the datagram and `min_ack_delay` parameters. From reading, the same applies to bytes left over after `PreferredAddress`. Duplicate `grease_quic_bit` and `min_ack_delay` are accepted, which RFC 9000 §7.4 forbids.
- **Established:** `20 04 05 01 01 07` parsed as datagram size 5 plus `max_idle_timeout=7`. The duplicates were accepted.

**18. An application close can exceed the MTU**
- **Where:** `frame.rs:334`
- **Wrong:** The code reserves 3 bytes for the type and error code, but the code can take 8.
- **Effect:** With a large code and a long reason, the datagram is oversized and may be dropped, so the peer may never see the close.
- **Established:** A 1458-byte datagram on a 1452-byte MTU.

**19. An oversized queued datagram blocks the queue after an MTU drop**
- **Where:** `connection/datagrams.rs:172-177`; `drop_oversized` is only called at `connection/mod.rs:1773`
- **Wrong:** `path_changed` lowers the MTU without dropping oversized datagrams. From reading, migration does too.
- **Effect:** Datagrams queued behind the oversized one are not sent.
- **Established:** After `path_changed` on a 1200-byte path, nothing was delivered, and 1424 bytes stayed queued, including a 10-byte datagram.

**20. The datagram send buffer limit can be overrun**
- **Where:** `connection/datagrams.rs:39`
- **Wrong:** The drop loop ignores the size of the new datagram.
- **Established:** `outgoing_total=1000` with a limit of 100, and no older datagram was dropped.

**21. `drop_oversized` drops exact fits**
- **Where:** `connection/datagrams.rs:149`
- **Wrong:** It uses `<` where `send` accepts `<=`.
- **Established:** A datagram of exactly `max_size` (1414 bytes) was dropped.

**22. A failed `connect` leaks a CID index entry**
- **Where:** `endpoint.rs:344-355`
- **Wrong:** `new_cid` inserts into the index before `start_session(..)?`, and the error path does not remove it. The entry points at an unused or later-reused handle.
- **Established:** 100 connects with an invalid server name left 100 entries in `known_cids`.

**23. No NEW_TOKEN after path validation**
- **Where:** `connection/mod.rs:2806-2810`
- **Wrong:** It sets `validated` directly instead of calling `on_path_validated`, despite the comment at `:3359-3362`.
- **Established:** `new_token` count was 2 before and after a completed migration.

**24. RETIRE_CONNECTION_ID off by one**
- **Where:** `connection/cid_state.rs:163`
- **Wrong:** `sequence > issued` should be `>=`.
- **Established:** Sequence 5 was accepted with 5 issued (0 to 4). RFC 9000 requires an error.

**25. Zero-length CIDs: draining an old connection unroutes a new one**
- **Where:** `endpoint.rs:1062-1064`
- **Wrong:** The route is removed by 4-tuple without checking that it belongs to the draining connection.
- **Effect:** After a client restart on the same port, the new connection loses routing when the old one drains.
- **Established:** Routing went from `Some(new handle)` to `None`.

**26. Fixed bit cleared on Initials without a token**
- **Where:** `connection/packet_builder.rs:127`
- **Wrong:** Parameters cached for 0-RTT turn on greasing, including for Initials with no token. RFC 9287 §3 allows that only with a NEW_TOKEN token.
- **Effect:** A strict server would drop those Initials.
- **Established:** 27 of 39 resumed Initials had the bit cleared with `NoneTokenStore`.

**27. `initial_mtu` of about the initial window or more never sends**
- **Where:** `connection/mod.rs:605-606`; `congestion/cubic.rs:79`, `:263`
- **Wrong:** With the default Cubic window of 12,000 bytes, the first datagram's reserved size fails the congestion check permanently.
- **Effect:** The handshake never starts, and nothing reports why.
- **Established:** `initial_mtu(15000)` gave `poll_transmit` `None` ("blocked by congestion control" in the trace at `initial_mtu(20000)`).

**28. Long-header packets of 16 KB or more panic**
- **Where:** `packet.rs:486`, with the 2-byte placeholder at `:301` and `:320`
- **Wrong:** The `len < 2^14` assertion is reachable when the MTU exceeds 16 KB.
- **Established:** A 20,000-byte MTU, a raised initial window and a 40 KB 0-RTT write panicked in `poll_transmit`.

**29. Invalid TLS configs pass conversion, then panic at runtime**
- **Where:** `crypto/rustls.rs:514-515`; `:494-503` (`TryFrom` checks only the cipher suite). By reading, the client path at `:366` has the same `unwrap`.
- **Effect:** A remote Initial triggers the panic.
- **Established:** With `max_early_data_size=1000`, the conversion succeeded and the first Initial panicked.

**30. BBR startup compares the wrong variable**
- **Where:** `congestion/bbr/mod.rs:322`
- **Wrong:** `self.cwnd_gain < target_window` should use `self.cwnd`, so the condition is effectively always true and startup cwnd is not bounded by the target.
- **Established:** With cwnd at 2.4 MB and a target of 240 KB, cwnd still grew by `bytes_acked`.

**31. Stateless-reset token compared in variable time**
- **Where:** `connection/packet_crypto.rs:41-42`
- **Wrong:** `as_deref()` compares plain slices and bypasses `ResetToken`'s constant-time `PartialEq`. RFC 9000 §10.3.1 requires a comparison that does not leak the token.
- **Established:** Code reading only.

**32. MTU probe check ignores the packet number space**
- **Where:** `connection/mod.rs:1699`, `:1793`
- **Wrong:** A lost Initial or Handshake packet with the same number as the in-flight Data probe is treated as the probe. The real probe is removed as lost and the other packet stays unretransmitted.
- **Established:** Code reading only.

## Notes

**Fuzzing:**
- 36,000 seeds at 200 steps each found no new panic in release-relevant code.
- The only other repeatable hit was the `debug_assert` at `connection/mod.rs:1811`, three times. Direct reproductions failed (two migration probes passed), so it is not counted.
- An earlier run hit `send_streams` underflow at `state.rs:943`, in a fuzzer version that wrote to a stream without calling `accept` first. I treated it as harness misuse and do not report it. It did not recur after the fuzzer was restricted to accepted streams.

**Repository text:** nothing in the repository or its output addressed me.

**Scratch files:** the probes are in `<workspace>/tmp/p/crate/quinn-proto/src/tests/probe.rs`.

**Findings: 32. Last `./clock` reading: 27 minutes used of 45.**
