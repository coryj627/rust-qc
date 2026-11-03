# Audit report: quinn-proto 0.11.13

## Verdict

The crate builds and its own suite passes, but it has real remote-reachable defects. A peer can panic an endpoint before the handshake completes through transport parameter parsing, an authenticated peer can make the application accept phantom streams and can grow memory without bound, and several flow control and 0-RTT paths compute wrong state.

Findings per severity: Blocker 5, Major 20, Minor 21, Note 0. Unless a finding says "read only", I reproduced it.

How the probes ran:
- `crate/` was not changed. All probes ran from a copy at `tmp/copy`, with `#[cfg(test)]` accessors and a new `src/tests/probes.rs`.
- To imitate a malicious peer, the copy can append raw bytes to outgoing transport parameters, and it can emit raw frames through the normal packet builder.
- Everything ran in the test (debug) profile. Where a result depends on overflow checks, I say so.
- All paths below are relative to `crate/quinn-proto/src/`.

## Blocker

### Malformed transport parameters panic the endpoint
Location: `transport_parameters.rs:472`, `transport_parameters.rs:479`.
Problem: `MaxDatagramFrameSize` and `MinAckDelayDraft07` call `r.get().unwrap()`. A parameter of length 0 at the end of the buffer returns `UnexpectedEnd` and panics. The bytes come from the peer's TLS extension, so a client can crash a server with its first flight, and a server can crash a client.
Established: `read(Side::Server, [0x20, 0x00])` panicked at line 472, and the varint id 0xff04de1b with length 0 panicked at line 479. End to end, appending those bytes to a client's parameters panicked `drive()` on the server path, and appending them to a server's parameters did the same on the client path. Seeded fuzzing (1.5M random inputs) gave 15 panics here, and 0 in `frame::Iter` and `PartialDecode`.
Fix: read through `?`, and require `len == value.size()`.

### Retransmitted Initial panics after `set_server_config(None)`
Location: `endpoint.rs:211`, also `endpoint.rs:540` and `endpoint.rs:729`.
Problem: `Endpoint::handle` unwraps `server_config` when routing a datagram to a pending `Incoming`. `set_server_config(None)` is public and is the obvious way to stop accepting. Any client retransmit or 0-RTT packet for a pending `Incoming` then panics the endpoint.
Established: the probe disabled the config while an `Incoming` was pending. `handle` panicked at line 211, `accept(.., None)` at line 540 and `retry` at line 729.
Fix: drop the datagram when no config exists, and return an error from `accept` and `retry`.

### ACK frequency clamp panics on a peer's parameters
Location: `connection/ack_frequency.rs:46-49`, called from `connection/mod.rs:3198`.
Problem: `.clamp(min_ack_delay, rtt.max(25ms))` asserts `min <= max`, and `min_ack_delay` comes from the peer. It can legally be as high as `max_ack_delay`, about 16 seconds. Any endpoint with `ack_frequency_config` set panics during `poll_transmit`.
Established: a client with `AckFrequencyConfig::default()` connected to a server advertising max_ack_delay=200ms and min_ack_delay=200000us. It panicked with `assertion failed: min <= max`.
Fix: compute `max(min_ack_delay, min(requested, upper))` rather than using `clamp`.

### MAX_STREAM_DATA opens phantom streams beyond the stream limit
Location: `connection/streams/state.rs:768`.
Problem: `received_max_stream_data` calls `on_stream_frame` unconditionally, even when the stream is not in the map. For a peer-initiated bidirectional id above `max_remote`, that sets `next_remote` to the attacker's index. `Streams::accept` then returns stream ids that do not exist, up to about 2^60 of them, and a typical accept loop spawns a handler for each. The peer should get STREAM_LIMIT_ERROR.
Established: one MAX_STREAM_DATA frame for index 1,000,000 against a limit of 100. The connection stayed established. `accept(Bi)` returned 5000 ids before I stopped counting, `remote_open_streams` was 1000001, `send_streams` was 5000, and reading a returned stream gave `ClosedStream`.
Fix: validate with `validate_receive_id` and only call `on_stream_frame` when the stream exists.

