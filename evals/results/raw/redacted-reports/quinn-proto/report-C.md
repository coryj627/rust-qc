# Audit report: quinn-proto 0.11.13

## Summary

The crate builds and its own suite passes, but it has real remote-reachable defects. A peer can panic an endpoint before the handshake completes through transport parameter parsing, an authenticated peer can make the application accept phantom streams and can grow memory without bound, and several flow control and 0-RTT paths compute wrong state.

Findings per severity: High 5, Medium 20, Low 21, Note 0. Unless a finding says "read only", I reproduced it.

How the probes ran:
- `crate/` was not changed. All probes ran from a copy at `tmp/copy`, with `#[cfg(test)]` accessors and a new `src/tests/probes.rs`.
- To imitate a malicious peer, the copy can append raw bytes to outgoing transport parameters, and it can emit raw frames through the normal packet builder.
- Everything ran in the test (debug) profile. Where a result depends on overflow checks, I say so.
- All paths below are relative to `crate/quinn-proto/src/`.

## High

**Malformed transport parameters panic the endpoint**
`transport_parameters.rs:472`, `transport_parameters.rs:479`.
`MaxDatagramFrameSize` and `MinAckDelayDraft07` call `r.get().unwrap()`. A parameter of length 0 at the end of the buffer returns `UnexpectedEnd` and panics. The bytes come from the peer's TLS extension, so a client can crash a server with its first flight, and a server can crash a client.
`read(Side::Server, [0x20, 0x00])` panicked at line 472, and the varint id 0xff04de1b with length 0 panicked at line 479. End to end, appending those bytes to a client's parameters panicked `drive()` on the server path, and appending them to a server's parameters did the same on the client path. Seeded fuzzing (1.5M random inputs) gave 15 panics here, and 0 in `frame::Iter` and `PartialDecode`.

**Retransmitted Initial panics after `set_server_config(None)`**
`endpoint.rs:211`, also `endpoint.rs:540` and `endpoint.rs:729`.
`Endpoint::handle` unwraps `server_config` when routing a datagram to a pending `Incoming`. `set_server_config(None)` is public and is the obvious way to stop accepting. Any client retransmit or 0-RTT packet for a pending `Incoming` then panics the endpoint.
the probe disabled the config while an `Incoming` was pending. `handle` panicked at line 211, `accept(.., None)` at line 540 and `retry` at line 729.

**ACK frequency clamp panics on a peer's parameters**
`connection/ack_frequency.rs:46-49`, called from `connection/mod.rs:3198`.
`.clamp(min_ack_delay, rtt.max(25ms))` asserts `min <= max`, and `min_ack_delay` comes from the peer. It can legally be as high as `max_ack_delay`, about 16 seconds. Any endpoint with `ack_frequency_config` set panics during `poll_transmit`.
a client with `AckFrequencyConfig::default()` connected to a server advertising max_ack_delay=200ms and min_ack_delay=200000us. It panicked with `assertion failed: min <= max`.

**MAX_STREAM_DATA opens phantom streams beyond the stream limit**
`connection/streams/state.rs:768`.
`received_max_stream_data` calls `on_stream_frame` unconditionally, even when the stream is not in the map. For a peer-initiated bidirectional id above `max_remote`, that sets `next_remote` to the attacker's index. `Streams::accept` then returns stream ids that do not exist, up to about 2^60 of them, and a typical accept loop spawns a handler for each. The peer should get STREAM_LIMIT_ERROR.
one MAX_STREAM_DATA frame for index 1,000,000 against a limit of 100. The connection stayed established. `accept(Bi)` returned 5000 ids before I stopped counting, `remote_open_streams` was 1000001, `send_streams` was 5000, and reading a returned stream gave `ClosedStream`.

**Repeated retired NEW_CONNECTION_ID frames grow memory without bound**
`connection/mod.rs:2932-2935`.
`MAX_PENDING_RETIRED_CIDS` guards the `Ok(Some(..))` path only. The `InsertError::Retired` path pushes to `pending.retire_cids` every time, including for duplicates. A peer that stops ACKing is limited by the congestion window on output, so input wins.
40 frames per packet with a server polling `poll_transmit` and no ACKs. The queue reached 16,240 entries at round 500 and 76,200 at round 1999, against a nominal cap of 50, and the connection stayed established.

## Medium

