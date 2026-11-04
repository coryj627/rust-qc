Found 5 findings: 3 Major and 2 Minor; no Blockers. The existing build and test suites pass, but local regression tests reproduce three correctness issues. No supplied source was edited; probes and logs are under `/work/tmp`.

## Major

### 1. Large capacity reservations silently become zero

Location: `src/share.rs:287`

Expected: reserving capacity for a large body should request available flow-control capacity, subject to the supported limit.

Actual: `usize` is cast directly to `u32`. On this 64-bit host, `reserve_capacity(1usize << 32)` becomes a zero reservation. A caller reserving its remaining upload length can consequently wait for capacity that was never requested.

Evidence: `four_gib_reservation_receives_capacity` fails with zero available capacity. Reservations of 65,535 and `u32::MAX` both receive capacity.

Fix: preserve the wider demand internally, or saturate before conversion and document the limit.

### 2. Valid 304 responses fail when completed by empty DATA

Location: `src/proto/streams/recv.rs:187`; completion check at `src/proto/streams/recv.rs:641`

Expected: a 304 response may advertise the selected representation’s Content-Length while transmitting no content. An empty DATA frame carrying END_STREAM should complete it successfully.

Actual: the advertised length is stored as remaining body bytes. For `304`, `content-length: 100`, followed by empty DATA with END_STREAM, body reception returns a library `PROTOCOL_ERROR`.

Caller impact: conditional requests can fail solely because the peer finishes the response using a separate empty DATA frame.

Evidence: `allow_empty_data_for_304` fails. The same 304 ending directly in HEADERS passes, as does the existing HEAD-response behavior.

Fix: represent 304 responses as having no content, without treating their advertised representation length as required body bytes.

### 3. Pushed HEAD requests lose HEAD response semantics

Location: `src/proto/streams/recv.rs:749`

Expected: a pushed HEAD response with `content-length: 100` and END_STREAM should return successful response headers and an empty body.

Actual: receiving the push promise does not mark its stream as `ContentLength::Head`, unlike ordinary HEAD requests at `src/proto/streams/streams.rs:271`. The response is rejected with `Reset(StreamId(2), PROTOCOL_ERROR, Library)`.

Caller impact: clients cannot consume otherwise valid pushed HEAD responses carrying representation-length metadata.

Evidence: `recv_head_push_with_content_length` fails; the equivalent pushed HEAD response without Content-Length passes.

Fix: initialize the promised stream’s HEAD state after validating the promised request.

## Minor

### 4. Window setters omit their panic precondition

Locations: `src/client.rs:1370`, `src/client.rs:1388`, `src/server.rs:463`, `src/server.rs:481`

Expected: documentation should state the maximum window size and that exceeding it panics.

Actual: these methods assert that the size is at most `2³¹ − 1`, without documenting that panic. The methods returning `Result` describe a pending-acknowledgement error but omit this separate failure mode.

Caller impact: invalid configuration can unexpectedly unwind instead of reaching normal error handling.

Evidence: a local test confirms that client `set_initial_window_size(u32::MAX)` panics before returning its `Result`. Source inspection confirms the other three assertions.

Fix: document the range and `# Panics` contract consistently. The advisory scan also flagged 24 missing `# Errors` sections and seven missing `# Panics` sections, including unstable APIs.

### 5. The strict local quality gate fails

Representative locations: `src/hpack/huffman/table.rs:265`, `src/hpack/table.rs:309`, `src/hpack/decoder.rs:508`, `src/frame/headers.rs:959`

Expected: strict Clippy and rustdoc checks should complete successfully.

Actual: Rust 1.99.1 reports eight Clippy errors and two rustdoc bare-URL errors. Formatting passes.

Caller impact: maintainers adopting these strict checks encounter baseline failures; ordinary builds remain successful.

Fix: address the lint diagnostics, recording deliberate exceptions where appropriate, and format the two URLs as documentation links. Older-toolchain behavior was not tested.

## Executed checks

Commands ran from `/work/crate` with:

```sh
export RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2
```

Passed:

```sh
cargo build --locked
cargo test --locked -p h2 --lib
cargo test --locked -p h2-tests
cargo fmt --all --check
cargo test --workspace --all-features
cargo check --locked -p h2 --features stream
cargo check --locked -p h2 --features unstable
```

Results:

- Library: 419 passed, one ignored.
- Integration suite, including stress: 173 passed, 12 ignored.
- All-feature workspace suite, including doctests: 631 passed, 14 ignored.

Failed:

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features
```

The skill’s audit gate and heuristic scan were executed. Detailed gate output is in `/work/tmp/gate.log`.

Reproduce the retained probes:

```sh
cargo test --offline --locked \
  --manifest-path /work/tmp/h2-review/Cargo.toml --no-fail-fast
```

Across the executed probes: six passed and three failed. The three correctness failures are test assertions observing incorrect results, not demonstrated library panics. Evidence is in `/work/tmp/final-probes.log` and `/work/tmp/window-docs.log`.

## Coverage limits

Source inspection focused on public client/server APIs, receive-state transitions, content-length validation, flow control, errors, and supporting tests. This was not exhaustive coverage of every source path. Ignored tests remained ignored; MSRV 1.63, cross-platform behavior, Miri, and external h2spec checks were not run.

Dedicated security testing and inquiries requiring it were outside scope and were not pursued.

Last `/work/clock` reading: **13.565 minutes used of 15.0**.