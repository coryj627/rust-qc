Baseline: moka 0.12.11, /work/crate. Features used: sync,future.

control_probes.rs contains exactly seven focused regression tests, one per
control. The probes were run as a cfg(test) sibling module in the scratch copy
/work/tmp/moka-control-probe. Its src/lib.rs has only an appended cfg(test)
module declaration; all pre-existing production logic is unchanged.

probe-run.log is the final actual cargo invocation output (7 passed, 0 failed).
The final invocation rebuilt the probes after adding an explicit flag proving
that the cancelled initializer in false-4 had begun execution before drop.

Time-dependent probes use the crate's existing test-only Clock::mock. The
existing reconfigure_for_testing disables automatic housekeeping and enables the
frequency sketch; these probes use no eviction pressure. False-1 deliberately
uses a timestamp strictly before the invalidation cutoff. False-3 tests both
an iterator created before expiry and a new iterator after expiry; source
inspection additionally establishes per-key revalidation of key-list entries.
False-4 uses one pending initializer, proves it was polled, drops it, and retries
on the same key. Its two-second timeout is a test bound only.

source-excerpts.txt contains line-numbered evidence from the original crate.
baseline-sha256.json records SHA-256 hashes of all original crate files before
copying or building. validation.log records schema/source-reference checks and
an exact subsequent comparison of the original crate against those hashes.

replay.sh can reconstruct and run the same tests in a fresh scratch directory.
write_controls.py generated controls.json. Neither helper script is represented
as having been executed unless recorded in the tool command history.

Scope: focused functional probes and source inspection. The full upstream test
suite, Miri, alternate feature combinations, and concurrency stress testing were
not run. No toolchain or dependency installation was performed; cargo was offline
and locked. The original crate was never used as a build or edit destination.
The supplied known-defect key was checked for conceptual overlap: these controls
do not concern stale Upsert/deque admission, concurrent future compute update
ordering, or expiry-after-update returning None on an expired entry.