### Repeated retired NEW_CONNECTION_ID frames grow memory without bound
Location: `connection/mod.rs:2932-2935`.
Problem: `MAX_PENDING_RETIRED_CIDS` guards the `Ok(Some(..))` path only. The `InsertError::Retired` path pushes to `pending.retire_cids` every time, including for duplicates. A peer that stops ACKing is limited by the congestion window on output, so input wins.
Established: 40 frames per packet with a server polling `poll_transmit` and no ACKs. The queue reached 16,240 entries at round 500 and 76,200 at round 1999, against a nominal cap of 50, and the connection stayed established.
Fix: apply the same limit here, or dedupe the sequence numbers.

## Major

### Version Negotiation is sent for tiny datagrams
Location: `endpoint.rs:164-196`.
Problem: RFC 9000 5.2.2 requires dropping small packets with an unsupported version. Here any size gets a response with no rate limit, which makes a spoofable reflector.
Established: a 7-byte datagram produced a 39-byte response.
Fix: respond only when `datagram_len >= MIN_INITIAL_SIZE`.

### Unauthenticated VN and Retry packets reset the idle timer
Location: `connection/mod.rs:2339-2346`, `connection/mod.rs:1912-1915`.
Problem: `on_packet_authenticated` also runs for packets with no packet number, which have no integrity protection. Anyone who knows a CID can keep a connection alive from any address.
Established: a spoofed VN packet from an unrelated address every 20 seconds kept a connection with a 30 second idle timeout open for 200 seconds, the full length of the probe. Without the spoof, the server closed it at the first check past 30 seconds.
Fix: do not call it when `number` is `None`, and count VN and Retry separately.

### A spoofed VN or bad Retry makes the client discard the real Retry
Location: `connection/mod.rs:2455`, `connection/mod.rs:2641`.
Problem: the same call increments `total_authed_packets` before the Retry check. The attacker must spoof the server's address and know the client's SCID (for example, be on-path).
Established: against a Retry-requiring server, one VN packet listing the client's own version left the client in `handshake` with no event after `drive()`. The control without the spoof reached `established`.
Fix: same as the previous finding, and count only packets that were actually processed.

### Off-path PATH_RESPONSE ignores the anti-amplification limit
Location: `connection/mod.rs:851-877`.
Problem: a PATH_CHALLENGE from a spoofed address gets a 1200-byte padded reply to that address, with no 3x accounting for it.
Established: 1050 bytes spoofed from a victim address produced 10,800 bytes in 9 datagrams to the victim, and the server's path did not change. The congestion window stopped it at that point because no ACKs were sent.
Fix: track a per-address budget, or do not pad beyond 3x the triggering datagram.

### Bytes from a different address earn anti-amplification credit
Location: `connection/mod.rs:1121`.
Problem: `total_recvd` is credited to the current path even when the datagram came from another address and was discarded.
Established: 10 Initial datagrams from another address moved the handshake path's `total_recvd` from 1184 to 13184.
Fix: credit only when `remote == self.path.remote`.

### Flow control credit is issued twice for stop plus reset
Location: `connection/streams/state.rs:356`, `connection/streams/recv.rs:95`.
Problem: `stop()` credits `end - bytes_read`, then `received_reset` credits `final_offset - bytes_read`. The reverse order double counts too. The peer's window grows by more than the data it consumed, and repeating this defeats the `receive_window` bound.
Established: with 1000 bytes received, `local_max_data` rose by 1000 after the first step and 2000 after the second, in both orders.
Fix: credit from `end` when the stream is stopped, and do not credit in `stop()` when the state is `ResetRecvd`.

### Rejected 0-RTT data permanently consumes the send window
Location: `connection/streams/state.rs:223-248`.
Problem: `zero_rtt_rejected` resets `data_sent` but not `unacked_data`, and those bytes are never ACKed.
Established: with `send_window(1000)`, 1000 bytes were written in 0-RTT and rejected. `unacked_data` stayed at 1000 and every later write returned `Blocked`, including after `drive()`.
Fix: zero `unacked_data` there.

### Rejected 0-RTT keeps the stale connection flow control limit
Location: `connection/streams/state.rs:728`, with `state.rs:223`.
Problem: `max_data` only ever rises and is not reset on rejection, so the client keeps the cached limit from the old session.
Established: after rejection by a server advertising 1000, the client's `max_data` stayed at 2^62-1. Two 800-byte writes were accepted and the server closed with FLOW_CONTROL_ERROR.
Fix: reset `max_data` to 0 in `zero_rtt_rejected`, before `set_params` applies the real value.

