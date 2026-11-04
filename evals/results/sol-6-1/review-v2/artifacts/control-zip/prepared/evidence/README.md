Seven functional negative controls for baseline zip 5.1.1.

Inputs were /work/crate and /work/known-defects.json. The controls concern CRC empty-buffer reads, leap-day validation, duplicate-name rejection, append comment preservation, entry-relative seeks, raw copying, and directory compression. None relabels the three supplied known defects (empty-archive append offset, Shrink code 8191, stream metadata callback).

All seven counterexamples ran successfully as public-API integration tests in a scratch copy. Archives were small in-memory Cursor<Vec<u8>> values. No extraction or external behavior was involved.

Reproduction:

    cp -a /work/crate /work/tmp/zip-controls
    cp /work/prepared/evidence/negative_controls.rs /work/tmp/zip-controls/tests/negative_controls.rs
    cd /work/tmp/zip-controls
    cargo test --locked --all-features --test negative_controls -- --nocapture

The scratch copy already exists. For a fresh reproduction, use a new scratch directory or remove only the existing scratch copy first.

Files:
- negative_controls.rs: exact integration-test source that ran.
- probes.log: concise test results and command; all seven passed.
- build-and-probes.log: unabridged cargo output, including a benign compiler warning about the deliberately invalid UTF-8 comment literal.
- source-excerpts.txt: numbered excerpts of guards and paths read from the original crate.
- baseline-sha256.json: initial SHA-256 hashes of every original-crate file.
- validation.log: schema/reference checks and final original-crate integrity comparison.
- write_controls.py: script that wrote controls.json.

Environment: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1 (5f94df478 2026-08-27), aarch64-unknown-linux-gnu. Cargo ran offline with the supplied lockfile and cached dependencies; nothing was installed.

Limits: The executed test target was negative_controls with all features enabled. The full baseline test suite, doctests, other feature combinations, other platforms, and Miri were not run. The supplied note identifies a doctest exceeding available memory; this integration-test command does not execute doctests. Each claim is additionally contradicted by the cited baseline source guard or implementation path. This is a set of negative controls, not an exhaustive defect inventory.