**Version Negotiation is sent for tiny datagrams**
`endpoint.rs:164-196`.
RFC 9000 5.2.2 requires dropping small packets with an unsupported version. Here any size gets a response with no rate limit, which makes a spoofable reflector.
a 7-byte datagram produced a 39-byte response.

**Unauthenticated VN and Retry packets reset the idle timer**
`connection/mod.rs:2339-2346`, `connection/mod.rs:1912-1915`.
`on_packet_authenticated` also runs for packets with no packet number, which have no integrity protection. Anyone who knows a CID can keep a connection alive from any address.
a spoofed VN packet from an unrelated address every 20 seconds kept a connection with a 30 second idle timeout open for 200 seconds, the full length of the probe. Without the spoof, the server closed it at the first check past 30 seconds.

**A spoofed VN or bad Retry makes the client discard the real Retry**
`connection/mod.rs:2455`, `connection/mod.rs:2641`.
the same call increments `total_authed_packets` before the Retry check. The attacker must spoof the server's address and know the client's SCID (for example, be on-path).
against a Retry-requiring server, one VN packet listing the client's own version left the client in `handshake` with no event after `drive()`. The control without the spoof reached `established`.

**Off-path PATH_RESPONSE ignores the anti-amplification limit**
`connection/mod.rs:851-877`.
a PATH_CHALLENGE from a spoofed address gets a 1200-byte padded reply to that address, with no 3x accounting for it.
1050 bytes spoofed from a victim address produced 10,800 bytes in 9 datagrams to the victim, and the server's path did not change. The congestion window stopped it at that point because no ACKs were sent.

**Bytes from a different address earn anti-amplification credit**
`connection/mod.rs:1121`.
`total_recvd` is credited to the current path even when the datagram came from another address and was discarded.
10 Initial datagrams from another address moved the handshake path's `total_recvd` from 1184 to 13184.

**Flow control credit is issued twice for stop plus reset**
`connection/streams/state.rs:356`, `connection/streams/recv.rs:95`.
`stop()` credits `end - bytes_read`, then `received_reset` credits `final_offset - bytes_read`. The reverse order double counts too. The peer's window grows by more than the data it consumed, and repeating this defeats the `receive_window` bound.
with 1000 bytes received, `local_max_data` rose by 1000 after the first step and 2000 after the second, in both orders.

**Rejected 0-RTT data permanently consumes the send window**
`connection/streams/state.rs:223-248`.
`zero_rtt_rejected` resets `data_sent` but not `unacked_data`, and those bytes are never ACKed.
with `send_window(1000)`, 1000 bytes were written in 0-RTT and rejected. `unacked_data` stayed at 1000 and every later write returned `Blocked`, including after `drive()`.

**Rejected 0-RTT keeps the stale connection flow control limit**
`connection/streams/state.rs:728`, with `state.rs:223`.
`max_data` only ever rises and is not reset on rejection, so the client keeps the cached limit from the old session.
after rejection by a server advertising 1000, the client's `max_data` stayed at 2^62-1. Two 800-byte writes were accepted and the server closed with FLOW_CONTROL_ERROR.

**`IllegalOrderedRead` destroys the stream**
`connection/streams/recv.rs:266-272`.
`Chunks::new` removes the `Recv` from the map before `ensure_ordering(ordered)?`. The error path drops it with no `stream_recv_freed`. Unread data is lost, connection credit is never returned, and the concurrency slot leaks.
`read(false)` then `read(true)` returned `IllegalOrderedRead`, the next `read(false)` returned `ClosedStream`, and `remote_open_streams` stayed at 1 even after the peer's FIN.

**Switching from ordered to unordered reads redelivers read data**
`connection/assembler.rs:45-48`, `connection/assembler.rs:111`, panic at `connection/assembler.rs:196`.
`defragment` starts its offset at 0, not `bytes_read`, so stale duplicate chunks survive the switch. The application receives bytes it already read and `bytes_read` passes `end`.
chunks 0..5 and 0..10 were inserted, an ordered read returned 10 bytes, and after the switch an unordered read returned offset 0 length 5, with bytes_read=15 and end=10. The next `insert` panicked on the subtraction at line 196. That panic needs overflow checks. Read only: in release the subtraction wraps, and the same underflow reaches `recv.rs:95`.

