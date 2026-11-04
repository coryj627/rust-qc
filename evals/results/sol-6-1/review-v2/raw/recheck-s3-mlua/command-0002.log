The review found 14 findings: 1 Blocker, 8 Major and 5 Minor. The Blocker is a pre-existing failure of the `send` configuration on the supplied Rust 1.99.1 toolchain. Nine findings concern observable behavior; all nine have local regression evidence.

Repository source files were left unchanged. Probes and logs are under `/work/tmp/`.

## Blocker

### 1. Shared borrowing of `Sync` userdata fails

Location: `crate/src/userdata/util.rs:33`; lock selection at `crate/src/userdata/ref.rs:66`.

Expected: `Sync` userdata supports overlapping immutable borrows, as explicitly tested in `tests/send.rs:73`.

Observed: borrowing a `String` userdata twice returns `UserDataBorrowError`. The existing multithreaded test also fails, and the expanded configuration additionally fails `test_userdata_wrappers`.

Consequence: callers cannot reliably share immutable userdata references, including within one thread. Source inspection identifies the dependence on standard-library array `Clone` specialization to detect `Sync`.

Evidence: executed the existing minimal `send` tests and a scratch test containing two successive immutable borrows.

Fix: replace the `Clone`-based detection with an explicit, reliable `Sync`-bounded shared-borrow mechanism, preserving exclusive handling for Send-only userdata.

## Major

### 2. `Table::clear()` retains tables on the Lua stack

Location: `crate/src/table.rs:417`.

Expected: clearing a table preserves stack balance; after its handles are dropped, an otherwise unreachable table can be collected.

Observed: the non-Luau branch pushes the table without popping it or installing a `StackGuard`. A probe created ten tables, placed them in a weak-value table, cleared and dropped them, then collected garbage. All ten remained reachable. The control without `clear()` retained zero.

Consequence: repeated cleanup retains tables and their allocated capacity and consumes stack space.

Evidence: executed regression and control tests on Lua 5.4. The equivalent Luau regression passes.

Fix: restore the stack on exit using a `StackGuard` or an explicit pop. Add collection and repeated-call regression tests.

### 3. Permissive Serde conversion silently removes nulls

Location: `crate/src/serde/de.rs:756`.

Expected: `deny_unsupported_types(false)` skips unsupported values while preserving the documented, supported `Value::NULL` representation.

Observed: `check_value_for_skip` skips every light userdata, including NULL. This affects both deserialization and `SerializableValue` serialization:

- `[Some(1), None, Some(3)]` becomes `[Some(1), Some(3)]`.
- A map containing `{"a": None}` becomes empty.
- JSON null fields disappear.

Consequence: successful conversions silently change data and array positions.

Evidence: five failing scratch tests covering maps, arrays and both conversion directions; default-option controls pass. Reproduced on Lua 5.4 and Luau.

Fix: exempt the null sentinel before the unsupported-light-userdata branch.

### 4. Table equality accepts longer slices with matching prefixes

Location: `crate/src/table.rs:901`, particularly the unconditional success at line 914.

Expected: sequence equality checks both values and sequence length.

Observed: an empty table compares equal to `[1]`; a table containing `[1, 2]` compares equal to `[1, 2, 3]`.

Consequence: callers receive incorrect comparison results, including false-positive assertions.

Evidence: executed separate empty and nonempty regressions. Matching-length controls pass. Reproduced on Lua 5.4 and Luau.

Fix: verify that both sequences end together, preserving the existing first-`nil` behavior for tables with holes. Add unequal-length tests in both directions.

### 5. Integer sets cannot round-trip through Lua

Location: `crate/src/conversion.rs:1078` and `crate/src/conversion.rs:1102`.

Expected: the generic set conversions preserve integer sets, just as existing tests establish round-tripping for string sets.

Observed: `IntoLua` produces `{[element] = true}`. If that table has positive raw length—for example, a set containing `1`—`FromLua` treats its boolean values as sequence elements. Both integer `HashSet` and `BTreeSet` probes fail converting a boolean to `i32`.

Consequence: ordinary integer sets are rejected when read back from their own generated Lua representation.

Evidence: executed failures for both set types. String sets and selected integer sets without key `1` pass controls.

Fix: distinguish generated set tables from sequence inputs, for example with a representation marker, and honor that distinction during reverse conversion.

### 6. Tuple variants bypass normal sequence serialization

Location: `crate/src/serde/ser.rs:290` and `crate/src/serde/ser.rs:456`.

Expected: tuple variants honor sequence options and retain field positions.

