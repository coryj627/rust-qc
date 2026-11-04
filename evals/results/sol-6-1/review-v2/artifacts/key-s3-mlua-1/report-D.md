The review found **15 findings: 1 High, 8 Medium and 6 Low**. Ten concern observable behavior; the remainder concern documentation, lint failures and maintainability. Runtime findings were reproduced through local public-API regression tests.

Production source files were not edited. The added tests are confined to `/work/tmp/review-crate/tests/review_regressions.rs`.

## High

### 1. Invalid table keys abort the process

`crate/src/table.rs:258`.

Expected: inserting an invalid Lua key returns an error that the caller can handle.

Observed: `table.raw_set(Value::Nil, 1)` and `table.set(f64::NAN, 1)` each terminated an isolated test process with **SIGABRT**. With a memory limit enabled, the nil-key operation correctly returned an error.

The fast path calls `lua_rawset` without protection when memory errors are considered unlikely, although invalid keys can still raise Lua errors. An ordinary failed insertion can therefore terminate the application.

validate keys before this path or protect operations that can raise non-memory errors. Add regressions for both nil and NaN keys.

## Medium

### 2. Permissive Serde conversion discards supported null values

`crate/src/serde/de.rs:752`.

Expected: disabling rejection of unsupported types preserves the supported `Value::NULL` sentinel.

Observed in executed tests:

- `[Some(1), None, Some(3)]` became `[Some(1), Some(3)]`.
- `{"present":true,"absent":null}` became `{"present":true}`.

The skip function treats every `LightUserData` as unsupported, including the null sentinel. This affects both deserialization and `Value::to_serializable()`, changing array positions and removing fields.

exempt null light userdata before the unsupported-type branch. Test nulls alongside unsupported values in arrays and maps.

### 3. Shared userdata borrowing fails with `send`

Locations: `crate/src/userdata/util.rs:33`, `crate/src/userdata/ref.rs:66`.

Expected: multiple immutable borrows of a `Sync` userdata type succeed.

Observed: on Rust 1.99.1/aarch64, borrowing the same stored `String` twice returned `UserDataBorrowError` on the second borrow, even in one thread. The existing `test_userdata_multithread_access_sync` also failed. The extended suite additionally failed `test_userdata_wrappers` with a borrowing error.

The `is_sync` implementation relies on undocumented standard-library specialization behavior. Its result selects an exclusive lock for a type that supports shared access.

replace this dependency with explicit, checked `Sync` capability at registration or borrowing. Preserve tests for both shared `Sync` access and exclusive access to Send-only types.

### 4. Numeric sets cannot round-trip

Locations: `crate/src/conversion.rs:1078`, `crate/src/conversion.rs:1102`.

Expected: supported Rust sets survive conversion to Lua and back, as established by the existing string-set round-trip tests.

Observed: both `HashSet<i32>` and `BTreeSet<i32>` containing `{1,2,3}` failed conversion back with a boolean-to-integer error.

Serialization creates `{[1]=true,[2]=true,[3]=true}`. Deserialization interprets its positive length as a sequence and reads the boolean values instead of its keys.

distinguish key-based sets from sequences explicitly, for example with a representation marker. Add numeric-set regressions.

### 5. Table equality accepts extra slice elements

`crate/src/table.rs:914`.

Expected: equality compares the complete sequence, including its length.

Observed: executed tests showed both `{1,2} == [1,2,3]` and `{}` `== [1]` evaluate to true. Equal-length comparisons and rejection of shorter slices passed.

The implementation returns true after exhausting the table without checking whether the slice also ended. Callers using equality for validation receive incorrect results.

check the matched sequence length before returning true. Cover empty, shorter, equal and longer slices.

### 6. `Table::clear` leaks a stack reference

`crate/src/table.rs:417`.

Expected: clearing a table removes its entries without creating a persistent reference to the table.

Observed: a table stored only in a weak-value table remained reachable after `clear`, dropping its Rust handle and two collections. The same control case without `clear` was collected. The regression passed under Luau, whose implementation differs.

