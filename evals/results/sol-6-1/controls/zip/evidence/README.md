Evidence for zip 5.1.1 negative controls

- controls_probe.rs is the complete public-API regression probe, copied unchanged to /work/tmp/zip-control-probe/tests/negative_controls.rs.
- probes.log is the final passing run: seven tests, zero failures, all features, locked dependencies, stable Rust 1.99.1 on aarch64 Linux.
- source-lines.txt records the exact baseline source lines cited by controls.json. References use paths relative to /work/crate.
- baseline-sha256.json records the original crate inventory and file hashes. validation.log confirms the original crate is unchanged and all original files in the scratch copy still match.
- probe-development.log retains the initial test output without compiler progress. Its two failures were probe assumptions: the Unicode-checksum rejection is surfaced as "Could not find EOCD" by EOCD retry logic, and a 30-byte encrypted fixture selects AE-1. The final probe checks archive rejection without assuming the internal error text, and uses a nine-byte fixture to exercise AE-2 with CRC zero. These are fixture/assertion corrections; no library code was changed.

Executed from /work/tmp/zip-control-probe:

    CARGO_BUILD_JOBS=2 cargo test --locked --all-features --test negative_controls -- --nocapture --test-threads=1

Only the seven targeted regression tests were run. The full suite and doctests were not run. All archive bytes were generated and modified in memory; no extraction, outside writes, large allocations, network requests, or tool installation were needed. The known-defect list was checked to keep these controls distinct from its eight real defects. These refutations establish the specified claims are false at this baseline; they do not assert broader archive safety.
