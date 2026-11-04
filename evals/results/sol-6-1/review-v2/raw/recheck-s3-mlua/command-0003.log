Confirmed **9 findings**. Eight affect runtime behavior; one affects documentation. Production source was unchanged.

1. **`Table::clear` leaves the table rooted on the Lua stack.**  
   [src/table.rs:417](/work/crate/src/table.rs:417) pushes the table without subsequently popping it or creating a `StackGuard`. Clearing should preserve capacity while allowing collection after the last reference disappears. Instead, the table remains reachable, retaining its allocation and consuming a stack slot on each direct call. **Executed evidence:** a weak-value-table regression collected an untouched control table but retained the cleared table after dropping both handles and running two collections. The same regression passed under Luau, whose implementation differs.

2. **Relaxed Serde conversion silently deletes supported null values.**  
   [src/serde/de.rs:756](/work/crate/src/serde/de.rs:756) treats every `LightUserData` as unsupported when `deny_unsupported_types(false)` is selected, including the supported `Value::NULL` sentinel. Disabling rejection of unsupported values should preserve supported values. **Executed evidence:** both deserialization and `SerializableValue` serialization changed `{"present":null,"items":[null,1]}` into `{"items":[1]}`. Ordinary callers lose fields and array positions.

3. **Numeric sets cannot round-trip through the supplied conversions.**  
   [src/conversion.rs:1078](/work/crate/src/conversion.rs:1078) and [src/conversion.rs:1102](/work/crate/src/conversion.rs:1102) interpret any table with positive raw length as a sequence. However, set serialization creates membership tables such as `{[1]=true,[2]=true,[3]=true}`. The reverse conversion consequently reads booleans instead of keys. **Executed evidence:** `lua.convert` failed for both `HashSet<i32>` and `BTreeSet<i32>` containing `{1,2,3}`, reporting a boolean-to-`i32` conversion error. Existing round-trip tests cover string sets.

4. **`send` incorrectly makes immutable borrows exclusive for `Sync` userdata.**  
   [src/userdata/util.rs:33](/work/crate/src/userdata/util.rs:33) relies on undocumented array-cloning specialization to detect `Sync`; [src/userdata/ref.rs:66](/work/crate/src/userdata/ref.rs:66) uses that result to select the lock. On Rust 1.99.1, the detector returns false even for `u64`. Ordinary shared borrows therefore fail unnecessarily. **Executed evidence:** two immutable borrows of a `Sync` userdata on the same thread returned `UserDataBorrowError`. Existing `test_userdata_multithread_access_sync` also failed, as did `test_userdata_wrappers`. An extracted detector probe returned false for `u64` in both debug and optimized builds.

5. **Table-to-slice equality accepts unequal sequence lengths.**  
   [src/table.rs:914](/work/crate/src/table.rs:914) returns true after matching the table’s elements without checking for remaining slice elements. Equality should compare the complete sequences. **Executed evidence:** a table containing `[1,2]` compared equal to `[1,2,3]`, and an empty table compared equal to `[1]`. This produces incorrect application comparisons and ineffective assertions; array comparisons delegate to the same implementation.

6. **Userdata equality contradicts `Value::equals`’ documented metamethod lookup.**  
   [src/userdata.rs:955](/work/crate/src/userdata.rs:955) returns false whenever userdata metatables differ. The public contract says to check the first operand’s `__eq`, then the second’s. **Executed evidence:** two userdata types, with the first defining an always-true `__eq`, compared true using Lua 5.4’s `a == b` but false using Rust’s `Value::equals`. Callers receive inconsistent results for the same operands.

7. **Metatable iteration converts restricted entries before skipping them.**  
   [src/userdata.rs:1039](/work/crate/src/userdata.rs:1039) requests the next already-converted `V` before checking whether its key is restricted. The documented iterator should skip `__gc` and `__metatable`. **Executed evidence:** an empty userdata’s visible entries were all strings when iterated as `Value`, but `pairs::<String>()` failed converting an internal function or boolean. Typed iteration can fail because of entries callers should never receive.