The non-Luau branch pushes the table onto the Lua stack without popping it or using a `StackGuard`. Repeated clearing retains tables and their allocated capacity.

restore the stack on exit. Add a collection-sensitive regression.

### 7. Luau vector deserialization silently truncates tuples

`crate/src/serde/de.rs:300`.

Expected: converting a three-component vector into a two-element tuple rejects the length mismatch, consistent with table deserialization.

Observed: a vector `(1,2,3)` deserialized as `(f32,f32)` successfully returned `(1,2)`. A three-element table correctly rejected that target, and a three-element vector target succeeded.

The vector branch does not check for unconsumed components after the visitor returns.

validate complete consumption before returning success.

### 8. Buffer seeking can panic before validating the offset

`crate/src/buffer.rs:136`.

Expected: an out-of-range seek returns `io::Error`, consistent with the implementation’s other invalid-position checks.

Observed: seeking a four-byte buffer with `SeekFrom::End(i64::MAX)` panicked with “attempt to add with overflow” in the debug test build.

Offset arithmetic happens before bounds checking. Callers handling fallible seeking can receive a panic instead of an error.

use checked arithmetic for relative offsets and return `InvalidInput` on overflow.

### 9. Strict documentation builds fail for valid feature selections

Locations: `crate/src/lib.rs:42`, `crate/src/thread.rs:447`, `crate/src/state.rs:880`.

Expected: documentation builds resolve links for the selected features.

Observed: the requested baseline feature configuration failed strict rustdoc with five errors: three unavailable async links, an unresolved `Lua::sandbox` link and an ambiguous `Debug` link. Enabling the extended async/send features still left the latter two errors.

These failures prevent the strict documentation check from completing and leave unresolved API references.

make optional-feature links feature-aware, qualify the sandbox link and disambiguate the `Debug` struct link.

## Low

### 10. Removing a hook retains its captured resources

Locations: `crate/src/state.rs:624`, `crate/src/thread.rs:288`; storage at `crate/src/state/raw.rs:504`.

Expected: removing a hook makes its callback collectible.

Observed: after installing a callback capturing an `Arc`, removing the main-thread hook and collecting twice, the strong count remained two instead of one.

Removal disables execution but leaves the callback in the registry table. A long-lived main thread therefore retains captured resources until replacement or VM destruction.

remove the corresponding registry entry when removing the hook.

### 11. Path lookup rejects the minimum integer key

`crate/src/util/path.rs:81`.

Expected: bracket notation accepts valid `Integer` keys.

Observed: storing a value at `i64::MIN` succeeded, but lookup through `[-9223372036854775808]` returned an integer-overflow error. The adjacent minimum and maximum values passed.

The parser accumulates a positive signed magnitude before applying the minus sign.

parse the complete signed token or accumulate negative numbers directly.

### 12. Strict Clippy fails on the supplied host/toolchain

Locations: `crate/src/conversion.rs:534`, `crate/src/conversion.rs:667`, `crate/src/error.rs:300`, `crate/src/string.rs:133`, `crate/src/util/mod.rs:279`.

Expected: the strict lint command completes.

Observed: it failed with four unnecessary pointer-cast errors and one redundant formatting borrow. The pointer diagnostics depend on the host’s `c_char` representation.

use portable pointer casts and remove the redundant borrow. These are Low lint issues; behavior on the declared MSRV was not tested.

### 13. Public documentation omits contracts

Examples: `crate/src/traits.rs:251`, `crate/src/debug.rs:232`, `crate/src/table.rs:269`.

The advisory pass reported **22 missing-item-documentation warnings and 116 missing `# Errors` warnings** within `src/`. Public native-function trait members and many fallible APIs omit explanations callers need when implementing traits or handling failures.

document public members and relevant error conditions. This is a source/lint finding.

### 14. Public types lack `Debug`

Examples: `crate/src/chunk.rs:137`, `crate/src/state.rs:61`, `crate/src/table.rs:1155`.

The advisory pass reported **16 missing-`Debug` sites**, including hidden helper types. Visible types such as `Chunk`, `WeakLua` and table iterators consequently cannot participate in ordinary `Debug`-based diagnostics.

