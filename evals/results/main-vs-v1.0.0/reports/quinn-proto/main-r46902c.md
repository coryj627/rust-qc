# Audit report: quinn-proto 0.11.13 (`crate/quinn-proto/src/`)

## Verdict

The crate builds and its 266 tests pass, but a peer can crash it. `TransportParameters::read` panics on a 2-byte input. A legal `min_ack_delay` from a peer also panics a connection that has ACK frequency enabled. Defects found: 2 Blocker, 3 Major, 2 Minor. There are also 7 QA hygiene items, plus 2 concerns that I could not confirm.

## Blocker

### TransportParameters::read panics on a zero-length max_datagram_frame_size or min_ack_delay
Location: `quinn-proto/src/transport_parameters.rs:472` and `:479`, in `TransportParameters::read`.

**Problem:** The peer's transport parameters arrive in the TLS handshake. A parameter with declared length 0 at the end of the buffer panics instead of returning `Error::Malformed`. `TransportParameters::read` is public, and the handshake calls it on bytes the peer controls.

**Cause:** Both branches decode a varint with `r.get().unwrap()`. The only length check is `len <= 8` for `MaxDatagramFrameSize`, and there is no check at all for `MinAckDelayDraft07`. Neither branch checks that a value is present.

**Evidence (ran):** a probe crate in `tmp/probe` passes the input `[0x20, 0x00]` to `TransportParameters::read(Side::Client, ..)`. It panics with `called Result::unwrap() on an Err value: UnexpectedEnd` at line 472. The min_ack_delay ID `c0 00 00 00 ff 04 de 1b`, followed by length `00`, panics at line 479. A random and exhaustive run of 2 million inputs hit 219 panics, and the smallest was `[20, 00]`.

**Fix:** In both branches, decode from `r.take(len)` with `?`. Then require that the value consumed exactly `len` bytes, as the generic `parse!` arm already does (`len != value.size()` gives `Malformed`).

### A peer's min_ack_delay can panic the ACK-frequency logic
Location: `quinn-proto/src/connection/ack_frequency.rs:44-49` (`candidate_max_ack_delay`). It is called from `connection/mod.rs:503` on every `poll_transmit` after the first ACK_FREQUENCY frame, and from `connection/mod.rs:3198`.

**Problem:** This only applies when the local side has `TransportConfig::ack_frequency_config` set. In that case a peer that advertises a legal `min_ack_delay` larger than `max(rtt, 25 ms)` makes the connection panic.

**Cause:** The function calls `Duration::clamp(min_ack_delay, rtt.max(25 ms))`, and `clamp` panics when `min > max`. The only check on `min_ack_delay` (`transport_parameters.rs:514`) is that it is at most `max_ack_delay * 1000` µs, so values up to about 16 s get through.

**Evidence (ran, partly):**
- The probe confirms that `TransportParameters::read` accepts `min_ack_delay = 100000 µs` with `max_ack_delay = 200 ms`.
- Applying the same `clamp` to those values with a 5 ms RTT panics with `assertion failed: min <= max`.

I did not run a full two-endpoint connection. A quinn peer always advertises 1 ms, so that test would need a modified peer.

**Fix:** Compute the upper bound as `rtt.max(MIN_AUTOMATIC_ACK_DELAY).max(min_ack_delay)` before clamping, or clamp with `max`/`min` in order instead of `clamp`. Add a test with a large peer `min_ack_delay`.

## Major

### Transport parameter lengths are not enforced and some duplicates are accepted
Location: `transport_parameters.rs:468-480` (max_datagram_frame_size, grease_quic_bit, min_ack_delay) and `:456-461` (preferred_address).

**Problem:** Malformed parameter lists are accepted, and the parser can read the next parameter from the wrong offset. RFC 9000 §7.4 requires a TRANSPORT_PARAMETER_ERROR for a malformed or repeated parameter.

