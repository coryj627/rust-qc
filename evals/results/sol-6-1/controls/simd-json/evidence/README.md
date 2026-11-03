These are seven independently constructed negative controls for the unchanged simd-json 0.16.0 baseline. Claim text and decisive refutations are separated in ../controls.json.

Validation ran on installed stable Rust 1.99.1, aarch64 Linux, with default features, the original Cargo.lock, offline mode, and RUSTFLAGS='--cap-lints warn'. Only a new integration test was added to /work/tmp/control-probe; the original crate was not edited. No dependency was installed. The three build warnings in probe.log are the newer-compiler unused-import warnings described in the task.

Evidence:
- negative_controls.rs: all seven safe public-API regression tests.
- run-command.txt: the exact executed cargo command, using the scratch copy.
- probe-summary.log: concise test results; probe.log: complete build/test output.
- source-guards.txt: numbered excerpts of baseline guards and locked dependency implementations used by those guards.
- baseline-sha256.json: original file hashes, recorded before the scratch copy and tests.
- validate_controls.py and validation.log: schema/count/location checks, unchanged-original verification, scratch source/manifest/lockfile identity, and test-log checks.
- toolchain.txt: actual compiler and architecture.
- write_controls.py: artifact generation source.

The known-defect list was used only to exclude real defects from the claim set. In particular, decoy-2 is specifically ordinary unknown-field IgnoredAny traversal, whose visitor consumes every descendant, rather than fixed-length target deserialization that stops early. false-1 uses fresh deserializers and alleges integer narrowing, rather than exhausted-deserializer reads. decoy-1 uses whitespace-separated second roots, and false-4 uses control bytes inside quoted strings; neither alleges the real NUL-after-number behavior. None alleges nesting limits, duplicate-key insertion behavior, tape get_array/get_object slicing, long decimals/exponents, lone surrogates, NEON allocation alignment, or hand-written MapAccess misuse.

Limits: the focused probes and source guards refute these seven specific claims, not every possible defect in these subsystems. No Miri, alternate-feature, or cross-architecture runs were performed. The ownership probe checks all keys/leaves are Cow::Owned before touching or dropping its input.
