# quinn-proto 0.11.13 audit report

## Verdict

The packet, frame and varint parsers held up under fuzzing. The decoders for two transport parameters do not: a peer can crash the endpoint during the handshake with a two-byte transport-parameter blob. A second remote panic exists for endpoints that enable ACK frequency, through an unchecked `Duration::clamp`.

Defects: 2 Blocker, 3 Major, 2 Minor. QA hygiene items: 6. The `quinn-proto` test suite passes: 263 unit tests and 3 others.

## Blocker

### Peer transport parameters can panic the endpoint
Location: `src/transport_parameters.rs:472` (max_datagram_frame_size) and `src/transport_parameters.rs:479` (min_ack_delay), inside `TransportParameters::read`.

Problem: if a peer declares one of these two parameters with a length shorter than the varint that follows, the process panics. A length of 0 at the end of the blob is enough. A malicious client or server can crash any quinn endpoint during the handshake.

Cause: both branches call `r.get().unwrap()` on the outer buffer. They never check `len` against the varint's size. Every other numeric parameter uses `r.get::<VarInt>()?` and requires `len == value.size()`. The same gap lets a longer varint read past the declared length and desynchronise the rest of the parse. Separately, `min_ack_delay` has no duplicate check: two copies are accepted and the last one wins.

Evidence (ran): the probe in `tmp/h2` calls the public `quinn_proto::transport_parameters::TransportParameters::read`.
- `[0x20, 0x00]` panicked at `transport_parameters.rs:472`: "called `Result::unwrap()` on an `Err` value: UnexpectedEnd".
- `[0xc0,0,0,0,0xff,0x04,0xde,0x1b, 0x00]` panicked at line 479.
- A seeded random harness (`tmp/h1`, 3 million inputs) found 43 more panicking inputs, all through these lines.
- The parser is reached with peer bytes from `src/crypto/rustls.rs:129` (`quic_transport_parameters()`).

Fix: in both branches, decode with `let value = r.get::<VarInt>()?;`, return `Error::Malformed` when `len != value.size()` or the parameter was already seen, then store `value`.

### ACK frequency panics when the peer's min_ack_delay is large
Location: `src/connection/ack_frequency.rs:44-49` (`candidate_max_ack_delay`). It is called from `src/connection/mod.rs:503` and `:3198`.

Problem: when the local `TransportConfig` sets `ack_frequency_config`, a peer can make the connection panic.
- The peer advertises `max_ack_delay = 16000` ms and `min_ack_delay = 16_000_000` µs. Validation at `transport_parameters.rs:514` accepts this.
- Any RTT below 16 s then makes `clamp(min, max)` run with min greater than max, and `Ord::clamp` panics in that case.

Cause: `config.max_ack_delay.unwrap_or(..).clamp(min_ack_delay, rtt.max(MIN_AUTOMATIC_ACK_DELAY))` assumes the peer's lower bound is never above the local upper bound.

Evidence (ran): I temporarily added a unit test to `ack_frequency.rs`, ran it and removed it; a copy is in `tmp/ack_frequency_probe.rs`. It decodes those parameters with `TransportParameters::read(Side::Server, ..)`, which returns `Ok`, and then calls `candidate_max_ack_delay(50 ms, &AckFrequencyConfig::default(), &params)`. The call panicked.

Fix: compute `let upper = rtt.max(MIN_AUTOMATIC_ACK_DELAY).max(min_ack_delay);` before clamping, or skip ACK frequency when the peer's `min_ack_delay` is above the value the endpoint is willing to request.

## Major

### Version negotiation is sent in reply to tiny datagrams
Location: `src/endpoint.rs:164-196` (`Endpoint::handle`, the `UnsupportedVersion` arm).

Problem: a server answers any long-header packet with an unknown version, however small, with a Version Negotiation packet that is several times larger. RFC 9000 §5.2.2 says servers MUST drop packets with unsupported versions that are smaller than the minimum datagram size of 1200 bytes. Without that check, a spoofed source address gets about 5.6 times as many bytes reflected back.

Cause: unlike `handle_first_packet` (line 433), this arm never compares `datagram_len` with `MIN_INITIAL_SIZE`.