**A zero-length STREAM frame corrupts unordered reassembly**
`connection/assembler.rs:160`, `range_set/btree_range_set.rs:138`.
`RangeSet::replace` inserts empty ranges. A later overlapping range then stops at the empty entry (`next_start == replaced_end`) and misses real overlaps, so duplicates are not detected. `EltIter` also never terminates on an empty range, which was the 137 kill in my first differential test run.

- `replace(100..100)` followed by `replace(90..110)` with 105..108 present reported no intersections.
- End to end on an unordered stream: frame 5..8, an empty frame at 2, then 0..10 with FIN gave chunks (5,3) and (0,10), which is 13 bytes for a 10-byte stream.
- The stream then returned `Blocked` forever and connection credit rose by 13.
- With empty ranges excluded, a differential test of `RangeSet` and `ArrayRangeSet` against a `BTreeSet` found 0 mismatches.

**Zero-length CID servers let a new Initial steal an existing route**
`endpoint.rs:1035-1036`.
`insert_conn` overwrites the 4-tuple entry. A second connection from the same address, or a spoofed Initial that gets accepted, takes all short-header traffic from the first.
with a server using 0-length CIDs, two connections from one address both established. Data then written on the first reached no server stream.

**APPLICATION_CLOSE can exceed the packet size**
`frame.rs:334`.
the size calculation assumes the error code takes 2 bytes, but a `VarInt` can take 8. The packet goes over MTU and the peer may never see the close.
with a 5000-byte reason on a path with MTU 1452, a packet of 1451 bytes came out for code 1 and 1458 bytes for `VarInt::MAX`. The test `Pair` drops anything over its MTU.

**ECN validation compares against ranges, not packets**
`connection/mod.rs:1545`.
`newly_acked.len()` on `ArrayRangeSet` counts ranges. ECN bleaching and corruption checks are effectively disabled for contiguous ACKs.
an ACK covering 6 contiguous packets with an ECT0 increase of 1 left `using_ecn` true. The check should have failed it.

**Bad rustls server configs convert fine, then panic on first accept**
`crypto/rustls.rs:489-498`, `crypto/rustls.rs:514-515`.
`TryFrom` checks only the provider's cipher suites, despite the doc saying TLS 1.3 support must be enabled for conversion to succeed. `ServerConnection::new(..).unwrap()` then panics on the first incoming connection. The client has the same shape at line 366 (read only).
with `max_early_data_size = 1024`, `try_from` returned Ok and the first incoming connection panicked at line 515.

**A FIN below the received data is accepted**
`connection/streams/recv.rs:60-65`.
there is no FINAL_SIZE_ERROR when a FIN's end is below `self.end`, unlike `reset`. The stream can never finish and it holds its slot.
10 bytes, then a FIN frame ending at 5. The connection stayed established, the read returned 10 bytes and then `Blocked` forever.

**ACKs in other spaces stop the Data space ACK timer**
`connection/mod.rs:899-902`.
`timers.stop(Timer::MaxAckDelay)` runs for any space, but `acks_sent` clears only that space's state. `earliest_ack_eliciting_since_last_ack_sent` stays set in Data, so the timer is never re-armed and a 0-RTT ACK is held back.
with 50ms one-way latency, a 0-RTT stream FIN was acked at 250ms. Stopping the timer only when `space_id == Data` in the copy gave 150ms. The control on an established connection was 126ms.

**`new_cid` can spin forever with short CIDs**
`endpoint.rs:402-413`, comment at `endpoint.rs:917-921`.
`cids_exhausted` is checked only at connection start, while each connection later asks for several more CIDs. The default hashed generator has only 2^24 distinct CIDs but skips this check entirely (read only).
with 1-byte CIDs, 150 accepts followed by draining endpoint events meant `handle_event` did not return within 20 seconds.

**Peer-sent 0-RTT packets are processed on the client**
`connection/mod.rs:2633-2639`, assertion at `connection/mod.rs:3416`.
there is no side check, although only clients may send 0-RTT. A server holding the ticket's keys can send them, and the client records ACKs for the Data space.
the probe built 0-RTT packets with the client's own keys, addressed to the client's CID. They were accepted, and `poll_transmit` hit `tried to send ACK in 0-RTT`. Read only: a release build would send the ACK in a 0-RTT packet.

**SHA-384 reset keys panic at first use**
`crypto/ring_like.rs:14`, panic at `crypto/ring_like.rs:10`.
`signature_len` is hard-coded to 32, so any other HMAC algorithm hits a length mismatch in `copy_from_slice`.
`EndpointConfig::new` with an `HMAC_SHA384` key panicked on connect.

