# quinn-proto 0.11.13 audit report

## Summary

The packet, frame and varint parsers held up under fuzzing. The decoders for two transport parameters do not: a peer can crash the endpoint during the handshake with a two-byte transport-parameter blob. A second remote panic exists for endpoints that enable ACK frequency, through an unchecked `Duration::clamp`.

Defects:  other notes items: 6. The `quinn-proto` test suite passes: 263 unit tests and 3 others.

## High

**Peer transport parameters can panic the endpoint**
`src/transport_parameters.rs:472` (max_datagram_frame_size) and `src/transport_parameters.rs:479` (min_ack_delay), inside `TransportParameters::read`.

if a peer declares one of these two parameters with a length shorter than the varint that follows, the process panics. A length of 0 at the end of the blob is enough. A malicious client or server can crash any quinn endpoint during the handshake.

both branches call `r.get().unwrap()` on the outer buffer. They never check `len` against the varint's size. Every other numeric parameter uses `r.get::<VarInt>()?` and requires `len == value.size()`. The same gap lets a longer varint read past the declared length and desynchronise the rest of the parse. Separately, `min_ack_delay` has no duplicate check: two copies are accepted and the last one wins.

the probe in `tmp/h2` calls the public `quinn_proto::transport_parameters::TransportParameters::read`.
- `[0x20, 0x00]` panicked at `transport_parameters.rs:472`: "called `Result::unwrap()` on an `Err` value: UnexpectedEnd".
- `[0xc0,0,0,0,0xff,0x04,0xde,0x1b, 0x00]` panicked at line 479.
- A seeded random harness (`tmp/h1`, 3 million inputs) found 43 more panicking inputs, all through these lines.
- The parser is reached with peer bytes from `src/crypto/rustls.rs:129` (`quic_transport_parameters()`).

**ACK frequency panics when the peer's min_ack_delay is large**
`src/connection/ack_frequency.rs:44-49` (`candidate_max_ack_delay`). It is called from `src/connection/mod.rs:503` and `:3198`.

when the local `TransportConfig` sets `ack_frequency_config`, a peer can make the connection panic.
- The peer advertises `max_ack_delay = 16000` ms and `min_ack_delay = 16_000_000` µs. Validation at `transport_parameters.rs:514` accepts this.
- Any RTT below 16 s then makes `clamp(min, max)` run with min greater than max, and `Ord::clamp` panics in that case.

`config.max_ack_delay.unwrap_or(..).clamp(min_ack_delay, rtt.max(MIN_AUTOMATIC_ACK_DELAY))` assumes the peer's lower bound is never above the local upper bound.

I temporarily added a unit test to `ack_frequency.rs`, ran it and removed it; a copy is in `tmp/ack_frequency_probe.rs`. It decodes those parameters with `TransportParameters::read(Side::Server, ..)`, which returns `Ok`, and then calls `candidate_max_ack_delay(50 ms, &AckFrequencyConfig::default(), &params)`. The call panicked.

## Medium

**Version negotiation is sent in reply to tiny datagrams**
`src/endpoint.rs:164-196` (`Endpoint::handle`, the `UnsupportedVersion` arm).

a server answers any long-header packet with an unknown version, however small, with a Version Negotiation packet that is several times larger. RFC 9000 §5.2.2 says servers MUST drop packets with unsupported versions that are smaller than the minimum datagram size of 1200 bytes. Without that check, a spoofed source address gets about 5.6 times as many bytes reflected back.

unlike `handle_first_packet` (line 433), this arm never compares `datagram_len` with `MIN_INITIAL_SIZE`.

`tmp/h2` sets up a server `Endpoint` and passes the 7-byte packet `c0 aa bb cc dd 00 00` to `Endpoint::handle`. It returned `DatagramEvent::Response` with `size = 39`. A 47-byte packet with 20-byte connection IDs got a 79-byte response.

**StreamId::new silently wraps and produces VarInt values of 2^62 or more**
`src/lib.rs:256` (`StreamId::new`) and `src/lib.rs:278-281` (`From<StreamId> for VarInt`).

- `StreamId::new(side, dir, index)` shifts `index << 2` without a check. Indices of 2^62 or more lose their high bits; for example, index 2^62+5 becomes 5.
- Indices of 2^60 or more give a stream ID whose `VarInt` conversion breaks the type's documented "less than 2^62" invariant. Encoding that `VarInt` then hits `unreachable!`.