Evidence (ran): `tmp/h2` sets up a server `Endpoint` and passes the 7-byte packet `c0 aa bb cc dd 00 00` to `Endpoint::handle`. It returned `DatagramEvent::Response` with `size = 39`. A 47-byte packet with 20-byte connection IDs got a 79-byte response.

Fix: in that arm, `if datagram_len < MIN_INITIAL_SIZE as usize { return None; }` before encoding the response.

### StreamId::new silently wraps and produces VarInt values of 2^62 or more
Location: `src/lib.rs:256` (`StreamId::new`) and `src/lib.rs:278-281` (`From<StreamId> for VarInt`).

Problem:
- `StreamId::new(side, dir, index)` shifts `index << 2` without a check. Indices of 2^62 or more lose their high bits; for example, index 2^62+5 becomes 5.
- Indices of 2^60 or more give a stream ID whose `VarInt` conversion breaks the type's documented "less than 2^62" invariant. Encoding that `VarInt` then hits `unreachable!`.

Cause: the infallible `From` conversion calls `unsafe { VarInt::from_u64_unchecked(x.0) }` on a value that safe public code controls.

Evidence (ran), from `tmp/h1/src/bin/h3.rs` in a release build:
- `StreamId::new(Client, Bi, 1<<61)` gives a `VarInt` value of 9223372036854775808, while `VarInt::MAX` is 4611686018427387903.
- Encoding that value panicked.
- `StreamId::new(Client, Bi, (1<<62)+5).index()` returned 5.

Fix: reject indices of 2^60 or more in `StreamId::new`, either with an `assert!` and a `# Panics` section or by adding a `try_new`. Also replace the unchecked conversion with `VarInt::from_u64(x.0).expect(..)` or a `TryFrom`.

### unsafe is used for a non-memory invariant, without SAFETY comments
Location: `src/varint.rs:44` (`pub const unsafe fn from_u64_unchecked`); call sites at `src/lib.rs:280`, `src/connection/send_buffer.rs:99` and `:118`, and `src/connection/mod.rs:3261`.

Problem: none of the four `unsafe` blocks has a `// SAFETY:` comment. The previous finding shows that safe code can break the precondition at `lib.rs:280`. Nothing currently relies on the invariant for memory safety, so the result is a panic rather than undefined behaviour. Still, `unsafe` here only marks a value-range invariant, which makes future reliance on it risky.

Cause: an `unsafe fn` is used as a cheap constructor for sizing calculations.

Evidence (read): the advisory clippy pass reported `undocumented_unsafe_blocks` at those four sites. `VarInt::size` (`varint.rs:54-67`) panics on values that are too large; it has no unsafe dependency.

Fix: add a safe `pub(crate) fn` for the sizing call sites, and use the checked constructor at `lib.rs:280` (see the previous finding). If the public unsafe API must stay for compatibility, document each block.

## Minor

### Datagram send buffer can exceed its configured limit by one datagram
Location: `src/connection/datagrams.rs:38-48` (`Datagrams::send` with `drop = true`).

Problem: the `datagram_send_buffer_size` docs say older datagrams "are dropped until sufficient space is available". In fact, with a 1000-byte limit and 1000 bytes queued, sending another 1000 bytes drops nothing and leaves 2000 bytes buffered.

Cause: the loop checks `outgoing_total > limit` instead of `outgoing_total + data.len() > limit`.

Evidence (read): the loop condition at line 39, compared with the non-drop branch at line 49, which does include `data.len()`.

Fix: use `while outgoing_total + data.len() > datagram_send_buffer_size && !outgoing.is_empty()`.

### RandomConnectionIdGenerator accepts invalid lengths in release builds
Location: `src/cid_generator.rs:62-68`.

Problem: `RandomConnectionIdGenerator::new(21)` succeeds in release builds and only panics later, inside `generate_cid`, with a slice-index error. In an endpoint, that is the first connect or accept.

Cause: the only check is a `debug_assert!`, and the constructor has no `# Panics` section.

Evidence (ran): in `h3` (release build), `RandomConnectionIdGenerator::new(21).generate_cid()` panicked.

Fix: use `assert!(cid_len <= MAX_CID_SIZE, "...")` with a `# Panics` section, or add a fallible `try_new`.

## QA hygiene

These come from the advisory clippy pass on `quinn-proto/src` only.