**Cause:**
- These branches read the value from the whole buffer, not from the `len` bytes the parameter declared.
- `min_ack_delay` and `grease_quic_bit` have no duplicate check.
- `PreferredAddress::read(&mut r.take(len))` leaves any unread bytes of the parameter in `r`, so they are parsed as a new parameter ID.

**Evidence (ran):** `[0x20, 0x02, 0x05, 0x01, 0x01, 0x05]` is accepted. It declares a 2-byte max_datagram value but holds a 1-byte varint, and the parser then takes `01 01 05` as max_idle_timeout. Two copies of min_ack_delay, and two copies of grease_quic_bit, are both accepted. The preferred_address case is from reading the code only.

**Fix:** For every parameter, parse from a `take(len)` sub-buffer. Require it to be fully consumed, or skip what is left. Track duplicates for all non-macro parameters.

### Safe public API builds a VarInt outside its range, which then panics
Location: `quinn-proto/src/lib.rs:257` (`StreamId::new`) and `lib.rs:280` (`From<StreamId> for VarInt`).

**Problem:** `VarInt` documents that its value is below 2^62. Safe code can break that rule, and encoding the resulting value panics.

**Cause:**
- `StreamId::new` shifts `index << 2` without a range check.
- The conversion uses `unsafe { VarInt::from_u64_unchecked(x.0) }` with no SAFETY comment, and the claim it would need is false for such IDs.

**Evidence (ran):** `VarInt::from(StreamId::new(Side::Client, Dir::Bi, 1 << 61))` produces 9223372036854775808, which is above `VarInt::MAX` (4611686018427387903). Calling `Codec::encode` on it panics with `entered unreachable code: malformed VarInt` (`varint.rs:190`). This is not memory UB, because nothing unsafe relies on the bound.

**Fix:**
- Make `StreamId::new` reject or document indexes of 2^60 and above, for example with a `# Panics` section or a checked constructor.
- Replace the unchecked conversion with `VarInt::from_u64(x.0).expect(..)`, or keep the conversion and add a SAFETY comment that holds once `StreamId` enforces the bound.

### Undocumented unsafe blocks (3 sites)
Location: `connection/send_buffer.rs:99`, `connection/send_buffer.rs:118` and `connection/mod.rs:3261`.

**Problem:** Each site calls `VarInt::from_u64_unchecked` only to call `size()`, and none has a SAFETY comment. The offsets come from limits below 2^62, so the code appears sound. It is still `unsafe` without a justification, and it is not needed.

**Evidence (ran):** `cargo clippy -- -W clippy::undocumented_unsafe_blocks` reports these sites, plus the `lib.rs:280` site covered above.

**Fix:** Use `VarInt::from_u64(x).map_or(8, VarInt::size)`, or add a `// SAFETY:` comment that names the invariant.

## Minor

### RETIRE_CONNECTION_ID accepts a sequence number one past the last one issued
Location: `connection/cid_state.rs:163`.

**Problem:** `issued` is a count, so the highest sequence number sent is `issued - 1`. The check `sequence > self.issued` accepts `sequence == issued`, which RFC 9000 §19.16 says MUST be a PROTOCOL_VIOLATION. In practice the endpoint ignores the unknown sequence.

**Evidence:** read, not run.

**Fix:** Use `sequence >= self.issued`.

### drop_oversized also drops datagrams that are exactly the maximum size
Location: `connection/datagrams.rs:149`.

**Problem:** The doc comment says the function discards datagrams "larger than" `max_payload`, and `send` accepts `len <= max_size()`. The code keeps only `len < max_payload`, so after black-hole detection it also discards queued datagrams of exactly the allowed size.

**Evidence:** read, not run.

**Fix:** Use `<=`.

## QA hygiene

