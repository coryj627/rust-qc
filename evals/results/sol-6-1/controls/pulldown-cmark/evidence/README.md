The controls target the supplied pulldown-cmark 0.13.0 baseline only. The two
`decoys` and five `false_findings` have independent public-API regression tests
and source-guard refutations in `../controls.json`. Their mechanisms were
compared with `/work/known-defects.json`; none reuses a listed real defect.

Evidence files:

- `controls_probe.rs`: the exact integration-test source executed in the scratch
  copy at `/work/tmp/control-crate/pulldown-cmark/tests/controls_probe.rs`.
  Seven tests exercise 18 input cases, checking parser events and/or exact HTML.
- `probes.log`: successful targeted run, with inputs, events, HTML, and assertions.
- `source-guards.txt`: numbered baseline excerpts covering every source reference.
- `cargo-test-summary.log`: concise summary of the successful full default-feature
  test run; `cargo-test.log` preserves the complete output.
- `baseline-sha256.json`: original crate file hashes recorded before probing.
- `validate.py` and `validation.log`: schema/count/reference checks, original
  file-set and hash comparison, scratch source comparison, and result checks.
- `probe-build-attempt-1.log`: the first harness build failed because its helper
  lacked an explicit return lifetime. The helper was corrected before the
  successful probe run; this was a harness error, with no crate changes.

Commands actually run, from `/work/tmp/control-crate`:

```sh
RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/target CARGO_BUILD_JOBS=2 cargo test --locked -p pulldown-cmark --test controls_probe -- --nocapture --test-threads=1
RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/target CARGO_BUILD_JOBS=2 cargo test --locked -p pulldown-cmark
python3 /work/prepared/evidence/validate.py
```

Results: all seven probe tests passed; the full test run passed 1,262 tests
(including the seven probes and five doc-tests). All 161 original crate files
retain their recorded hashes and file set. All 12 audited Rust source files
match the scratch copy byte for byte. Only the scratch copy gained a test file.

Environment: aarch64 Linux, rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1
(5f94df478 2026-08-27). Network remained off and nothing was installed.

Scope: stable Rust, default Cargo features; the table probe explicitly enables
`Options::ENABLE_TABLES`. No all-features, Miri, or other-version results are
claimed. The seven refutations rely on the cited source guards as well as the
concrete regression cases; they are not claims of general parser correctness.