Observed:

- An empty tuple variant serializes onward as `{"Items": {}}` instead of `{"Items": []}`, despite the default array-metatable option.
- With `serialize_none_to_null(false)`, `Items(None, 7)` stores `7` at index 1. An ordinary tuple stores `nil` at index 1 and `7` at index 2.

Consequence: enum payload shape and positional meaning change depending on which serializer path is used.

Evidence: two executed scratch regressions comparing against Serde JSON output and ordinary tuple serialization.

Fix: reuse the normal sequence serializer for tuple-variant payloads, including its metatable and explicit index tracking.

### 7. Failed Luau vector conversion omits destructors

Location: `crate/src/conversion.rs:960`.

Expected: if an array-element conversion fails, previously initialized elements are dropped.

Observed: vector-to-array conversion writes elements into `MaybeUninit` and uses `?` without a cleanup guard. A custom positive-value conversion succeeds for the first component and rejects the second; the first element’s destructor is never called. The equivalent table conversion calls it once.

Consequence: fallible custom conversions can leak resources or retain guards.

Evidence: executed Luau regression with a destructor counter and a passing table-conversion control.

Fix: use guarded initialization or collect converted elements into an owned collection before constructing the array.

### 8. Buffer seeks panic before rejecting excessive offsets

Location: `crate/src/buffer.rs:136` and `crate/src/buffer.rs:137`.

Expected: invalid seeks return `io::ErrorKind::InvalidInput`, consistent with existing buffer-cursor tests.

Observed: on a one-byte buffer, `SeekFrom::End(i64::MAX)` panics with arithmetic overflow. `SeekFrom::Current(i64::MAX)` also panics after seeking to position 1.

Consequence: callers handling a fallible I/O operation instead encounter a panic in debug builds.

Evidence: two executed Luau regression tests. Release behavior was not tested.

Fix: use checked arithmetic and return `InvalidInput` when the position calculation overflows.

### 9. Strict documentation builds fail

Location: `crate/src/lib.rs:42`, `crate/src/thread.rs:447`, `crate/src/state.rs:880`.

Expected: documentation builds successfully for supported feature configurations.

Observed: the requested configuration produces five rustdoc errors: three references to disabled async APIs, an unresolved `Lua::sandbox` link, and an ambiguous `Debug` link. Enabling async removes the first three but leaves two errors.

Consequence: documentation jobs using warnings as errors fail; ordinary documentation builds contain broken links.

Evidence: executed strict rustdoc builds for both baseline and expanded feature configurations.

Fix: make optional-API links feature-aware, qualify the sandbox link, and disambiguate `struct@Debug`.

## Minor

### 10. `get_path` rejects the minimum integer key

Location: `crate/src/util/path.rs:81`.

Expected: integer bracket notation can address the minimum valid `Integer` key.

Observed: a Lua 5.4 table accepts `i64::MIN`, but looking it up using `[-9223372036854775808]` returns “integer overflow”. Parsing first accumulates its positive magnitude in a signed integer.

Consequence: one valid boundary key is inaccessible through `get_path`.

Evidence: executed regression.

Fix: accumulate negative numbers using checked subtraction or parse the signed substring directly.

### 11. Strict Clippy fails on the supplied host/toolchain

Location: `crate/src/conversion.rs:534`, `crate/src/conversion.rs:667`, `crate/src/string.rs:133`, `crate/src/util/mod.rs:279`, `crate/src/error.rs:300`.

Expected: the strict lint command succeeds for the checked configuration.

Observed: four unnecessary pointer-cast errors on aarch64 and one redundant formatting-borrow error.

Consequence: strict lint jobs fail on this configuration.

Evidence: executed Clippy gate on Rust 1.99.1, aarch64 Linux. Older toolchains were not tested.

Fix: use portable pointer-cast methods and remove the redundant borrow. This is baseline lint debt, not evidence of incorrect runtime behavior.

### 12. Public documentation leaves contracts incomplete

Location examples: `crate/src/traits.rs:14`, `crate/src/debug.rs:222`, `crate/src/state.rs:866`.

Expected: callers can determine conversion, error and panic behavior from public documentation.

Observed: the root-crate advisory pass reports 22 undocumented items, 116 missing `# Errors` sections and one missing `# Panics` section.

Consequence: callers must inspect implementations to understand failure and conversion behavior.

Evidence: executed advisory scan; source inspection confirmed representative public items.

Fix: document the actual contracts, prioritizing conversion traits and methods with caller-visible failure conditions.