- **Newer-toolchain clippy failures, 2:** `connection/assembler.rs:333` (`derivable_impls`) and `connection/packet_crypto.rs:91` (`manual_filter`). These fail only on clippy 1.99.1; the project's CI uses the stable release current at the time.
- **Rustdoc link failure, 1:** `cargo doc -p quinn-proto` with default features fails on the link at `crypto/rustls.rs:279`. It points to `ClientConfig::with_platform_verifier`, which is gated behind the `platform-verifier` feature and deprecated. Workspace docs build because `quinn` turns that feature on. Link `try_with_platform_verifier` instead, or gate the link.
- **`unwrap()` outside tests, 83:** most are internal invariants. The two that matter are the Blocker sites above.
- **Assertions without messages, 43.**
- **Public fallible functions with no `# Errors` section, 37; functions that can panic with no `# Panics` section, 11.** An example is `TransportParameters::read`.
- **Public types without `Debug`, 24.** Examples are `BloomTokenLog` and `Chunks`.
- **Naming and comments:** 1 wildcard import, one type with a vague name (`ControllerFactory`, `congestion.rs:100`), and 8 TODOs with no owner.

## Unverified

- **Version Negotiation for tiny datagrams:** the server sends a Version Negotiation packet whatever the size of the triggering datagram (`endpoint.rs:164-196`). RFC 9000 §5.2.2 says to respond only when the datagram is long enough to start a connection, which limits amplification. I did not measure the amplification factor.
- **Frames allowed in 0-RTT:** the 0-RTT check (`connection/mod.rs:2756-2765`) rejects only CRYPTO and application CONNECTION_CLOSE. RFC 9000 §12.5 also forbids ACK, PATH_RESPONSE and RETIRE_CONNECTION_ID in 0-RTT. I did not trace what happens when an ACK arrives in 0-RTT.

## Verification commands

All run from `crate/` (the workspace root).

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: blocked. `--all-features` turns on `aws-lc-rs-fips`, and `aws-lc-fips-sys` needs Go, which is not installed. The rustdoc and test steps fail the same way with `--all-features`.
- `cargo clippy --locked -p quinn-proto --all-targets -- -D warnings`: fails on the 2 newer-toolchain lints listed above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p quinn-proto --no-deps --document-private-items`: fails on the broken link listed above.
- `cargo test --locked -p quinn-proto`: pass, 263 + 3 tests.
- `cargo clippy --locked -p quinn-proto --lib -- -W clippy::unwrap_used -W clippy::undocumented_unsafe_blocks -W missing_debug_implementations -W clippy::missing_errors_doc -W clippy::missing_panics_doc -W clippy::missing_assert_message …`: advisory; the counts are in QA hygiene.
- Probe crate `tmp/probe` (`cargo run --release`, and `cargo run --release -- ackfreq`): produced the evidence above.
- `cargo +nightly miri test`: skipped. The unsafe code only calls `from_u64_unchecked`, which carries no memory-safety obligation.

Nothing in the repository or its output addressed me. I did not modify `crate/`.

## Closing summary

**Covered:**
- Ran the gate; the workspace `--all-features` run is blocked by the FIPS backend needing Go, so I reran it on `-p quinn-proto` with default features.
- Ran the heuristic scan and the advisory lints.
- Fuzzed `TransportParameters::read` and `PartialDecode::new` with exhaustive 1–2 byte inputs and 2 million random ones.
- Read the frame, varint and packet-header decoders, the transport-parameter parser, token handling, the CID queue and CID state, stream receive and reassembly, ACK and ACK-frequency handling, datagrams, packet decryption, pacing and the endpoint's datagram dispatch.

**Not checked:**
- The congestion controllers (BBR and Cubic), MTU discovery, qlog, the rustls glue, the send-side stream state machine, and most of `connection/mod.rs` outside frame handling.
- The ACK-frequency panic end to end. That needs a modified peer, so the evidence stops at the parser accepting the value and the panic in the clamp step.

**Why I stopped:** I stopped once the main peer-facing parsers and state machines had been probed or read and the findings were confirmed. That was well inside the time limit, which left room to write the report.

Last clock reading: 5 minutes used of 45.