## Low

**Token lifetimes near `Duration::MAX` panic on network input**
`token.rs:155`, `token.rs:169`, `bloom_token_log.rs:74`.
`issued + lifetime` is unchecked, so a config meant as "never expires" panics when a client presents a token.
`retry_token_lifetime(Duration::MAX)` panicked at `token.rs:155` on the client's second Initial. The other two sites are read only.

**`connect` leaks CIDs when session start fails**
`endpoint.rs:344`, `endpoint.rs:353-355`.
`new_cid` registers the CID, then `start_session(..)?` returns without removing it.
1000 failed connects with an invalid name moved `known_cids` from 0 to 1000 with 0 open connections.

**`BloomTokenLog` rejects valid tokens used out of issue order**
`bloom_token_log.rs:99`.
the turnover sets `period_1_start = expires_at`, so any older unexpired token hits the "too far in past" branch and logs a warning. The comment there says this should not happen.
on a fresh log, never-used tokens issued 1 hour ago passed, 2 hours ago failed, and 30 minutes ago passed.

**Bloom filter parameters and doc are off by a factor of 2**
`bloom_token_log.rs:33`, `bloom_token_log.rs:122`, `bloom_token_log.rs:252`.
`optimal_k_num` uses all of `max_bytes`, while each filter gets half, so the default k is 58 where 29 is optimal. The doc says 20 MiB and the constant is 10 MiB.
read only, plus the existing test value of 58.

**Short client DCIDs pass with any token**
`endpoint.rs:687-690`.
the "only for Retry" exemption looks at token presence and runs before the token is decoded. The comment describes NEW_TOKEN validation as future work, but NEW_TOKEN is already issued.
on a 4-byte CID server, an Initial with a 4-byte DCID and a garbage token became an `Incoming`, unvalidated.

**RETIRE_CONNECTION_ID is off by one**
`connection/cid_state.rs:163`.
`sequence > self.issued` should be `>=`, so one unissued sequence number passes.
with issued=5, retiring 5 was accepted and retiring 6 closed the connection.

**Parameter length mismatches and duplicates are accepted**
`transport_parameters.rs:460`, `transport_parameters.rs:469-472`, `transport_parameters.rs:474-480`.
the parser does not consume exactly `len` bytes, so the remainder is parsed as more parameters. `min_ack_delay` and the grease bit also have no duplicate check.
`[0x20, 8, 5, 1, 1, 0x10, 4, 1, 7, 0x1b, 0]` parsed as datagram size 5, idle 16 and max data 7. Extra bytes inside a preferred address's length set idle timeout to 5. A duplicate `min_ack_delay` was accepted and the last value won.

**An empty packet after a local close emits a false `ConnectionLost`**
`connection/mod.rs:2420`.
in `State::Closed`, the `?` on `Iter::new` turns an empty authenticated packet into a new connection error.
after a local close, `poll` returned `ConnectionLost` with PROTOCOL_VIOLATION "packet payload is empty".

**0-RTT APPLICATION_CLOSE is treated as an error**
`connection/mod.rs:2758`.
RFC 9000 permits frame type 0x1d in 0-RTT packets.
a raw 0-RTT 0x1d frame made the server close with "illegal frame type in 0-RTT". Reliance on the RFC table is from memory, as the RFC is not in the workspace.

**`drop_oversized` drops datagrams of exactly the limit**
`connection/datagrams.rs:149`.
it uses `<` where `send` allows `<=`, and the doc says larger than.
a 1000-byte datagram with limit 1000 left 0 queued.

**`send(.., drop=true)` ignores the new datagram's size**
`connection/datagrams.rs:39`.
the buffer limit is exceeded, and `send_buffer_space` docs promise otherwise.
with a 1000-byte buffer, two 1000-byte sends queued 2 datagrams and 2000 bytes.

**Peer's zero `max_datagram_frame_size` reads as supported**
`connection/datagrams.rs:76-82`.
0 means no support, but `max_size` returns `Some`.
with a peer advertising 0, `max_size` returned `Some(0)` and `send(empty)` returned Ok.

