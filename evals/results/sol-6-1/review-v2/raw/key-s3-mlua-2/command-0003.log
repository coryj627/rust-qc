Found **13 issues**, including a process abort, retained Lua objects, incorrect comparisons and Serde data loss. Runtime findings below were reproduced with local tests; source inspection established their causes.

1. **Invalid table keys abort the process instead of returning an error** — [src/table.rs:258](/work/crate/src/table.rs:258). With default `Lua::new()` options, `table.set(Value::Nil, 1)` and `table.raw_set(f64::NAN, 1)` each terminate the test process with **SIGABRT**. Invalid keys should produce `Error::RuntimeError`, allowing callers to handle bad input. The fast path calls `lua_rawset` without protection. A control test with an 8 MiB memory limit returns the expected error. Source inspection finds the same exposure in `create_table_from` at [src/state/raw.rs:575](/work/crate/src/state/raw.rs:575).

2. **`Table::clear` leaves a table reference on the Lua stack** — [src/table.rs:417](/work/crate/src/table.rs:417). The non-Luau implementation pushes the table without subsequently popping it or installing a stack guard. A stack-observation test measured growth from zero to one value after one call. An ordinary weak-table test also showed that a cleared table remains alive after its Rust handle is dropped and two collections run; the control without `clear` is collected. Repeated calls directly from Rust retain objects and grow the stack.

3. **`Sync` detection fails on both installed toolchains** — [src/userdata/util.rs:33](/work/crate/src/userdata/util.rs:33), used by [src/userdata/ref.rs:66](/work/crate/src/userdata/ref.rs:66). Shared borrows of `Sync` userdata should coexist. Instead, the standard-library specialization trick reports `String` as non-`Sync`, causing exclusive locking and unexpected `UserDataBorrowError`. The existing `test_userdata_multithread_access_sync` fails on Rust 1.99.1 and nightly-2026-10-01. `test_userdata_wrappers` also fails with `send`. A standalone copy of the detection logic reports `String: false` in debug and optimized builds.

4. **Relaxed Serde handling silently deletes supported null values** — [src/serde/de.rs:756](/work/crate/src/serde/de.rs:756). `Value::NULL` is the documented representation of optional values, but `deny_unsupported_types(false)` skips every lightuserdata, including null. Executed regressions converted `[Some(1), None, Some(2)]` into `[Some(1), Some(2)]`, and serialized `{"x":null,"y":1}` as `{"y":1}`. Callers enabling this option to omit functions can lose ordinary data or shift array positions.

5. **Nil fields shift later tuple-variant fields left** — [src/serde/ser.rs:456](/work/crate/src/serde/ser.rs:456). With `serialize_none_to_null(false)`, serializing `E::Tuple(1, None, 2)` should preserve field positions: index 2 nil and index 3 equal to 2. Instead, `raw_push` recomputes the length after the nil field, placing 2 at index 2. The equivalent plain Rust tuple preserves its positions in a passing control test. This changes positional enum payloads.

6. **Table-to-array/slice equality accepts unequal lengths** — [src/table.rs:914](/work/crate/src/table.rs:914). Equality should compare the complete sequence. The implementation returns true after exhausting the table without checking for remaining slice elements. Regressions show an empty table comparing equal to `[1]`, and `{1}` comparing equal to `[1,2]`. Callers can incorrectly accept incomplete results; existing table tests cover equal-length cases.

7. **Numeric equality rounds distinct values into equality** — [src/value.rs:617](/work/crate/src/value.rs:617). Comparing `Value::Integer(9007199254740993)` with `Value::Number(9007199254740992.0)` returns true through both `PartialEq` and `equals`. Lua 5.4 compares the same values as false. Casting the integer to a float loses precision, producing incorrect comparisons for large integer identifiers and counters.

8. **Fixed-size Serde tuples reject trailing values configured to be skipped** — [src/serde/de.rs:312](/work/crate/src/serde/de.rs:312). With `deny_unsupported_types(false)`, `{1,2,function() end}` should deserialize to `(1,2)`, just as unsupported sequence values are skipped elsewhere. Instead, it returns `invalid length 3`. The final length check reads the underlying iterator directly, bypassing the skip logic. Existing coverage exercises variable-length vectors but misses this boundary.

9. **Removing hooks retains their captured resources** — [src/state.rs:627](/work/crate/src/state.rs:627) and [src/thread.rs:291](/work/crate/src/thread.rs:291). Removal disables execution but leaves the callback in the registry’s hook table. Both removal regressions retained a captured `Arc` after two collections: strong count 2 instead of 1. Removed callbacks can therefore retain substantial resources for the lifetime of a live thread; the main-thread entry can persist until VM destruction. Existing tests verify that execution stops, without checking resource release.

10. **Large invalid buffer seeks panic before returning an error** — [src/buffer.rs:136](/work/crate/src/buffer.rs:136). Under `luau` in the debug test profile, seeking a three-byte buffer with `SeekFrom::End(i64::MAX)` panics with integer overflow. Invalid seeks should return `io::Error`, consistent with the implementation’s range checks and existing invalid-seek tests. Unchecked addition prevents callers from handling this boundary through the returned `Result`.