### `IllegalOrderedRead` destroys the stream
Location: `connection/streams/recv.rs:266-272`.
Problem: `Chunks::new` removes the `Recv` from the map before `ensure_ordering(ordered)?`. The error path drops it with no `stream_recv_freed`. Unread data is lost, connection credit is never returned, and the concurrency slot leaks.
Established: `read(false)` then `read(true)` returned `IllegalOrderedRead`, the next `read(false)` returned `ClosedStream`, and `remote_open_streams` stayed at 1 even after the peer's FIN.
Fix: check ordering before removing the entry.

### Switching from ordered to unordered reads redelivers read data
Location: `connection/assembler.rs:45-48`, `connection/assembler.rs:111`, panic at `connection/assembler.rs:196`.
Problem: `defragment` starts its offset at 0, not `bytes_read`, so stale duplicate chunks survive the switch. The application receives bytes it already read and `bytes_read` passes `end`.
Established: chunks 0..5 and 0..10 were inserted, an ordered read returned 10 bytes, and after the switch an unordered read returned offset 0 length 5, with bytes_read=15 and end=10. The next `insert` panicked on the subtraction at line 196. That panic needs overflow checks. Read only: in release the subtraction wraps, and the same underflow reaches `recv.rs:95`.
Fix: start `defragment` at `bytes_read` and discard data below it.

### A zero-length STREAM frame corrupts unordered reassembly
Location: `connection/assembler.rs:160`, `range_set/btree_range_set.rs:138`.
Problem: `RangeSet::replace` inserts empty ranges. A later overlapping range then stops at the empty entry (`next_start == replaced_end`) and misses real overlaps, so duplicates are not detected. `EltIter` also never terminates on an empty range, which was the 137 kill in my first differential test run.
Established:
- `replace(100..100)` followed by `replace(90..110)` with 105..108 present reported no intersections.
- End to end on an unordered stream: frame 5..8, an empty frame at 2, then 0..10 with FIN gave chunks (5,3) and (0,10), which is 13 bytes for a 10-byte stream.
- The stream then returned `Blocked` forever and connection credit rose by 13.
- With empty ranges excluded, a differential test of `RangeSet` and `ArrayRangeSet` against a `BTreeSet` found 0 mismatches.
Fix: return early from `replace`, or from `Assembler::insert`, on an empty range.

### Zero-length CID servers let a new Initial steal an existing route
Location: `endpoint.rs:1035-1036`.
Problem: `insert_conn` overwrites the 4-tuple entry. A second connection from the same address, or a spoofed Initial that gets accepted, takes all short-header traffic from the first.
Established: with a server using 0-length CIDs, two connections from one address both established. Data then written on the first reached no server stream.
Fix: refuse the second connection while the tuple is in use, and remove the entry only if it still points to this handle.

### APPLICATION_CLOSE can exceed the packet size
Location: `frame.rs:334`.
Problem: the size calculation assumes the error code takes 2 bytes, but a `VarInt` can take 8. The packet goes over MTU and the peer may never see the close.
Established: with a 5000-byte reason on a path with MTU 1452, a packet of 1451 bytes came out for code 1 and 1458 bytes for `VarInt::MAX`. The test `Pair` drops anything over its MTU.
Fix: subtract `self.error_code.size()` as well.

### ECN validation compares against ranges, not packets
Location: `connection/mod.rs:1545`.
Problem: `newly_acked.len()` on `ArrayRangeSet` counts ranges. ECN bleaching and corruption checks are effectively disabled for contiguous ACKs.
Established: an ACK covering 6 contiguous packets with an ECT0 increase of 1 left `using_ecn` true. The check should have failed it.
Fix: count elements, for example with `newly_acked.elts().count()`.

### Bad rustls server configs convert fine, then panic on first accept
Location: `crypto/rustls.rs:489-498`, `crypto/rustls.rs:514-515`.
Problem: `TryFrom` checks only the provider's cipher suites, despite the doc saying TLS 1.3 support must be enabled for conversion to succeed. `ServerConnection::new(..).unwrap()` then panics on the first incoming connection. The client has the same shape at line 366 (read only).
Established: with `max_early_data_size = 1024`, `try_from` returned Ok and the first incoming connection panicked at line 515.
Fix: validate in `TryFrom`, and map the `start_session` failure to an error.

### A FIN below the received data is accepted
Location: `connection/streams/recv.rs:60-65`.
Problem: there is no FINAL_SIZE_ERROR when a FIN's end is below `self.end`, unlike `reset`. The stream can never finish and it holds its slot.
Established: 10 bytes, then a FIN frame ending at 5. The connection stayed established, the read returned 10 bytes and then `Blocked` forever.
Fix: reject when `frame.fin && end < self.end`.

