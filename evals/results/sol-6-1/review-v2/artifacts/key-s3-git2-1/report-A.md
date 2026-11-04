The required build and tests pass, but focused checks found failures in ordinary boundary cases and several ownership errors. There are **21 findings: 6 High, 11 Medium and 4 Low**. No production Rust source was edited; regression harnesses and logs are under `/work/tmp`.

All source locations below are relative to `/work/crate`.

## High

### 1. Empty collections create invalid slices and abort

`src/buf.rs:47`, `src/buf.rs:53`, `src/remote.rs:387`, `src/diff.rs:1281`.

Empty buffers, reference lists and binary data should be readable as empty slices. These methods pass null pointers to `slice::from_raw_parts` or `from_raw_parts_mut`, which require non-null pointers even at length zero.

Executed tests independently produced SIGABRT for:

- `Buf::new().is_empty()`.
- Mutably borrowing a new `Buf`.
- Listing references from an empty bare remote.
- Reading binary data from a valid `Binary files … differ` patch marker.

Return empty slices when the length is zero. Existing tests miss these empty-result paths.

### 2. Signatures of edited blame lines contain null pointers

`src/blame.rs:110`, `src/blame.rs:131`; dereference at `src/signature.rs:73`.

`blame_buffer` documents newly changed lines as having a zero commit ID. Such hunks also have absent C signatures, but these getters construct an ordinary `Signature` without checking for absence.

An executed test changed one committed line, obtained its blame hunk and called `final_signature().name()`. It aborted with a null-pointer dereference. Source inspection shows the same problem for `orig_signature`.

Represent missing signatures with `Option`. The existing blame test checks the changed line’s zero OID but never queries its signature.

### 3. Extracting a revwalk leaves its callback registered

`src/revwalk.rs:43`.

`into_inner` releases the Rust callback borrow but only resets the C walker. Inspection of vendored `git_revwalk_reset` confirms that it leaves the callback and payload installed.

An executed two-commit test expected two visited commits and zero callback calls after extraction; it observed **one visited commit and one callback call**. Callers also may drop the callback after extraction, leaving its pointer registered.

Explicitly unregister the callback before returning the walker. The existing one-commit test misses callback invocation while traversing parents.

### 4. A refspec can outlive its owning remote

`src/remote.rs:283`.

`get_refspec` returns `Refspec<'repo>` although the underlying allocation belongs to the `Remote`. Dropping the remote frees its refspecs.

A compile-only harness successfully returned a refspec from a function whose local remote was dropped. Source inspection confirms the resulting handle points into freed storage; no freed handle was dereferenced during testing.

Tie the result to the borrow of `self`, using `Refspec<'_>`, or return owned data.

### 5. Rebase operations can outlive their owning rebase

`src/rebase.rs:212`, `src/rebase.rs:220`.

The iterator yields operations with the repository-related lifetime rather than borrowing the rebase. Their pointers refer to an operation array freed by `git_rebase_free`.

A compile-only harness successfully returned an operation while dropping its local rebase. Subsequent operation getters would access freed storage.

Return owned operation data, or provide an API whose returned operation borrows the rebase.

### 6. Movable callback options cache their own address

`src/apply.rs:127`, `src/apply.rs:140`, `src/stash.rs:132`.

Callback setters store `self` as the C payload. These option types remain freely movable, and their `raw` methods do not refresh that payload. Moving configured options therefore leaves callbacks targeting their former address.

This finding is from source inspection. A helper-return smoke test passed on this build, but does not establish that the cached address remains valid under other ordinary moves.

Refresh payloads immediately before synchronous C calls, or keep callback state in a stable allocation. Add regressions that move configured options.

## Medium

### 7. `skip_reuc` changes conflict handling

`src/merge.rs:98`.

The documented option should suppress the REUC extension. It instead toggles `GIT_MERGE_FAIL_ON_CONFLICT`.

An executed conflicting-tree merge returned an index with conflicts by default, but returned `MergeConflict` when only `skip_reuc(true)` was enabled. Callers cannot independently configure these behaviors.

Use `GIT_MERGE_SKIP_REUC` and test both flag combinations.

### 8. Multiple credential helpers are reduced to one

`src/cred.rs:256`.

The implementation says it discovers all helper directives, but uses single-value configuration getters. Repeated `credential.helper` entries therefore lose earlier helpers.

