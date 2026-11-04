The existing tests pass, but additional local checks expose failures on ordinary empty results, optional metadata and callback reuse. I found 8 High, 2 Medium and 4 Low findings. No repository source was changed; regression tests and compile-only examples are in [review-harness](/work/tmp/review-harness).

## High

### 1. Empty native results produce invalid Rust slices

Locations: [buf.rs:47](/work/crate/src/buf.rs:47), [remote.rs:387](/work/crate/src/remote.rs:387), [diff.rs:1281](/work/crate/src/diff.rs:1281).

Expected: empty buffers, reference advertisements and binary payloads are readable as empty slices.

Observed: these wrappers pass null pointers with length zero to `slice::from_raw_parts`, which requires a non-null pointer even for empty slices. Separate regression tests for `Buf::new().is_empty()`, listing references from a newly initialized local repository, and reading a binary-only diff payload each terminated with SIGABRT on Rust 1.99.1.

Consequence: ordinary inspection can terminate the application. Normalize empty results before constructing slices; also fix `Buf::deref_mut` at line 53. Add empty-result regression tests.

### 2. Nullable metadata getters panic on valid objects

Locations: [merge.rs:56](/work/crate/src/merge.rs:56), [rebase.rs:319](/work/crate/src/rebase.rs:319), [refspec.rs:37](/work/crate/src/refspec.rs:37).

Executed tests reproduced three cases:

- `find_annotated_commit(id)?.refname()` panics because lookup by ID supplies no reference name.
- `exec()` on an ordinary rebase `Pick` panics, despite documentation saying this field is populated only for `Exec`.
- `dst()` for the accepted fetch refspec `refs/heads/main` panics because its destination is absent.

Expected: the optional string accessors return `None`. Observed: their implementations unwrap nullable native fields.

Consequence: callers cannot safely inspect otherwise valid objects. Propagate absence through the string accessors and provide optional byte accessors where the existing slice return type cannot represent absence. Existing smoke tests omit these cases.

### 3. Moving configured callback options leaves stale payload pointers

Locations: [apply.rs:127](/work/crate/src/apply.rs:127), [apply.rs:140](/work/crate/src/apply.rs:140), [stash.rs:132](/work/crate/src/stash.rs:132).

Expected: configured Rust options remain usable after ordinary moves, including returning them from a helper or placing them in a `Box`.

Observed: callback setters store the current address of `self`; `raw()` does not refresh it. Two executed tests configured callbacks, moved the options into boxes, and confirmed that their exported payloads still contained the previous stack addresses.

Consequence: subsequent native callbacks access the old location. This was established by pointer comparison and source inspection; callbacks were not invoked through stale pointers.

Refresh payloads immediately before native calls, or store callback state at a stable owned address. Add move-after-configuration tests.

### 4. `into_inner` leaves the old revision filter installed

[revwalk.rs:43](/work/crate/src/revwalk.rs:43).

Expected: consuming `RevwalkWithHideCb` releases its callback and returns an independently usable walker.

Observed: `into_inner()` only resets the native walker. Native reset preserves its callback and payload. An executed three-commit regression subsequently pushed the tip and hid the base: the supposedly detached callback ran four times and incorrectly removed the tip.

Consequence: results remain filtered unexpectedly. Moreover, the returned type releases the Rust callback borrow while native code retains its pointer.

Explicitly unregister the callback before returning the walker. The existing test misses this because its subsequent walk does not reactivate limited traversal.

### 5. A refspec can outlive its owning remote

[remote.rs:283](/work/crate/src/remote.rs:283).

Expected: a borrowed `Refspec` cannot outlive its `Remote`.

Observed: `get_refspec` returns `Refspec<'repo>`, tying it to the repository instead. A compile-only example successfully returns a refspec obtained from a local remote that is dropped before the function returns. Supporting native source confirms that `git_remote_free` frees these refspecs.

Consequence: ordinary helper functions can return dangling handles through safe Rust.

Tie the return lifetime to the receiver borrow. Evidence is successful compilation plus ownership inspection; expired data was not accessed.

### 6. Pathspec diff matches can outlive their diff

[pathspec.rs:63](/work/crate/src/pathspec.rs:63).

Expected: matching diff entries remain valid for the lifetime of the returned match list.

Observed: `match_diff` ties its result only to `self`. A compile-only example successfully returns matches from a locally created diff. Native implementation inspection shows the match list stores pointers to the original diff deltas without retaining or copying their owner.

Consequence: callers can retain a match list after its entries have been freed.

Borrow both the pathspec and diff for the result’s lifetime, or retain owned native data. Add a compile-fail lifetime regression.

### 7. Rebases can outlive their callback options

Locations: [repo.rs:2289](/work/crate/src/repo.rs:2289), [repo.rs:2313](/work/crate/src/repo.rs:2313).

Expected: callbacks and checkout configuration used by later `next()` or `abort()` calls remain alive.

Observed: returned rebases borrow only the repository. A compile-only example successfully returns a rebase configured with local options and a callback capturing a local variable. Native `rebase_alloc` shallow-copies the checkout options, including their pointers; it does not retain the Rust owners.

Consequence: subsequent rebase operations can access dropped callback or configuration state.

Make the rebase retain ownership or a borrow of the required options. Evidence is compilation and source inspection; the expired callback was not executed.

### 8. Message prettification panics on legacy-encoded bytes

[message.rs:32](/work/crate/src/message.rs:32).

Expected: the `IntoCString` API either produces a valid `String` or returns an error for non-UTF-8 output.

Observed: `message_prettify(&b"caf\xe9\n"[..], None)` panicked at `as_str().unwrap()` in an executed regression.