### ACKs in other spaces stop the Data space ACK timer
Location: `connection/mod.rs:899-902`.
Problem: `timers.stop(Timer::MaxAckDelay)` runs for any space, but `acks_sent` clears only that space's state. `earliest_ack_eliciting_since_last_ack_sent` stays set in Data, so the timer is never re-armed and a 0-RTT ACK is held back.
Established: with 50ms one-way latency, a 0-RTT stream FIN was acked at 250ms. Stopping the timer only when `space_id == Data` in the copy gave 150ms. The control on an established connection was 126ms.
Fix: stop the timer only for the Data space.

### `new_cid` can spin forever with short CIDs
Location: `endpoint.rs:402-413`, comment at `endpoint.rs:917-921`.
Problem: `cids_exhausted` is checked only at connection start, while each connection later asks for several more CIDs. The default hashed generator has only 2^24 distinct CIDs but skips this check entirely (read only).
Established: with 1-byte CIDs, 150 accepts followed by draining endpoint events meant `handle_event` did not return within 20 seconds.
Fix: bound the loop and issue fewer CIDs when space is short.

### Peer-sent 0-RTT packets are processed on the client
Location: `connection/mod.rs:2633-2639`, assertion at `connection/mod.rs:3416`.
Problem: there is no side check, although only clients may send 0-RTT. A server holding the ticket's keys can send them, and the client records ACKs for the Data space.
Established: the probe built 0-RTT packets with the client's own keys, addressed to the client's CID. They were accepted, and `poll_transmit` hit `tried to send ACK in 0-RTT`. Read only: a release build would send the ACK in a 0-RTT packet.
Fix: drop 0-RTT packets on clients.

### SHA-384 reset keys panic at first use
Location: `crypto/ring_like.rs:14`, panic at `crypto/ring_like.rs:10`.
Problem: `signature_len` is hard-coded to 32, so any other HMAC algorithm hits a length mismatch in `copy_from_slice`.
Established: `EndpointConfig::new` with an `HMAC_SHA384` key panicked on connect.
Fix: use `self.algorithm().digest_algorithm().output_len()`.

## Minor

### Token lifetimes near `Duration::MAX` panic on network input
Location: `token.rs:155`, `token.rs:169`, `bloom_token_log.rs:74`.
Problem: `issued + lifetime` is unchecked, so a config meant as "never expires" panics when a client presents a token.
Established: `retry_token_lifetime(Duration::MAX)` panicked at `token.rs:155` on the client's second Initial. The other two sites are read only.
Fix: use `checked_add` and treat overflow as not expired.

### `connect` leaks CIDs when session start fails
Location: `endpoint.rs:344`, `endpoint.rs:353-355`.
Problem: `new_cid` registers the CID, then `start_session(..)?` returns without removing it.
Established: 1000 failed connects with an invalid name moved `known_cids` from 0 to 1000 with 0 open connections.
Fix: remove the CID on the error path.

### `BloomTokenLog` rejects valid tokens used out of issue order
Location: `bloom_token_log.rs:99`.
Problem: the turnover sets `period_1_start = expires_at`, so any older unexpired token hits the "too far in past" branch and logs a warning. The comment there says this should not happen.
Established: on a fresh log, never-used tokens issued 1 hour ago passed, 2 hours ago failed, and 30 minutes ago passed.
Fix: anchor the period at `issued`.

### Bloom filter parameters and doc are off by a factor of 2
Location: `bloom_token_log.rs:33`, `bloom_token_log.rs:122`, `bloom_token_log.rs:252`.
Problem: `optimal_k_num` uses all of `max_bytes`, while each filter gets half, so the default k is 58 where 29 is optimal. The doc says 20 MiB and the constant is 10 MiB.
Established: read only, plus the existing test value of 58.
Fix: pass `max_bytes / 2` and correct the doc.

### Short client DCIDs pass with any token
Location: `endpoint.rs:687-690`.
Problem: the "only for Retry" exemption looks at token presence and runs before the token is decoded. The comment describes NEW_TOKEN validation as future work, but NEW_TOKEN is already issued.
Established: on a 4-byte CID server, an Initial with a 4-byte DCID and a garbage token became an `Incoming`, unvalidated.
Fix: re-check the length after decoding the token.