11. **Boolean errors are inverted during thread reset** — [src/util/mod.rs:251](/work/crate/src/util/mod.rs:251). A Lua 5.4 coroutine whose pending `__close` method calls `error(true)` produces `runtime error: false` when reset. The utility converts booleans using `!= 1`, reversing their values. Callers receive an incorrect diagnostic for the documented thread-closing error path.

12. **Path lookup rejects the minimum valid integer key** — [src/util/path.rs:84](/work/crate/src/util/path.rs:84). A table accepts `i64::MIN` as a key, but `get_path("[-9223372036854775808]")` returns an integer-overflow error. The parser accumulates the positive magnitude before negation, so it cannot represent the minimum signed value. The documented integer-key syntax consequently cannot address every valid integer key.

13. **Documentation contains two broken links** — [src/thread.rs:447](/work/crate/src/thread.rs:447) and [src/state.rs:880](/work/crate/src/state.rs:880). `Lua::sandbox` is unresolved in the former scope; `Debug` is ambiguous between the struct and derive macro in the latter. Ordinary documentation generation succeeds with two warnings, leaving these references unresolved. With `RUSTDOCFLAGS="-D warnings"`, generation fails on Rust 1.99.1.

Validation used aarch64 Linux, Rust 1.99.1, `RUSTUP_AUTO_INSTALL=0`, and prefetched dependencies. Subsequent Cargo commands used offline mode.

| Command run from `/work/crate` | Outcome |
|---|---|
| `cargo test --locked --features lua54,vendored,serde,macros --lib --tests` | Exit 101: table test process received SIGKILL while the large-capacity test was pending. This is not counted as a defect. |
| Same command with `-- --skip test_table_with_large_capacity` | 227 passed, 1 ignored. |
| `cargo test --locked --features lua53,vendored,serde,macros --lib --tests -- --skip test_table_with_large_capacity` | 226 passed, 1 ignored. |
| `cargo test --locked --features lua54,vendored,async,serde,macros,anyhow,userdata-wrappers --lib --tests -- --skip test_table_with_large_capacity` | 252 passed, 1 ignored. |
| `cargo test --locked --features luau,vendored,async,serde,macros,userdata-wrappers --lib --tests -- --skip test_table_with_large_capacity` | 266 passed, 1 ignored. |
| `cargo test --locked --features lua54,vendored,async,send,serde,macros,anyhow,userdata-wrappers --lib --tests --no-fail-fast -- --skip test_table_with_large_capacity` | 253 passed, 2 failed, 1 ignored. Failures: `test_userdata_multithread_access_sync` and `test_userdata_wrappers`. |
| `cargo +nightly-2026-10-01 test --locked --features lua54,vendored,send --test send test_userdata_multithread_access_sync -- --exact` | Same borrowing failure. |
| `cargo test --locked --features lua54,vendored,serde,macros --test compile -- --ignored` | Failed: six of eight cases have diagnostic snapshot mismatches. Expected compile failures still fail compilation; snapshots were left unchanged. |
| `cargo test --locked --features lua54,vendored,async,serde,macros,anyhow,userdata-wrappers --doc` | 38 passed, 3 ignored. |
| `cargo clippy --locked --features lua54,vendored,async,send,serde,macros,userdata-wrappers --lib` | Passed with five warnings. |
| `cargo doc --locked --no-deps --features lua54,vendored,async,send,serde,macros,userdata-wrappers` | Passed with two link warnings; the same command with `RUSTDOCFLAGS="-D warnings"` failed. |
| `cargo fmt --all -- --check` | Passed; stable rustfmt warned that three nightly-only settings were ignored. |
| `cargo +nightly-2026-10-01 fmt --all -- --check` | Environment block: nightly rustfmt is not installed. Nothing was installed. |

Six integration-test files were added only under [the scratch copy](/work/tmp/review/tests). Production source files remain unchanged. The final combined Lua 5.4 regression command was:

```sh
cargo test --locked --features lua54,vendored,serde,macros \
  --test review_regressions --test review_hooks \
  --test review_serde_variants --test review_boundaries \
  --test review_lua_semantics --no-fail-fast -- --test-threads=1
```

It produced 13 expected regression failures and two passing controls. Separate `review_errors` runs reproduced both SIGABRT cases and passed the memory-limit control. Running `review_boundaries` with `luau,vendored,serde,macros` reproduced the seek panic. Evidence and logs remain under `/work/tmp`. An exploratory empty-pop concern was excluded after a passing comparison established that it matches Lua’s `table.remove` behavior.

**Finding count: 13.** Coverage emphasized tables, values/conversions, Serde, userdata, threads, state and error handling. MSRV 1.79, Lua 5.1/5.2, LuaJIT, JIT/vector4 configurations, module/non-vendored builds, other platforms, full release-profile tests and Miri were not run. No dedicated security assessment was performed. Last clock reading: **12.48495 minutes used of 45.0**.