the infallible `From` conversion calls `unsafe { VarInt::from_u64_unchecked(x.0) }` on a value that safe public code controls.

Evidence (ran), from `tmp/h1/src/bin/h3.rs` in a release build:
- `StreamId::new(Client, Bi, 1<<61)` gives a `VarInt` value of 9223372036854775808, while `VarInt::MAX` is 4611686018427387903.
- Encoding that value panicked.
- `StreamId::new(Client, Bi, (1<<62)+5).index()` returned 5.

**unsafe is used for a non-memory invariant, without SAFETY comments**
`src/varint.rs:44` (`pub const unsafe fn from_u64_unchecked`); call sites at `src/lib.rs:280`, `src/connection/send_buffer.rs:99` and `:118`, and `src/connection/mod.rs:3261`.

none of the four `unsafe` blocks has a `// SAFETY:` comment. The previous finding shows that safe code can break the precondition at `lib.rs:280`. Nothing currently relies on the invariant for memory safety, so the result is a panic rather than undefined behaviour. Still, `unsafe` here only marks a value-range invariant, which makes future reliance on it risky.

an `unsafe fn` is used as a cheap constructor for sizing calculations.

the advisory clippy pass reported `undocumented_unsafe_blocks` at those four sites. `VarInt::size` (`varint.rs:54-67`) panics on values that are too large; it has no unsafe dependency.

## Low

**Datagram send buffer can exceed its configured limit by one datagram**
`src/connection/datagrams.rs:38-48` (`Datagrams::send` with `drop = true`).

the `datagram_send_buffer_size` docs say older datagrams "are dropped until sufficient space is available". In fact, with a 1000-byte limit and 1000 bytes queued, sending another 1000 bytes drops nothing and leaves 2000 bytes buffered.

the loop checks `outgoing_total > limit` instead of `outgoing_total + data.len() > limit`.

the loop condition at line 39, compared with the non-drop branch at line 49, which does include `data.len()`.

**RandomConnectionIdGenerator accepts invalid lengths in release builds**
`src/cid_generator.rs:62-68`.

`RandomConnectionIdGenerator::new(21)` succeeds in release builds and only panics later, inside `generate_cid`, with a slice-index error. In an endpoint, that is the first connect or accept.

the only check is a `debug_assert!`, and the constructor has no `# Panics` section.

in `h3` (release build), `RandomConnectionIdGenerator::new(21).generate_cid()` panicked.

## Other notes

These come from the advisory clippy pass on `quinn-proto/src` only.

**Toolchain drift: clippy with -D warnings fails on Rust 1.99**
- quinn-proto: `manual_filter` (`src/connection/packet_crypto.rs:91`) and `derivable_impls` (`src/connection/assembler.rs:333`).
- quinn-udp's `build.rs`, which is outside the audit scope, fails the new trailing-semicolon-in-macro lint.
- The workspace pins no toolchain; its MSRV is 1.74.1.

**Documentation sections**
- 38 missing `# Errors` sections, for example `TransportParameters::read` (`transport_parameters.rs:407`) and `VarInt::from_u64`.
- 12 missing `# Panics` sections, for example `Datagrams::send` (`datagrams.rs:28`).

**Missing Debug implementations**
26 types, mostly in `range_set/btree_range_set.rs`.

**Panics and assertions in production code**
- 85 `unwrap()` calls outside tests. The ones that matter are at `transport_parameters.rs:472/479`, reported above. Most others are invariant unwraps with a comment.
- 43 assertions without messages.
- 5 `map_err(|_| ...)` calls that discard the error, for example `varint.rs:106` and `crypto/rustls.rs:201`.

**Naming and imports**
- `ControllerFactory` (`congestion.rs:100`) is a weasel-word name, but renaming this public trait would be a breaking change.
- One glob import, at `connection/mod.rs:3957`.

**Bare TODOs**
8 TODOs with no issue reference, for example `packet.rs:603` and `token.rs:373`.

## Other notes

- I did not test whether `PacketNumber::expand` (`packet.rs:775`, which uses `candidate > win` where RFC 9000 Appendix A uses `>=`) matters. My reasoning suggests the two differ only on unreachable inputs.