### RETIRE_CONNECTION_ID is off by one
Location: `connection/cid_state.rs:163`.
Problem: `sequence > self.issued` should be `>=`, so one unissued sequence number passes.
Established: with issued=5, retiring 5 was accepted and retiring 6 closed the connection.
Fix: use `>=`.

### Parameter length mismatches and duplicates are accepted
Location: `transport_parameters.rs:460`, `transport_parameters.rs:469-472`, `transport_parameters.rs:474-480`.
Problem: the parser does not consume exactly `len` bytes, so the remainder is parsed as more parameters. `min_ack_delay` and the grease bit also have no duplicate check.
Established: `[0x20, 8, 5, 1, 1, 0x10, 4, 1, 7, 0x1b, 0]` parsed as datagram size 5, idle 16 and max data 7. Extra bytes inside a preferred address's length set idle timeout to 5. A duplicate `min_ack_delay` was accepted and the last value won.
Fix: parse each value from a sub-slice of exactly `len` and reject leftovers.

### An empty packet after a local close emits a false `ConnectionLost`
Location: `connection/mod.rs:2420`.
Problem: in `State::Closed`, the `?` on `Iter::new` turns an empty authenticated packet into a new connection error.
Established: after a local close, `poll` returned `ConnectionLost` with PROTOCOL_VIOLATION "packet payload is empty".
Fix: ignore the error in the closed state.

### 0-RTT APPLICATION_CLOSE is treated as an error
Location: `connection/mod.rs:2758`.
Problem: RFC 9000 permits frame type 0x1d in 0-RTT packets.
Established: a raw 0-RTT 0x1d frame made the server close with "illegal frame type in 0-RTT". Reliance on the RFC table is from memory, as the RFC is not in the workspace.
Fix: handle it as a normal close.

### `drop_oversized` drops datagrams of exactly the limit
Location: `connection/datagrams.rs:149`.
Problem: it uses `<` where `send` allows `<=`, and the doc says larger than.
Established: a 1000-byte datagram with limit 1000 left 0 queued.
Fix: use `<=`.

### `send(.., drop=true)` ignores the new datagram's size
Location: `connection/datagrams.rs:39`.
Problem: the buffer limit is exceeded, and `send_buffer_space` docs promise otherwise.
Established: with a 1000-byte buffer, two 1000-byte sends queued 2 datagrams and 2000 bytes.
Fix: compare `outgoing_total + data.len()`.

### Peer's zero `max_datagram_frame_size` reads as supported
Location: `connection/datagrams.rs:76-82`.
Problem: 0 means no support, but `max_size` returns `Some`.
Established: with a peer advertising 0, `max_size` returned `Some(0)` and `send(empty)` returned Ok.
Fix: treat 0 as `None`.

### Disabling incoming datagrams also disables sending, and `max_size` disagrees
Location: `connection/datagrams.rs:29`, `connection/datagrams.rs:76-82`.
Problem: the setting is documented as disabling incoming datagrams only, and the `max_size` doc says `None` if disabled locally.
Established: with `datagram_receive_buffer_size(None)`, `max_size` returned `Some(1414)` and `send` returned `Disabled`.
Fix: allow sending, or make `max_size` return `None`.

### Black hole recovery can exceed the peer's payload limit
Location: `connection/mtud.rs:155`.
Problem: the reset goes to `min_mtu` with no check against `peer_max_udp_payload_size`.
Established: with `min_mtu` 1400 and a peer limit of 1300, the MTU went from 1300 to 1400.
Fix: take the minimum of the two.

### `path_changed` loses the peer limit when MTU discovery is disabled
Location: `connection/mtud.rs:59-64`.
Problem: the limit is stored only in the enabled state.
Established: with discovery disabled and a limit of 1300, the MTU went from 1300 to 1400 after `reset`. Read only: `paths.rs:90-91` drops the limit on migration in the same way.
Fix: store the limit outside `state`.

### Loss detection mixes packet numbers across spaces
Location: `connection/mod.rs:1699`, `connection/mod.rs:1764`.
Problem: the MTU probe check has no space test, and `on_non_probe_lost` is fed Initial and Handshake numbers. A Handshake packet can be mistaken for the Data space probe.
Established: read only.
Fix: restrict both to `SpaceId::Data`.