### 13. Nine exposed public types lack `Debug`

Location examples: `crate/src/chunk.rs:137`, `crate/src/state.rs:61`, `crate/src/types.rs:73`, `crate/src/table.rs:1155`.

Expected: public wrapper types support routine diagnostic formatting.

Observed: nine exposed types lack `Debug`, including `Chunk`, `WeakLua`, `VmState` and `TablePairs`. Private or hidden helper types were excluded from this count.

Consequence: downstream diagnostic wrappers cannot derive `Debug` when containing these types.

Evidence: advisory diagnostics plus public-export inspection.

Fix: add derived or appropriately opaque manual implementations.

### 14. Unsafe invariants need local justification

Location examples: `crate/src/conversion.rs:124`, `crate/src/userdata/ref.rs:72`.

Expected: lifetime extensions and raw operations explain the ownership and lifetime conditions that justify them.

Observed: the advisory pass reports 180 unsafe-block comment diagnostics. Inspected lifetime-transmutation sites lack local explanations connecting the extended lifetime to the retained owner.

Consequence: future ownership or field-order changes are harder to review correctly.

Evidence: advisory diagnostics and targeted source inspection. This finding does not assert undefined behavior.

Fix: document the relevant ownership, lifetime and stack invariants at the operations.

## Verification

Commands ran offline with `RUSTUP_AUTO_INSTALL=0` and two Cargo build jobs.

From `/work/crate`:

- `cargo test --locked --features lua54,vendored,serde,macros --lib --tests` — stopped when the table test process received SIGKILL. `test_table_with_large_capacity` requests a very large allocation; the kill alone establishes no defect.
- The same command with `-- --skip test_table_with_large_capacity` — **227 passed**, one ignored.
- `cargo test --locked --features <runtime>,vendored,serde,macros --lib --tests -- --skip test_table_with_large_capacity`, separately for `lua51`, `lua52`, `lua53` — **226 passed each**, one ignored each.
- `cargo test --locked --features luau,vendored,async,serde,macros --lib --tests -- --skip test_table_with_large_capacity` — **265 passed**, one ignored.
- `cargo test --locked --features lua54,vendored,send --test send -- --test-threads=1` — **one passed, one failed**.
- `cargo test --locked --features lua54,vendored,async,send,serde,macros,anyhow,userdata-wrappers --lib --tests -- --skip test_table_with_large_capacity` — failed in the existing `send` test. Excluding it exposed the wrapper-test failure. Excluding both known failures and the allocation test yielded **253 passed**, one ignored.
- `cargo test --locked --features lua54,vendored,serde,macros --doc` — **33 passed**, three ignored.
- `bash /work/skill/scripts/check.sh --audit --features lua54,vendored,serde,macros /work/crate` — strict Clippy and rustdoc failed as reported; workspace tests hit the allocation-test kill. Proper formatting verification was **skipped** because nightly rustfmt is unavailable and the project uses nightly-only options.
- Gate commands included `cargo clippy --workspace --all-targets --features lua54,vendored,serde,macros -- -D warnings` and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features lua54,vendored,serde,macros`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --document-private-items --features lua54,vendored,async,send,serde,macros,anyhow,userdata-wrappers` — failed with two documentation errors.
- `python3 -I /work/skill/scripts/audit_scan.py /work/crate` — completed; no additional substantive in-scope finding was confirmed.
- `cargo +nightly-2026-10-01 miri test --locked --features lua54,vendored,serde,macros --lib` — **two passed**.
- The corresponding Miri command with `--test table test_table_clear -- --exact` — blocked by unsupported foreign function `lua_newstate`; this is not defect evidence.

From `/work/tmp/review-probes`:

- `cargo test --test controls -- --test-threads=1` — **four passed**.
- `cargo test --test regression -- --test-threads=1` — **12 expected regression failures**.
- `cargo test --no-default-features --features luau --test regression -- --test-threads=1` — **one passed, 11 expected failures**.
- `cargo test --no-default-features --features luau --test luau_regression -- --test-threads=1` — **three expected failures**.
- `cargo test --features send --test send_regression -- --test-threads=1` — **one expected failure**.

Final count: **14 findings — 1 Blocker, 8 Major, 5 Minor**. Coverage was targeted, not exhaustive. Security testing was excluded. MSRV 1.79, unavailable cross-targets, ignored compile-fail tests, module mode, LuaJIT, Luau JIT/vector4 and release checks were not run; `cargo-hack` was unavailable. Last clock reading: **15.3375 minutes used of 45.0**.