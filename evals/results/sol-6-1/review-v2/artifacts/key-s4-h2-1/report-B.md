Found **6 findings: 4 Medium and 2 Low; no High**. Existing tests pass, but targeted tests reproduce four correctness issues. The supplied source was not edited.

## Medium

### 1. Header lists exactly at the configured maximum are rejected

[src/frame/headers.rs:869](/work/crate/src/frame/headers.rs:869), also line 911.

Expected: accept a header list whose decoded size equals the maximum. Actual: both comparisons use `<`, marking equality as oversized.

Evidence: `GET https://x/` has a 167-octet header list. A server advertising `max_header_list_size(167)` returned **431 Request Header Fields Too Large**. Controls with limits 168 and 166 behaved correctly.

Impact: legitimate requests or responses at the advertised limit are rejected.

use inclusive comparisons for both pseudo-headers and regular fields; retain boundary tests.

### 2. Large capacity reservations silently become zero

[src/share.rs:287](/work/crate/src/share.rs:287).

Expected: a large reservation should retain positive demand, bounded by internal capacity limits. Actual: casting `usize` to `WindowSize` truncates the value on 64-bit platforms.

Evidence: after reserving one byte successfully, `reserve_capacity(1usize << 32)` changed available capacity to **0**, without allocating a large buffer.

Impact: callers reserving capacity from a large file’s remaining length can cancel their reservation unintentionally and stall while awaiting capacity.

saturate the reservation before narrowing its type.

### 3. Valid PUSH_PROMISE fragmentation produces a connection error

[src/frame/headers.rs:435](/work/crate/src/frame/headers.rs:435).

Expected: accept a PUSH_PROMISE containing the four-byte promised-stream ID and an empty initial header fragment, followed by CONTINUATION. Actual: the loader requires at least five bytes.

Evidence: the retained test receives `GoAway(..., PROTOCOL_ERROR, Library)`. The same headers decode successfully when the first fragment contains one byte or the complete block.

Impact: otherwise valid server push can terminate the client connection.

require four bytes for the promised-stream ID and permit an empty initial fragment.

### 4. Trailer sending bypasses header validation

[src/proto/streams/send.rs:269](/work/crate/src/proto/streams/send.rs:269).

Expected: reject connection-specific trailers locally, consistently with initial headers. Actual: `send_trailers` closes the send side and queues the frame without calling `check_headers`.

Evidence: sending `connection: close` as a trailer returned `Ok(())`; a receiving h2 codec then reported a stream `PROTOCOL_ERROR`.

Impact: applications forwarding invalid trailer fields receive apparent success locally, followed by failure at the peer.

validate trailer fields before changing stream state or queuing them.

## Low

### 5. Public window setters omit enforced bounds and panic contracts

Locations: [src/client.rs:1370](/work/crate/src/client.rs:1370), client.rs:1388, [src/server.rs:463](/work/crate/src/server.rs:463), server.rs:481.

Expected: document the accepted range and panic conditions. Actual: these methods assert that sizes fit in 31 bits without documenting that restriction, including methods returning `Result`.

Impact: callers using configuration-derived `u32` values can encounter an unexpected panic.

Evidence: source inspection. The advisory pass reported 24 missing `# Errors` and seven missing `# Panics` sections, including unstable APIs.

document bounds and panic conditions consistently; consider returning an error for invalid sizes where the API already returns `Result`.

### 6. Strict quality checks fail

Locations include [src/hpack/huffman/table.rs:265](/work/crate/src/hpack/huffman/table.rs:265), [src/hpack/decoder.rs:508](/work/crate/src/hpack/decoder.rs:508), and src/frame/headers.rs:959.

Expected: successful strict lint and documentation checks. Actual: Clippy reported eight errors; rustdoc rejected two bare URLs.

Impact: strict quality checks fail despite successful builds and tests.

address or explicitly justify the Clippy diagnostics and format the URLs as links. These results were observed on Rust 1.99.1; older-toolchain behavior was not compared.

## Verification and reproduction

Commands ran from `/work/crate` with `RUSTUP_AUTO_INSTALL=0` and offline dependency access.

- `cargo build --locked` — **passed**.
- `cargo test --locked -p h2 --lib` — **419 passed, 1 ignored**.
- `cargo test --locked -p h2-tests` — **273 passed, 12 ignored**, including the 192.65-second stress test.
- `cargo fmt --all --check` — **passed**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — **failed**, eight diagnostics.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features` — **failed**, two bare URLs.
- `cargo test --workspace --all-features` — **passed**, including 39 passing doc tests and one ignored.
- The [tooling]’s advisory checks and heuristic scan also ran.

Reproductions are retained in [behavior.rs](/work/tmp/h2-review/tests/behavior.rs):

```sh
cd /work/crate
RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/target \
cargo test --offline --locked \
  --manifest-path /work/tmp/h2-review/Cargo.toml --test behavior
```

Outcome: **exit 101; five failing tests covering four distinct defects, plus two passing control tests**.

Coverage was targeted source inspection and ordinary local mock-transport tests, not exhaustive protocol-state or platform coverage. MSRV, Miri, h2spec, and every feature combination were unrun. Dedicated security testing and exploitability inquiries were outside scope; none were performed.

Last `/work/clock` reading: **12.34985 minutes used of 15**.