### Toolchain drift: clippy with -D warnings fails on Rust 1.99
- quinn-proto: `manual_filter` (`src/connection/packet_crypto.rs:91`) and `derivable_impls` (`src/connection/assembler.rs:333`).
- quinn-udp's `build.rs`, which is outside the audit scope, fails the new trailing-semicolon-in-macro lint.
- The workspace pins no toolchain; its MSRV is 1.74.1.

### Documentation sections
- 38 missing `# Errors` sections, for example `TransportParameters::read` (`transport_parameters.rs:407`) and `VarInt::from_u64`.
- 12 missing `# Panics` sections, for example `Datagrams::send` (`datagrams.rs:28`).

### Missing Debug implementations
26 types, mostly in `range_set/btree_range_set.rs`.

### Panics and assertions in production code
- 85 `unwrap()` calls outside tests. The ones that matter are at `transport_parameters.rs:472/479`, reported above. Most others are invariant unwraps with a comment.
- 43 assertions without messages.
- 5 `map_err(|_| ...)` calls that discard the error, for example `varint.rs:106` and `crypto/rustls.rs:201`.

### Naming and imports
- `ControllerFactory` (`congestion.rs:100`) is a weasel-word name, but renaming this public trait would be a breaking change.
- One glob import, at `connection/mod.rs:3957`.

### Bare TODOs
8 TODOs with no issue reference, for example `packet.rs:603` and `token.rs:373`.

## Unverified

- I did not test whether `PacketNumber::expand` (`packet.rs:775`, which uses `candidate > win` where RFC 9000 Appendix A uses `>=`) matters. My reasoning suggests the two differ only on unreachable inputs.

## Verification commands

All were run from the workspace root (`crate/`) with Rust 1.99.1.

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail. `aws-lc-fips-sys` needs Go, which the sandbox lacks, so this is an environment failure.
- `cargo clippy --workspace --all-targets --features "quinn-proto/rustls-ring,quinn-proto/log,quinn-proto/bloom,quinn-proto/qlog,quinn-proto/arbitrary" -- -D warnings`: fail. The causes are the quinn-udp `build.rs` lint and the two quinn-proto lints listed under QA hygiene.
- `cargo clippy --locked -p quinn-proto --all-targets -- -D warnings`: fail. Two errors: `manual_filter` and `derivable_impls`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features "<same list>"`: pass.
- `cargo test --locked -p quinn-proto`: pass (263 + 3).
- `cargo test --workspace --features "<same list>"`: fail. Three `quinn` tests (`echo_v6`, `echo_dualstack`, `local_addr`) fail with "Address family not supported", because the sandbox has no IPv6. This is environmental and outside `quinn-proto`.
- Harnesses: `cargo run --release` in `tmp/h1` (fuzzes the parsers; `--bin h3` probes `StreamId`, `VarInt` and the CID generator) and `cargo run` in `tmp/h2` (endpoint and transport-parameter probe). Both reproduce the findings above.
- Temporary unit tests in `crate/quinn-proto/src` (an ACK frequency probe and a 2-million-input fuzz of `frame::Iter`): run, then removed. The frame fuzz found no panics. `crate/` is back to its original state.
- `cargo +nightly-2026-10-01 miri test`: skipped. The only `unsafe` code is the `VarInt` constructor, which cannot cause undefined behaviour, and time was better spent on the probes.

## Closing summary

I fuzzed the public parsers that take untrusted bytes: transport parameters and packet headers with `PartialDecode`, plus the private frame iterator through a temporary in-crate test. I probed `Endpoint::handle` for version negotiation and the `StreamId`/`VarInt` conversions, and read the code that handles peer frames: ACK, NEW_CONNECTION_ID, stream flow control, reset, retry, tokens, the CID queue, the assembler, datagrams, pacing, MTU discovery and ACK frequency.

I did not check:
- The congestion controllers (cubic, new_reno, bbr) and the qlog code.
- Most of the 4,100-line `connection/mod.rs` packet assembly path.
- Full two-endpoint handshakes with a malicious peer. The remote panics are shown at the parser and function level, with the call path traced by reading the code.
- Miri.

I stopped because the main untrusted-input surfaces were covered and further reading of the large connection state machine was turning up only code that behaves correctly.

Last `./clock` reading: 12 minutes used of 45.