provide appropriate implementations without requiring stored callbacks to implement `Debug`. This is a source/lint finding.

### 15. Unsafe invariants lack local explanations

Examples: `crate/src/userdata/ref.rs:72`, `crate/src/string.rs:113`, `crate/src/scope.rs:54`.

The advisory pass reported **180 unsafe blocks missing safety comments**. Important lifetime-extension and callback boundaries lack local statements of their lifetime, locking or stack requirements, making future changes harder to assess.

document the specific invariants at those boundaries. This finding concerns maintainability; the warning count does not establish runtime defects.

## Verification

Commands ran offline with `RUSTUP_AUTO_INSTALL=0` and two Cargo build jobs.

- `cargo test --locked --features lua54,vendored,serde,macros --lib --tests` — **incomplete**: the table test process received SIGKILL. The suite includes `test_table_with_large_capacity`, requesting two capacities of `1 << 26`. The kill alone establishes no source defect.
- The same command with `-- --skip test_table_with_large_capacity` — **passed: 227 tests; one ignored**.
- `cargo test --locked --features VERSION,vendored,serde,macros,async --lib --tests -- --skip test_table_with_large_capacity`, executed separately for `lua53`, `lua52`, `lua51` and `luau` — **passed**: respectively 248, 248, 248 and 265 tests; one ignored per run.
- `cargo test --locked --features lua54,vendored,send --test send -- --test-threads=1` — **failed**: one passed, `test_userdata_multithread_access_sync` failed.
- Extended Lua 5.4 tests using `lua54,vendored,serde,macros,async,send,anyhow,userdata-wrappers` exposed both existing borrowing failures. With the large-capacity test and those two failures skipped, **253 tests passed; one ignored**.
- `cargo test --locked --features lua54,vendored,serde,macros --doc` — **passed: 33; three ignored**.

The [tooling] gate was executed:

```text
bash /work/[tooling]/scripts/[tooling] --audit --features lua54,vendored,serde,macros /work/crate
```

Its Clippy and strict rustdoc steps failed as described above; its tests were killed in the table suite. Formatting was skipped. Explicitly running `cargo +nightly-2026-10-01 fmt --all --check` confirmed that nightly rustfmt is unavailable. No formatting pass is claimed.

The additional strict documentation command also failed:

```text
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --document-private-items --features lua54,vendored,serde,macros,async,send,anyhow,userdata-wrappers
```

Scratch regressions were executed from `/work/tmp/review-crate`:

```text
cargo test --locked --features lua54,vendored,serde,macros,send --test review_regressions -- --skip invalid_nil_key_returns_error --skip invalid_nan_key_returns_error --test-threads=1
cargo test --locked --features luau,vendored,serde,macros,async --test review_regressions -- --skip invalid_nil_key_returns_error --skip invalid_nan_key_returns_error --test-threads=1
```

Results: **nine failed/two passed** for Lua 5.4; **eight failed/four passed** for Luau. These failures reproduce the findings; passing cases provide neighboring controls. Each invalid-key test was also run separately with its exact test filter and terminated with SIGABRT.

`python3 -I /work/[tooling]/scripts/[tooling] /work/crate` completed; heuristic hits without concrete scoped consequences were discarded.

Miri commands:

```text
cargo +nightly-2026-10-01 miri test --locked --features lua54,vendored,serde,macros --lib
cargo +nightly-2026-10-01 miri test --locked --features lua54,vendored,serde,macros --test table test_table -- --exact
```

The two pure Rust unit tests **passed**. The integration test was **blocked** by Miri’s unsupported `lua_newstate` foreign-function call; this establishes no defect.

**Finding count: 15 — 1 High, 8 Medium, 6 Low.** Coverage excludes dedicated security testing, the ignored compile-test suite, MSRV 1.79, cross-platform targets, native modules, LuaJIT and Luau JIT/vector4 configurations. Cargo-hack was unavailable; nothing was installed.

Last `/work/clock` reading: **14.391916666666667 minutes used of 45.0**.