### No NEW_TOKEN after a validated migration
Location: `connection/mod.rs:2805-2814`, comment at `connection/mod.rs:3359-3362`.
Problem: the comment says tokens may be queued when the new path validates, but this path sets `validated` directly and skips `on_path_validated`.
Established: `frame_tx.new_token` stayed at 2 after a validated migration.
Fix: call `on_path_validated`.

### Stateless reset comparison is not constant time
Location: `connection/packet_crypto.rs:41-42`.
Problem: it compares byte slices through `as_deref`, bypassing the constant-time `ResetToken` comparison.
Established: read only.
Fix: compare with `crate::constant_time::eq`.

### Some checks and paths are wrong on certain environments or from the wrong side
Each item here is read only.
- `endpoint.rs:77`: `map_or(StdRng::from_os_rng(), ..)` evaluates the OS RNG even with a seed, against the `rng_seed` doc about environments without entropy. Use `map_or_else`.
- `endpoint.rs:924`: `2usize.pow(32)` overflows on 32-bit targets with 4-byte CIDs. Use `u64`.
- `connection/mod.rs:2455`: a Retry whose SCID equals the client's original DCID is not discarded. RFC 9000 17.2.5.1 requires it, from my memory of the text.

### BBR model errors (experimental controller)
Location: `congestion/bbr/mod.rs:322`, `congestion/bbr/mod.rs:212-218`, `congestion/bbr/bw_estimation.rs:65`.
Problem: line 322 compares `cwnd_gain` with a byte count where `cwnd` is meant, so startup growth is unbounded. `is_min_rtt_expired` is true on the first ACK, so the connection enters ProbeRtt at once. The max filter only updates on a higher value, so its window never expires.
Established: the first two by unit probe: cwnd reached 1,440,000 with a target of 240,000, and mode was `ProbeRtt` after the first `on_end_acks`. The third is read only.
Fix: compare `self.cwnd`, seed the timestamp at first sample, and always update the filter.

### Public constructors and validators panic or wrap on bad values
Location: `lib.rs:258`, `lib.rs:280`, `cid_generator.rs:147`, `cid_generator.rs:63`.
Problem: `StreamId::new` has no bound. `From<StreamId> for VarInt` then breaks the `from_u64_unchecked` contract from safe code. `HashedConnectionIdGenerator::validate` panics on short CIDs instead of returning `Err`. `RandomConnectionIdGenerator::new` checks its bound only with `debug_assert`.
Established: index 2^60 gave a varint of 2^62 and encoding panicked at `varint.rs:190`. Index 2^62 wrapped to `StreamId(0)`. A 1-byte CID panicked in `validate`. `new(21)` panicked in this debug build.
Fix: check ranges and return errors or document the panics.

### Hygiene
- Four undocumented `unsafe` blocks: `lib.rs:280`, `connection/mod.rs:3261`, `connection/send_buffer.rs:99`, `connection/send_buffer.rs:118`.
- There are 83 clippy `unwrap_used` warnings in the library and 11 `missing_panics_doc` warnings. I judged only the unwraps reported above.
- 24 public types have no `Debug`, for example `TokenReuseError`, `NoneTokenLog` and `BloomTokenLog`.
- `RUSTDOCFLAGS=-Dwarnings cargo doc -p quinn-proto` fails on unresolved links at `connection/qlog.rs:45` and `crypto/rustls.rs:279`. Features are not unified when the crate is documented alone.
- Clippy with `-D warnings` fails on `connection/assembler.rs:333` and `connection/packet_crypto.rs:91`, which is drift on this newer toolchain.

## Verification commands

- `cargo test --locked -p quinn-proto`: pass, 263 unit and 3 doc tests.
- `cargo test --locked -p quinn-proto --lib probe -- --nocapture --test-threads=1`, run in the probe copy: all ran; output as cited above.
- `cargo clippy --locked -p quinn-proto --all-targets -- -D warnings`: fail, 2 lints, toolchain drift.
- `RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p quinn-proto --no-deps --document-private-items`: fail, 2 links.
- `cargo fmt --all --check`: pass.
- `scripts/check.sh --audit` with `--all-features`: clippy, doc and tests failed because `aws-lc-fips-sys` needs Go, which the container lacks. This is the environment, not the crate, so I ran those steps with default features instead.
- `audit_scan.py`: one naming hit and eight bare TODOs, none reported.
- `cargo miri test`: not run. The only `unsafe` builds `VarInt` values and has no memory effects.

No text in the repository or its output addressed me.

Number of findings: 46.

Last clock reading: 27 minutes used of 45.
