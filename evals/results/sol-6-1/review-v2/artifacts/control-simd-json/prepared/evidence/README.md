Evidence for seven negative controls against the supplied simd-json 0.16.0 baseline.

- `negative_controls.rs` contains seven independent public-API tests, one per control. It is byte-identical to the integration test added only in `/work/tmp/control-crate/tests/negative_controls.rs`.
- `run-probes.sh` records the exact successful command and scratch directory. It runs offline with default features, stable Rust 1.99.1, on aarch64 Linux, in the debug test profile. `RUSTFLAGS="--cap-lints warn"` is the supplied compatibility workaround; crate source was not edited.
- `probe-run.log` contains the successful build/test output: seven passed, zero failed, exit code zero. The seven per-ID `.log` files extract the actual observations. Assertions check expected results and errors rather than only printing output.
- `probe-build-attempt-1.log` preserves an initial harness compilation failure: the buffer-reusing Serde entry point was initially referenced at the crate root. The harness was corrected to `simd_json::serde::from_slice_with_buffers` before the successful run. No tests executed in that first attempt.
- `source-excerpts.txt` contains line-numbered guards/invariants read from `/work/crate`, matching the relative source references in `controls.json`.
- `baseline-sha256.json` records all original crate files before the scratch tests. `validation.log` records the final schema, reference, test-log, scratch-source, and unchanged-baseline checks.

The known-defect list was read to exclude its six entries. These controls do not claim the nested object-key tape slicing defect, huge-exponent or long-decimal defects, lone-high-surrogate behavior, NUL numeric terminators, or fixed-length Serde targets leaving unread elements. The tape probe uses `as_array` plus `Array::get`; the ignored-field probe uses normal recursive unknown-field handling rather than an early-stopping fixed-length visitor.

Limits: these are targeted ordinary functional regression probes, not the full crate test suite or a feature/platform matrix. No Miri run, security assessment, external interaction, or network access was used. The claims are narrowly tied to the demonstrated public-API triggers and corresponding baseline invariants.