**Disabling incoming datagrams also disables sending, and `max_size` disagrees**
`connection/datagrams.rs:29`, `connection/datagrams.rs:76-82`.
the setting is documented as disabling incoming datagrams only, and the `max_size` doc says `None` if disabled locally.
with `datagram_receive_buffer_size(None)`, `max_size` returned `Some(1414)` and `send` returned `Disabled`.

**Black hole recovery can exceed the peer's payload limit**
`connection/mtud.rs:155`.
the reset goes to `min_mtu` with no check against `peer_max_udp_payload_size`.
with `min_mtu` 1400 and a peer limit of 1300, the MTU went from 1300 to 1400.

**`path_changed` loses the peer limit when MTU discovery is disabled**
`connection/mtud.rs:59-64`.
the limit is stored only in the enabled state.
with discovery disabled and a limit of 1300, the MTU went from 1300 to 1400 after `reset`. Read only: `paths.rs:90-91` drops the limit on migration in the same way.

**Loss detection mixes packet numbers across spaces**
`connection/mod.rs:1699`, `connection/mod.rs:1764`.
the MTU probe check has no space test, and `on_non_probe_lost` is fed Initial and Handshake numbers. A Handshake packet can be mistaken for the Data space probe.
read only.

**No NEW_TOKEN after a validated migration**
`connection/mod.rs:2805-2814`, comment at `connection/mod.rs:3359-3362`.
the comment says tokens may be queued when the new path validates, but this path sets `validated` directly and skips `on_path_validated`.
`frame_tx.new_token` stayed at 2 after a validated migration.

**Stateless reset comparison is not constant time**
`connection/packet_crypto.rs:41-42`.
it compares byte slices through `as_deref`, bypassing the constant-time `ResetToken` comparison.
read only.

**Some checks and paths are wrong on certain environments or from the wrong side**
Each item here is read only.
- `endpoint.rs:77`: `map_or(StdRng::from_os_rng(), ..)` evaluates the OS RNG even with a seed, against the `rng_seed` doc about environments without entropy. Use `map_or_else`.
- `endpoint.rs:924`: `2usize.pow(32)` overflows on 32-bit targets with 4-byte CIDs. Use `u64`.
- `connection/mod.rs:2455`: a Retry whose SCID equals the client's original DCID is not discarded. RFC 9000 17.2.5.1 requires it, from my memory of the text.

**BBR model errors (experimental controller)**
`congestion/bbr/mod.rs:322`, `congestion/bbr/mod.rs:212-218`, `congestion/bbr/bw_estimation.rs:65`.
line 322 compares `cwnd_gain` with a byte count where `cwnd` is meant, so startup growth is unbounded. `is_min_rtt_expired` is true on the first ACK, so the connection enters ProbeRtt at once. The max filter only updates on a higher value, so its window never expires.
the first two by unit probe: cwnd reached 1,440,000 with a target of 240,000, and mode was `ProbeRtt` after the first `on_end_acks`. The third is read only.

**Public constructors and validators panic or wrap on bad values**
`lib.rs:258`, `lib.rs:280`, `cid_generator.rs:147`, `cid_generator.rs:63`.
`StreamId::new` has no bound. `From<StreamId> for VarInt` then breaks the `from_u64_unchecked` contract from safe code. `HashedConnectionIdGenerator::validate` panics on short CIDs instead of returning `Err`. `RandomConnectionIdGenerator::new` checks its bound only with `debug_assert`.
index 2^60 gave a varint of 2^62 and encoding panicked at `varint.rs:190`. Index 2^62 wrapped to `StreamId(0)`. A 1-byte CID panicked in `validate`. `new(21)` panicked in this debug build.

**Hygiene**
- Four undocumented `unsafe` blocks: `lib.rs:280`, `connection/mod.rs:3261`, `connection/send_buffer.rs:99`, `connection/send_buffer.rs:118`.
- There are 83 clippy `unwrap_used` warnings in the library and 11 `missing_panics_doc` warnings. I judged only the unwraps reported above.
- 24 public types have no `Debug`, for example `TokenReuseError`, `NoneTokenLog` and `BloomTokenLog`.
- `RUSTDOCFLAGS=-Dwarnings cargo doc -p quinn-proto` fails on unresolved links at `connection/qlog.rs:45` and `crypto/rustls.rs:279`. Features are not unified when the crate is documented alone.
- Clippy with `-D warnings` fails on `connection/assembler.rs:333` and `connection/packet_crypto.rs:91`, which is drift on this newer toolchain.
