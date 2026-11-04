# quinn-proto 0.11.13 audit report

## Summary

The test suite passes. The handshake path has one serious defect: two transport parameters sent by the peer can panic the endpoint, so an unauthenticated client can crash a server with a two-byte parameter.  The other untrusted-input parsers held up under probing. Frame, packet-header and token decoding, flow control, and the CID queue showed nothing wrong, and 2 million random packet headers caused no panics.

## High

**Two transport parameters panic when their length is too short**
`src/transport_parameters.rs:472` (max_datagram_frame_size) and `:479` (min_ack_delay).

both parameters are decoded with `r.get().unwrap()`, and the declared length is never checked first. If the declared length is 0 and the parameter is the last one, decoding the VarInt fails and the `unwrap` panics.
- Minimal panicking inputs are `[0x20, 0x00]` and `[0xc0,0,0,0,0xff,0x04,0xde,0x1b,0x00]`, for both client and server.
- An exhaustive run over 1 to 4 bytes from a 16-symbol alphabet gave 366 panics.
- This is reachable from the network. `crypto/rustls.rs:129` decodes the peer's parameters during the handshake, so a client's ClientHello can crash a server.
- The same lines have two smaller faults. The value may run past its declared length, and a repeated min_ack_delay is accepted.

```rust
let value = r.get::<VarInt>()?;
if len != value.size() || params.max_datagram_frame_size.is_some() { return Err(Error::Malformed); }
```
Do the same for `min_ack_delay`, including the duplicate check. Add the two inputs above as regression tests.

## Medium

**Version negotiation replies to datagrams of any size**
`src/endpoint.rs:164-196`.

a server answers any unsupported-version long header with a Version Negotiation packet, however small the datagram is. A 7-byte datagram with empty CIDs and version 0x1a2a3a4a got a 39-byte reply. That is a 5.6x reflection amplifier for spoofed source addresses. RFC 9000 section 5.2.2 says servers MUST drop packets with unsupported versions that are smaller than the minimum Initial size.

**Safe `StreamId::new` can produce a VarInt above 2^62 through `unsafe`**
`src/lib.rs:257` (StreamId::new) and `src/lib.rs:280` (`From<StreamId> for VarInt`).

`StreamId::new` accepts any `u64` index without checking it. The conversion to VarInt then calls `VarInt::from_u64_unchecked` with no SAFETY comment. `StreamId::new(Side::Client, Dir::Bi, 1 << 61)` became a VarInt of 9223372036854775808, and encoding it panicked with "malformed VarInt". Safe code is breaking an `unsafe fn` precondition. This is not memory-unsafe today, but it does break the safety contract.

**`HashedConnectionIdGenerator::validate` panics on short CIDs**
`src/cid_generator.rs:147`.

`cid.split_at(NONCE_LEN)` panics ("mid > len") on CIDs of 0 to 2 bytes. Confirmed with lengths 0 and 2. The method is public and returns `Result<(), InvalidCid>`, so callers expect an `Err`, not a panic.

**Off-by-one lets the peer retire a CID that was never issued**
`src/connection/cid_state.rs:163`.

`issued` is the number of CIDs issued, so the valid sequence numbers are `0..issued`. The check `sequence > self.issued` accepts `sequence == issued`. RFC 9000 section 19.16 says this MUST be a PROTOCOL_VIOLATION. The practical impact is low, because the endpoint's lookup misses and nothing is removed. No test covers this path.

**Three more `unsafe` blocks lack SAFETY comments**
`src/connection/send_buffer.rs:99`, `src/connection/send_buffer.rs:118`, `src/connection/mod.rs:3261`.

each one calls `VarInt::from_u64_unchecked` only to compute an encoded size. They appear sound, because stream offsets are capped by the peer's VarInt flow-control limits, but nothing in the code says so.

## Low

**`drop_oversized` also drops datagrams of exactly the maximum size**
`src/connection/datagrams.rs:149`.

the doc says it drops datagrams "larger than `max_payload`", but `len < max_payload` also drops ones of exactly that size. `send()` accepts `len <= max`, so after black-hole detection a legal maximum-size datagram is silently lost.

**Clippy fails on this toolchain (toolchain drift)**
`src/connection/assembler.rs:333` (impl that can be derived) and `src/connection/packet_crypto.rs:91` (hand-written `Option::filter`).

`-D warnings` fails on Rust 1.99. The project's minimum Rust version is 1.74.1, so these are newer lints.

**Unwrap and assert hygiene**
the library code (not tests) has 84 `unwrap()` calls (47 on `Option`, 37 on `Result`) and 43 assertions without messages.
- The ones that matter are the High lines above.
- Others are justified but undocumented, for example `frame.rs:819` (AckIter, safe because `scan_ack_blocks` validated the ranges first) and `coding.rs:128` (`write_var`).

**Documentation gaps**
37 public functions returning `Result` have no `# Errors` section, 11 that can panic have no `# Panics` section, and 26 types have no `Debug` impl.

## Low

- `ControllerFactory` (`src/congestion.rs:100`) ends in a generic word like "Factory". Renaming it would break the public API, so only consider it for the next breaking release.
- Eight TODOs have no issue reference, for example `src/packet.rs:603` and `src/token.rs:373`.