An executed local fixture configured one helper supplying a username and another supplying a password. Expected credentials were `("alice", "dummy")`; the result was `None`. A single-helper positive control passed.

Enumerate multivars in the established order, including empty-value reset behavior. Existing tests use one helper per key.

### 9. Numeric URL hosts are omitted from helper input

`src/cred.rs:211`.

Only `url::Host::Domain` is retained. Valid IPv4 and IPv6 hosts are discarded, so helpers receive no `host` field.

An executed IPv4 fixture returned `None` where the same helper with a domain host succeeded. Ordinary callers using numeric remote addresses cannot reliably retrieve credentials.

Handle every host variant, preserving the required IPv6 formatting and port.

### 10. Annotated commits created by OID panic when queried for a refname

`src/merge.rs:56`.

An annotated commit created with `find_annotated_commit` legitimately has no reference name. `refname()` returns `Option`, but calls a byte getter that unwraps the absent C value.

An executed `find_annotated_commit(id)?.refname()` test panicked instead of returning `None`. Model absence in the byte getter and propagate it through the string getter.

### 11. Fetch refspecs without destinations panic in their getter

`src/refspec.rs:37`.

A source-only fetch refspec such as `refs/heads/topic` is accepted by `remote_add_fetch`. Its destination is absent.

An executed test loaded that valid refspec and called `dst()`, which panicked rather than returning `None`. Callers inspecting configured fetch specifications encounter this on valid configuration.

Expose an optional destination byte accessor and preserve absence in `dst()`.

### 12. `RebaseOperation::exec` panics for ordinary pick operations

`src/rebase.rs:319`.

The documentation says the command is populated only for `Exec`, and the return type permits absence. Nevertheless, the implementation unwraps the null command pointer for `Pick`.

An executed in-memory rebase test obtained a valid pick operation and observed a panic from `exec()`. Return `None` for an absent command.

### 13. Message prettification panics on accepted byte input

`src/message.rs:32`.

`message_prettify` accepts byte-oriented `IntoCString` inputs and returns `Result<String, Error>`. Non-UTF-8 output is unconditionally unwrapped.

An executed test using the ordinary legacy-encoded bytes `caf\xe9\n` panicked. A Unicode string positive control passed.

Return a conversion error, or provide a byte-preserving variant with an explicit encoding contract.

### 14. Path queries panic during validation

`src/pathspec.rs:162`; analogous source path at `src/index.rs:397`.

`matches_path` unwraps a fallible path conversion without documenting a panic. An executed test matched `lib.rs` successfully, then panicked for the equivalent relative spelling `./lib.rs`.

Callers cannot safely pass paths obtained through ordinary relative-path construction. Normalize supported relative spellings and handle rejected paths through the API’s return value. Apply equivalent handling to `Index::get_path`.

### 15. Time ordering disagrees with equality

`src/time.rs:57`.

Derived equality includes the timezone sign, while ordering compares only seconds and offset minutes.

An executed test read otherwise identical `-0000` and `+0000` timestamps from a valid commit. They compared unequal, but `cmp` returned `Equal`. Ordered collections consequently treat unequal values as the same key.

Make equality and ordering consistently include or exclude the sign, and test negative zero.

### 16. Parent iteration violates its advertised iterator contracts

`src/commit.rs:363`, `src/commit.rs:379`, `src/commit.rs:381`.

The iterator documents stopping when a parent cannot be found and implements `FusedIterator` and `ExactSizeIterator`. On lookup failure, however, it advances only one range position.

An executed merge-commit fixture with an unavailable first parent and available second parent returned `None`, then `Some`. Its positive size-hint lower bound also incorrectly promises available items.

Permanently exhaust the iterator after failure, and remove or redesign the exact-size guarantee when lookups can fail.

### 17. Nonempty written packs never return a hash

`src/packbuilder.rs:190`.

The deprecated but public `hash` method returns a value only when `object_count() == 0`.

An executed test wrote a one-object pack successfully. `name()` supplied its checksum, while `hash()` returned `None`. Existing hash assertions exercise only an empty pack.

Determine availability from completed output rather than requiring an empty pack, and retain a nonempty-pack regression.

## Low

These counts are advisory diagnostics from the supplied [tooling] gate, restricted to `src/`; they are not counts of demonstrated runtime defects.

