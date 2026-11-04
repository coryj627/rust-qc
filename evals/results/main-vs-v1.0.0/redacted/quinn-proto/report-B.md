# Audit report: quinn-proto 0.11.13

## Summary

Any client can crash a quinn-proto server, before the handshake completes, by sending a two-byte malformed transport parameter. That makes the crate unsafe to deploy as-is. Beyond that the code is mostly solid: about 1.3 million random frame and transport-parameter inputs, and differential tests of the range sets, stream reassembly and packet-number expansion, turned up nothing else. The other findings are protocol-conformance and hardening gaps found by reading.

## High

**Malformed transport parameters crash the peer that parses them**
src/transport_parameters.rs:472 (`max_datagram_frame_size`) and src/transport_parameters.rs:479 (`min_ack_delay`), both in `TransportParameters::read`.

both branches call `r.get().unwrap()`. The value is read from the whole remaining buffer rather than from the parameter's declared length. When that read fails, the code panics. The server reads the client's parameters from the TLS ClientHello (src/crypto/rustls.rs:129), so this needs no authentication.
- Confirmed by a probe: the inputs `20 00` and `20 01 40` panic with `called Result::unwrap() on an Err value: UnexpectedEnd` for both sides.
- A random harness hit the same line 528 times in 300,000 inputs.
- `min_ack_delay` (id 0xFF04DE1B) has the same code shape. It also has no length check and no duplicate check. My crafted probe for it used the wrong ID (…1A instead of …1B), so this branch is shown by reading only, not by a run.
- Even when the read succeeds, it ignores `len`. A parameter that declares 8 bytes but holds a 1-byte varint makes the parser read the next 7 bytes as new parameters.

## Medium

**RETIRE_CONNECTION_ID accepts a sequence number that was never issued**
src/connection/cid_state.rs:163 (`on_cid_retirement`).

`issued` is a count, so valid sequence numbers are `0..issued`. The check `sequence > self.issued` lets `sequence == issued` through. RFC 9000 section 19.16 requires PROTOCOL_VIOLATION here. Instead the frame is accepted, `allow_more_cids` is computed, and an endpoint event goes out for a CID that doesn't exist.

**Stateless reset tokens are compared in variable time**
src/connection/packet_crypto.rs:41 (`unprotect_header`).

`stateless_reset_token.as_deref() == Some(&packet[..])` compares plain byte slices, which bypasses the constant-time `PartialEq` that `ResetToken` provides. RFC 9000 section 10.3.1 says this comparison MUST NOT leak information about the token.

**Close frames can exceed the packet size budget**
src/frame.rs:333 (`ApplicationClose::encode`) and src/frame.rs:293 (`ConnectionClose::encode`).

the reason length is capped at `max_len - 3 - …`, which assumes a 1-byte error code. The application error code is a caller-chosen `VarInt` of up to 8 bytes. With a code of 2^30 or more and a reason longer than the space left, the frame comes out 6 bytes over `max_len`, so the close datagram is larger than the path MTU. This is derived by arithmetic: frame size = `max_len + code_size - 2`. I couldn't run the probe I wrote for it (see the closing summary).

**Safe code can build a VarInt that breaks the unsafe constructor's rule**
src/lib.rs:280 (`impl From<StreamId> for VarInt`).

`StreamId::new(initiator, dir, index)` is public and safe and accepts any `u64` index. `From<StreamId> for VarInt` then calls `unsafe { VarInt::from_u64_unchecked(x.0) }` with no check and no SAFETY comment. An index of 2^60 or more gives a VarInt of 2^62 or more. Later, `size()` panics with "malformed VarInt" and `encode` hits `unreachable!`. This is not memory unsafety, because nothing unsafe relies on the bound. But it breaks the documented `# Safety` precondition from safe code.

**The other unsafe blocks have no SAFETY comments**
src/connection/send_buffer.rs:99, src/connection/send_buffer.rs:118, src/connection/mod.rs:3261.

each one calls `VarInt::from_u64_unchecked` on a stream or crypto offset. They look sound, because flow control keeps offsets below 2^62, but that reasoning isn't written down. The crate has 0 SAFETY comments in total.

## Low

**The 0-RTT frame filter does not match RFC 9000**
src/connection/mod.rs:2756.

it rejects APPLICATION_CLOSE (type 0x1d), which RFC 9000 Table 3 allows in 0-RTT. It accepts ACK, RETIRE_CONNECTION_ID and PATH_RESPONSE, which are not allowed there and are replayable.

**Loss detection can panic when the monotonic clock is close to its starting point**
src/connection/mod.rs:1674.

`now.checked_sub(loss_delay).unwrap()` panics if `Instant` is closer to its origin than `rtt * time_threshold`. That can happen shortly after boot with a large configured `initial_rtt` or `time_threshold`.

**Clippy and rustdoc fail**
src/connection/assembler.rs:333, src/connection/packet_crypto.rs:91, src/crypto/rustls.rs:279.

- Clippy 1.99 flags `derivable_impls` and `manual_filter`. These lints are newer than the toolchain the project targets, so this is toolchain drift.
- Rustdoc with default features fails on a link to `ClientConfig::with_platform_verifier`, which only exists with the `platform-verifier` feature.

**A public type uses a weasel-word name**
src/congestion.rs:100 (`ControllerFactory`).

names ending in `Factory` describe a pattern rather than what the item does. Renaming this public trait would be a breaking change.

## Low

- Eight TODO comments have no owner or issue reference: connection/mod.rs:2318, connection/mod.rs:3426, send_buffer.rs:22, spaces.rs:157, streams/mod.rs:51, packet.rs:603, array_range_set.rs:72, token.rs:373.
- What found nothing:
  - 1,000,000 random frame payloads through `frame::Iter`, including iterating every ACK it decoded: no panics.
  - 3,000 randomised rounds comparing `ArrayRangeSet`, `RangeSet` and `replace` against a `BTreeSet`: no mismatches.
  - 2,000 reassembly rounds in `Assembler` (ordered and unordered, with overlaps): no mismatches.
  - 1,000,000 packet-number expansion cases against a reference: no mismatches.

  Keep these as regression tests.
