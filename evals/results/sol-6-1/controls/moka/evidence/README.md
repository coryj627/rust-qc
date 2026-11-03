These artifacts accompany /work/prepared/controls.json for the baseline moka 0.12.11 in /work/crate.

- sync_controls.rs: four executed regression probes (two decoys, two findings).
- future_controls.rs: three executed regression probes (three findings).
- probes.log: exact Cargo invocation, concise results and exit status.
- build-and-probes.log: original complete Cargo output, including compilation.
- source-excerpts.txt: numbered excerpts from original source near every cited location/guard.
- baseline-sha256.json: SHA-256 of every original crate file.
- validate.py and validation.log: executed structural, source-reference, probe-result and original/scratch integrity checks.
- write_controls.py: generator for the delivered JSON.

The scratch crate is /work/tmp/moka-controls. It was copied before probes, and its only differences from the original are appended cfg(test) module registrations in src/sync/cache.rs and src/future/cache.rs. Those registrations load the two preserved probe files by absolute path. No production algorithm was changed. The original crate was never edited.

The tests use Cache operations and existing test-only helpers: Clock::mock controls time, reconfigure_for_testing disables automatic maintenance, and is_waiter_map_empty observes initialization cleanup. Explicit run_pending_tasks calls establish admitted entries where needed. The async cancellation test polls both callers to Pending before dropping the initializer; it uses a two-second timeout only as a failure bound. No network or harmful external behavior is involved.

To rerun against the preserved scratch copy, use the command in probes.log. To reconstruct elsewhere inside this sandbox, make a fresh copy of /work/crate and append the following module registrations, selecting sync_controls.rs for sync/cache.rs and future_controls.rs for future/cache.rs:

    #[cfg(test)]
    #[path = "/work/prepared/evidence/sync_controls.rs"]
    mod negative_control_probes;

All seven probes passed under the installed default Rust 1.99.1 on aarch64 Linux, offline and locked, with sync,future enabled. The crate's 155 unrelated unit tests were filtered out; the full suite and Miri were not run. Source guards, in addition to the reachable API counterexamples, establish why each stated control claim is false.

Known-defect separation was reviewed by behavior: none asserts or negates the stale-upsert orphan, compute waiter/write race, timer-wheel dangling pointer, deque Tree Borrows aliasing, or expired-entry custom-Expiry update defect. In particular, the weight control concerns the admitted-entry update branch and correct arithmetic, with completed maintenance between writes; it does not claim stale upserts are safe.