Consequence: processing a legitimate legacy-encoded Git message unwinds instead of returning the advertised `Result`.

Convert UTF-8 validation failure into an error and test both UTF-8 and legacy-encoded messages.

## Medium

### 9. `skip_reuc` changes conflict handling instead

[merge.rs:98](/work/crate/src/merge.rs:98).

Expected: `skip_reuc` controls writing the REUC extension independently of conflict handling.

Observed: it changes `GIT_MERGE_FAIL_ON_CONFLICT`. Executed conflicting-merge tests showed that `skip_reuc(true)` returns error −24 instead of a conflicted index, while `fail_on_conflict(true).skip_reuc(false)` unexpectedly permits that index.

Consequence: callers receive different merge outcomes merely by configuring extension output.

Use `GIT_MERGE_SKIP_REUC` and test both settings independently and together.

### 10. `Time` ordering disagrees with equality

[time.rs:57](/work/crate/src/time.rs:57).

Expected: `cmp` returns `Equal` exactly when `Eq` does.

Observed: derived equality includes the timezone sign, while ordering compares only seconds and offset. An executed test read a commit signature containing `-0000` and compared it with the equivalent `Time::new(..., 0)`: ordering reported equality, but `Eq` reported inequality.

Consequence: ordered collections treat distinct values as the same key.

Make equality and ordering use the same fields, including the sign if its distinction is retained. Add negative-zero timestamp coverage.

## Low

### 11. Public failure contracts are incomplete

Representative locations: [build.rs:177](/work/crate/src/build.rs:177), [pathspec.rs:172](/work/crate/src/pathspec.rs:172).

The advisory pass reported 428 missing `# Errors`, 117 missing `# Panics` and 17 missing `# Safety` sections in `src/`. Some constraints already appear in prose, and some panic diagnostics concern invariants; these are diagnostic counts, not additional bug counts.

Confirmed omissions include NUL-triggered builder panics and path restrictions enforced through `unwrap`. Document actual caller obligations and recoverable failures.

### 12. Public handles frequently lack `Debug`

Representative locations: [index.rs:17](/work/crate/src/index.rs:17), [repo.rs:109](/work/crate/src/repo.rs:109).

The advisory pass identified 113 public types without `Debug`. This prevents standard diagnostic patterns such as calling `unwrap_err()` on a result whose successful value is `Index`.

Provide useful manual implementations where deriving would expose inappropriate internals or require formatting callbacks.

### 13. Native invariants lack consistent justification

Representative locations: [oid_array.rs:24](/work/crate/src/oid_array.rs:24), [rebase.rs:88](/work/crate/src/rebase.rs:88).

The advisory pass emitted 799 undocumented-block diagnostics across 798 distinct reported locations, plus eight unsafe-implementation diagnostics. Some sites contain equivalent prose.

The concrete maintenance concern is that nullability and owner-lifetime assumptions are difficult to verify—the same assumptions implicated above. Document validated native contracts at these boundaries; missing comments alone are not evidence of further undefined behavior.

### 14. Broad lint allowances weaken the documentation gate

Locations: [lib.rs:69](/work/crate/src/lib.rs:69), [lib.rs:150](/work/crate/src/lib.rs:150).

There were 109 allowance-without-reason diagnostics across 16 distinct locations, including macro expansions. Generated public convenience methods explicitly bypass `deny(missing_docs)`, so a passing documentation check does not establish complete public documentation.

Document generated methods and keep narrowly justified allowances or expectations.

## Verification

All commands used installed tools and prefetched dependencies, with `RUSTUP_AUTO_INSTALL=0`, `CARGO_NET_OFFLINE=true` and two build jobs. Host: aarch64 Linux, Rust/Cargo 1.99.1.

From `/work/crate`:

- `cargo build --locked -p git2 --no-default-features` — passed.
- `cargo test --locked -p git2 --no-default-features` — passed: 187 unit tests, four integration tests and 12 doctests; one doctest ignored.
- `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` — formatting, workspace all-feature documentation and workspace all-feature tests passed; strict Clippy failed.
- `cargo clippy --locked -p git2 --all-targets --no-default-features -- -D warnings` — also failed before reaching the reviewed package, at pre-existing `libgit2-sys/build.rs:41` and `:68`: `unnecessary_map_or` and `needless_borrows_for_generic_args`. No older toolchain was tested, so this is not established as newer-toolchain-only drift.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p git2 --no-deps --document-private-items --no-default-features` — passed.
- `cargo run --locked -p systest` — passed all 4,523 ABI checks, with generated-code warnings.
- `cargo +nightly-2026-10-01 miri test --locked -p git2 --no-default-features --lib time::tests::smoke` — passed the single selected native-free test.
- `python3 -I /work/[tooling]/scripts/[tooling] /work/crate` — completed. Inspection of naming and TODO candidates produced no additional in-scope findings.

From `/work/tmp/review-harness`:

- `cargo test --offline -- --skip empty_` — one control test passed; ten regressions failed as described above.
- Six separate `cargo test --offline NAME -- --exact tests::NAME` invocations exercised empty results: buffer, remote advertisement and binary payload tests aborted; empty blob, prettified message and configuration-string tests passed.
- `cargo check --offline --examples` — all three owner-lifetime examples unexpectedly compiled.
- Scratch-harness formatting, strict Clippy and warning-denying documentation checks passed.

Total: 14 findings—8 High, 2 Medium and 4 Low. Coverage was limited to local correctness review and the host platform; full-suite Miri, other targets, historical toolchains and network behavior were not tested. No dedicated security testing was performed. Last clock reading: 17.05855 of 45 minutes used, 1,023.8 seconds active time.