### 18. Public contract documentation omits failure conditions

examples at `src/util.rs:102`, `src/build.rs:177`, `src/build.rs:595`.

The audit emitted **428 missing `# Errors`, 117 missing `# Panics` and 17 missing `# Safety` section diagnostics**. These omissions make failure behavior and unsafe preconditions difficult for callers to establish.

Document applicable conditions after correcting unexpected panics; documentation should not legitimize the defects above.

### 19. Public types lack `Debug`

examples at `src/repo.rs:109`, `src/signature.rs:20`, `src/string_array.rs:15`.

The compiler emitted **113 missing-`Debug` diagnostics**. This prevents callers from deriving `Debug` for containing types and makes failures harder to inspect.

Provide appropriate manual implementations for opaque handles and derived implementations where suitable.

### 20. Unsafe operations lack local justification

examples at `src/attr.rs:26`, `src/diff.rs:25`, `src/odb.rs:23`.

The audit emitted **807 missing safety-comment diagnostics**, covering 806 distinct diagnostic/location pairs. These missing explanations obscure pointer ownership, nullability and callback-lifetime assumptions during maintenance.

Add focused explanations of the actual preconditions and why each wrapper satisfies them. This count does not imply every reported operation is incorrect.

### 21. Lint suppressions and strict-check failures need maintenance

`src/lib.rs:69`, `src/lib.rs:150`; supporting build failures at `libgit2-sys/build.rs:41` and `:68`.

The advisory pass emitted **109 unexplained-`allow` diagnostics across 16 source locations**, including macro expansions. Strict Clippy also failed on `unnecessary_map_or` and `needless_borrows_for_generic_args` in the supporting build script, before reaching the main crate.

Explain necessary suppressions and resolve the strict-gate failures. These are pre-existing findings on Rust 1.99.1, not failures of the requested normal build.

## Verification

Environment: aarch64 Linux, Rust **1.99.1**, Clippy **0.1.99**. Commands used `RUSTUP_AUTO_INSTALL=0`, offline dependency resolution and two build jobs.

From `/work/crate`:

- `cargo build --locked -p git2 --no-default-features` — **passed**.
- `cargo test --locked -p git2 --no-default-features` — **passed**: 187 unit tests, four integration tests and 12 doctests; one doctest ignored.
- `cargo clippy --locked -p git2 --all-targets --no-default-features -- -D warnings` — **failed** in the supporting build script as described above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p git2 --no-deps --document-private-items --no-default-features` — **passed**.
- `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` — formatting, rustdoc and tests **passed**; strict Clippy **failed**. This script additionally checked all features across the workspace.
- `python3 -I /work/[tooling]/scripts/[tooling] /work/crate` — completed; inspected candidates yielded no additional finding.
- `cargo +nightly-2026-10-01 miri test --locked -p git2 --no-default-features oid::tests::zero_is_zero` — **blocked** by Miri’s unsupported `git_oid_iszero` foreign call. This is not evidence of a crate defect.

From `/work/tmp/review-harness`:

- `cargo fmt --all --check` — **passed**.
- `cargo test --locked --offline --test controls` — **5 passed**.
- `cargo test --locked --offline --test contracts` — **12 failed**, reproducing the contract findings.
- `cargo check --locked --offline --test lifetimes` — **passed compilation**, demonstrating acceptance of the two invalid ownership patterns; those functions were not executed.
- `cargo test --locked --offline --test moved_options` — **1 passed**; the cached-address concern remains a source-inspection finding.

The following command was executed separately for each named boundary test:

`cargo test --locked --offline --test empty_buffers TEST_NAME -- --exact`

Names: `new_buffer_can_be_read_as_empty`, `new_buffer_can_be_mutably_borrowed_as_empty`, `empty_remote_reference_list_can_be_read`, `binary_marker_without_data_returns_empty_slices`, and `buffer_blame_signature_can_be_queried`. **Each test process aborted with SIGABRT.**

**Finding count: 21 — 6 High, 11 Medium, 4 Low.** Coverage was targeted, not exhaustive. Other platforms, older toolchains, a feature-powerset check and a complete Miri run were not completed. No dedicated security assessment or exploit testing was performed. Last clock reading: **20.7558 minutes used of 45.0**.