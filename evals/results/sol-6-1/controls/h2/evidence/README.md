This evidence accompanies exactly two defect-key decoys and five verifier findings in ../controls.json, authored against /work/crate (h2 0.4.12).

All seven claims were disproved. Five bounded integration tests use the existing in-memory transport or raw codec, and two unit tests exercise internal state invariants. The unit tests for negative windows and shutdown PINGs are not complete connection-level reproductions; controls.json explicitly separates their runtime observations from source-based scheduler/state-machine evidence. No network, tool installation, Miri run, stress test, or full upstream test-suite run was used.

Probe source:
- probes/negative_controls.rs: five integration tests, copied to tests/h2-tests/tests/negative_controls.rs in the scratch crate.
- probes/flow_control_probe.rs: one #[cfg(test)] module appended to src/proto/streams/flow_control.rs in the scratch crate.
- probes/ping_pong_probe.rs: one #[cfg(test)] module appended to src/proto/ping_pong.rs in the scratch crate.

The scratch crate is /work/tmp/control-probe-crate. All baseline production code is byte-for-byte unchanged in the scratch copy, apart from appending those test-only modules. /work/crate itself was never edited. baseline-sha256.json records all 521 original files; validate.py verifies the original file set and hashes, the limited scratch changes, the schema, IDs, references and final test logs. scratch-changes.diff preserves the test-only source additions.

Commands actually run (in /work/tmp/control-probe-crate, with RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR=/target/control-probes):
- cargo build --locked -> logs/build.log (passed)
- cargo test --locked -p h2-tests --test negative_controls -- --nocapture -> logs/integration-probes.log (5 passed)
- cargo test --locked -p h2 --lib negative_control_probe -- --nocapture -> logs/unit-probes.log (2 passed; 420 upstream tests filtered out)

The initial integration run passed the capacity-release assertions but its cleanup expected GOAWAY before the RST_STREAM(CANCEL) emitted when dropping an unfinished body. The test cleanup was corrected to expect CANCEL first, then the five tests passed. The earlier log is retained as logs/integration-probes-initial-teardown-failure.log; no product-code fix was made.

To reproduce in a fresh scratch directory, copy /work/crate, install the three probe files as described above, then run the listed Cargo commands in that directory. Reuse the offline Cargo home and target directory. write_controls.py is the artifact generator; it does not run the probes.

Known-defect exclusion review (known-defects.json used only to avoid relabeling real defects):
- h2-1: unbounded queued DATA events. No control alleges frame/event-count resource bounds.
- h2-2: HeaderMap capacity panic from huge unique header lists. The ordering control uses one tiny ordinary field and tests the existing pseudo-header guard.
- h2-3: poll_write(Ok(0)) busy loop. No control concerns write progress or zero-byte transport writes.
- h2-4: unlimited reserved push streams. No control concerns push stream accounting.
- h2-5: conflicting duplicate Content-Length. No control concerns Content-Length.
- h2-6: missing :status defaults to 200. No control concerns missing response status.
- h2-7: GOAWAY initiator filtering. The PING control concerns retaining a mismatched shutdown ACK, not the last_stream_id closing logic.
- h2-8: DATA dropped above local GOAWAY without accounting. The capacity-release control concerns caller over-release validation, not DATA after GOAWAY.