8. **Luau buffer seeking panics on an overflowing offset.**  
   [src/buffer.rs:137](/work/crate/src/buffer.rs:137) adds the relative offset before checking its range. An invalid destination should return an I/O error. **Executed evidence:** seeking to position 1 in a three-byte buffer, then seeking by `SeekFrom::Current(i64::MAX)`, panicked with “attempt to add with overflow” in the debug test profile. Generic code using the advertised `Seek` adapter can unexpectedly panic at this boundary.

9. **Two documentation links prevent warning-free documentation builds.**  
   [src/thread.rs:447](/work/crate/src/thread.rs:447) cannot resolve `Lua::sandbox`; [src/state.rs:880](/work/crate/src/state.rs:880) has an ambiguous `Debug` link. **Executed evidence:** normal documentation generation succeeded with both warnings. With `RUSTDOCFLAGS='-D warnings'`, it exited 101. These leave unresolved API links and break documentation pipelines that deny warnings.

All commands used `RUSTUP_AUTO_INSTALL=0`, Rust/Cargo 1.99.1, and aarch64 Linux. Existing-project checks ran from `/work/crate`.

For the test matrix, the command was:

```sh
cargo test --locked --features "$F" --lib --tests -- $FILTERS
```

| Features (`F`) | Filters | Outcome |
|---|---|---|
| `lua54,vendored,serde,macros` | None—the requested baseline | Table test process killed by SIGKILL; cargo exited 101 |
| Same | `--skip test_table_with_large_capacity` | 227 passed, 1 ignored |
| `lua54,vendored,serde,macros,async,send,userdata-wrappers` | Skip large-capacity test | Existing shared-borrow test failed |
| Same | Also skip shared-borrow test | Existing userdata-wrapper test failed |
| Same | Also skip userdata-wrapper test | 252 passed, 1 ignored |
| Each of `lua51`, `lua52`, `lua53`, plus `vendored,serde,macros,async` | Skip large-capacity test | 248 passed, 1 ignored per configuration |
| `luau,serde,macros,async` | Skip large-capacity test | 265 passed, 1 ignored |
| `luajit,vendored,serde,macros,async` | Skip large-capacity test | Existing `tests/hooks.rs:78` assertion failed: unexpected additional C-call event |
| Same | Also `--skip test_function_calls` | 248 passed, 1 ignored |

The SIGKILL is a resource-limited result, **not evidence of a code defect**. The large-capacity test requests `1 << 26` entries in each table part. The LuaJIT hook assertion is recorded as a pre-existing test failure; its underlying cause was not established.

Other executed checks:

```sh
cargo clippy --locked --features lua54,vendored,serde,macros,async,send,userdata-wrappers --lib --tests
cargo test --locked --features lua54,vendored,serde,macros,async --doc
cargo doc --locked --no-deps --features lua54,vendored,serde,macros,async,send
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --features lua54,vendored,serde,macros,async,send
cargo fmt --all -- --check
cargo +nightly-2026-10-01 fmt --all -- --check
```

Clippy completed with warnings, including five library-source warnings. Doctests passed: 38 passed, 3 ignored. Documentation outcomes are described above. Stable rustfmt passed while warning that three configured options require nightly. Nightly formatting was blocked because its `cargo-fmt` component is absent; nothing was installed.

Regression tests were added only to [the scratch copy](/work/tmp/review/tests/review_regressions.rs). Commands run there included:

```sh
cargo test --locked --features lua54,vendored,serde,macros,async,send --test review_regressions
cargo test --locked --features luau,serde,macros,async --test review_regressions
```

The final Lua 5.4 run failed all 10 targeted assertions. The Luau run passed the clear/collection regression and failed nine others. These deliberately assert the expected behavior and expose the findings above. `diff -qr` confirmed that the scratch regression file was the only difference between the crate and its copy.

**Finding count: 9.** Coverage was targeted, not exhaustive. Ignored compile-fail tests, Miri, MSRV 1.79, module/system-linked builds, Luau JIT/vector4, and other targets were not checked. No dedicated security assessment was performed. Last clock reading: **8.35635 minutes used of 45**.