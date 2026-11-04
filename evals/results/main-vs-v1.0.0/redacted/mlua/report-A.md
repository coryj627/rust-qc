# Audit report: mlua 0.11.4 (`crate/src`)

## Summary

The core binding layer works and the existing test suite passes. But probing the public API through a separate throwaway crate showed several ways a Lua script, or ordinary host code, can crash the process or quietly get wrong results. Most of them sit in table iteration, serde conversion and thread status.  All runtime claims below come from probes built against `lua54,vendored,serde` (the Buffer finding used `luau,vendored`), except where marked as from reading the code.

## High

**Changing a table while iterating it aborts the process**
`src/table.rs:1184` (`TablePairs::next`), `src/table.rs:627` (`Table::for_each`).
`lua_next` is called without protection. The comment says it "fails only if the key is not found ... which seems impossible", but it is easy to reach. A Lua callback that sets the current key to nil and then adds about 100 new keys forces a rehash. The next `lua_next` then raises "invalid key to 'next'" outside any protected call, and the process gets SIGABRT. This was reproduced with both `t.pairs()` and `t.for_each(|k, v| cb.call((t.clone(), k)))`. So a host that iterates a table and calls script code for each entry can be killed by that script. The `pairs` docs also lost their warning about this: the `[Lua manual]` link reference at line 600 is defined but never used.

**Deeply nested tables overflow the Rust stack in serde and `Debug`**
`src/serde/de.rs` (`Deserializer::deserialize_*`), `src/table.rs:1072` (`SerializableTable::serialize`), `src/table.rs:828` (`fmt_pretty`).
the script `local t = {} for i = 1, 200000 do t = {t} end return t` crashes the host in three places, each ending in "thread 'main' has overflowed its stack ... aborting":
- `lua.from_value::<serde_json::Value>(v)`
- `serde_json::to_string(&v)`
- `format!("{v:#?}")`

The recursion guard only catches cycles, not depth.

**`detect_mixed_tables` turns a one-key table into an unbounded array**
`src/table.rs:761-806` (`find_array_len`, `encode_as_array`).
if there are fewer than 10 entries, the table is always treated as an array, with length equal to the largest key, whatever that key is. `{[1.0e300] = 'x'}` gives `max_index = usize::MAX` (the `k as usize` cast saturates). Deserializing it into `serde_json::Value` was killed for running out of memory. `{[1]='a', [1e9]='b'}` would likewise build a billion nulls.

**Safe `Lua::new()` loads precompiled bytecode by default**
`src/state/raw.rs:352-356` (`load_chunk`), `src/chunk.rs:747` (`detect_mode`).
when no mode is set, chunks load with mode `"bt"`. So in a state created by safe `Lua::new()`, the safe call `lua.load(untrusted_bytes).exec()` will run Lua 5.x bytecode. Lua does not verify bytecode, and `chunk.rs:549` itself says crafted bytecode "can crash the interpreter". That is a plausible path to undefined behaviour from safe code. It is not demonstrated here, because building a malicious chunk was out of scope for the time available. The warning only appears on `set_mode`, not on `Lua::new` or `Lua::load`.

## Medium

**`Table::clear` leaks one stack slot per call until the state stops working**
`src/table.rs:412-427` (non-Luau path).
the table is pushed and never popped, because there is no `StackGuard`. Calling `t.set(1, 1)` then `t.clear()` in a loop failed with `Error::StackError` at iteration 999,995. From then on, every API call that needs stack space fails, so a long-running host that reuses a table is permanently broken.

**`detect_mixed_tables` ignores `encode_empty_tables_as_array`**
`src/table.rs:800-806`.
with `detect_mixed_tables(true)` and `encode_empty_tables_as_array` left at its default of false, `{}` deserializes to `[]`. The documented default is that empty tables become maps.

**`Value::to_string` does not match Lua's `tostring` for floats**
`src/value.rs:174`, and the same pattern at `src/value.rs:555` (`fmt_pretty`).
Rust float formatting is used. Probe output compared with Lua 5.4's `tostring`:

| Input | `Value::to_string` | Lua `tostring` |
|---|---|---|
| `1.0` | `"1"` | `"1.0"` |
| `1e100` | a 101-digit string | `"1e+100"` |
| `-0.0` | `"-0"` | `"-0.0"` |
| `0/0` | `"NaN"` | `"-nan"` |
| `2^63` | `"9223372036854776000"` | `"9.2233720368548e+18"` |

`{:#?}` also prints `{1.0, 2}` as `1, 2`, so floats look like integers.

**`Thread::status` and `reset` misreport a suspended coroutine as resumable**
`src/thread.rs:246-259` (`status_inner`), `src/thread.rs:330-336` (`reset_inner`).
a coroutine counts as `Running` only if it is the current state. If coroutine A resumes coroutine B, and B calls into Rust, A is mid-call (Lua's "normal" status) but `status()` returns `Resumable`. The main thread in the same situation also returns `Resumable`. `reset()` on either one returns `Ok(())` and runs `lua_settop(0)` on the live frame, which wipes the stack of the C frame that is currently inside `coroutine.resume`. The coroutine then carries on unreset. This was seen in probes `reset_normal` and `reset_main`, which printed "Resumable" and returned `Ok`.

**Array serialization declares a length, then skips elements**
`src/table.rs:1092-1109`. Found by reading the code.
`serializer.serialize_seq(Some(len))` is called, and then elements for which `check_value_for_skip` returns true are left out. That happens for functions or userdata when `deny_unsupported_types(false)` is set, or for recursive tables when `deny_recursive_tables(false)` is set. A length-prefixed format such as bincode writes `len` and then fewer items, which corrupts the output. JSON hides the problem.

**`Buffer` cursor `seek` panics or reports the wrong error on large offsets**
`src/buffer.rs:134-137`.
in a debug build, `SeekFrom::End(i64::MAX)` panics with "attempt to add with overflow". `SeekFrom::Start(u64::MAX)` is cast to `-1` and reported as "invalid seek to a negative position". An `io::Seek` implementation should return an error, not panic.

**180 unsafe blocks without SAFETY comments**
across `crate/src` (clippy `undocumented_unsafe_blocks`), for example `src/string.rs:113`, `src/buffer.rs:68`, `src/table.rs:1229`, `src/conversion.rs:958`.
no unsafe block states why its preconditions hold. Reading the code turned up one example where the reasoning is wrong: `TableSequence::next` (`src/table.rs:1231`) reserves 1 slot with `check_stack(state, 1)` but pushes 2. The Lua API contract requires reserving 2, and only Lua's internal `EXTRA_STACK` keeps this from going out of bounds.

`src/error.rs:300` (clippy), `src/state.rs:880`, `src/lib.rs:42-44`, `src/thread.rs:447` (rustdoc).
- **Clippy:** `useless_borrows_in_formatting`, a lint from a newer toolchain.
- **Rustdoc:** the ambiguous `[`Debug`]` link, plus intra-doc links to `create_async_function`, `call_async`, `AsyncThread` and `Lua::sandbox`. These break whenever the `async` or `luau` features are off.

The project pins no toolchain. The clippy failure is toolchain drift. The feature-gated links are a real rustdoc defect for non-docs.rs feature sets.
- Remove the `&` in `error.rs:300`.
- Write `struct@Debug` in `state.rs:880`.
- Point the feature-gated links at their items through `cfg_attr(feature = "...", doc = "...")`, or link with plain URLs.

## Low

**`Buffer::read_bytes` and `write_bytes` panic without saying so**
`src/buffer.rs:41-57`.
an out-of-range `offset` panics, and `offset + N` can overflow (`read_bytes::<2>(usize::MAX)` panicked with "attempt to add with overflow"). Neither function has a `# Panics` section.

**`Either` conversion throws away both underlying errors**
`src/conversion.rs:1189-1199` (and `from_lua` next to it).
when both `L` and `R` fail, the error has `message: None`, so the caller cannot tell why. There is also an `unwrap` on `CStr::to_str` at line 1196.

**`unwrap` in library code**
18 instances in total. `src/table.rs:727` matters: `raw_seti` is a hidden but public function and panics when `idx > i64::MAX`. Others are `src/state.rs:870`, `src/serde/ser.rs:385,392`, `src/value.rs:743` and `src/userdata/cell.rs:81,84`.

**Missing documentation**
22 public items with no docs. 116 functions returning `Result` have no `# Errors` section (for example `Table::raw_insert`, `Table::clear`, `Thread::reset`).

**Public types without `Debug`**
16 types, for example `serde::ser::SerializeSeq`, `SerializeStruct`, `SerializeStructVariant`, `TablePairs` and `TableSequence`.

**Lint configuration**
`Cargo.toml` (no `[lints]` table). Three `assert!` calls have no message, and there is one `#[allow]` without a reason.

## Low

- **Silent float-to-integer truncation:** a Lua float converts to Rust integer types by truncating (3.7 becomes 3, -0.5 becomes 0 for `u8`, the string "3.7" becomes 3). The test at `tests/tests.rs:588` shows this is intended, but it differs from Lua's own "number has no integer representation" error and is not documented on `FromLua`. It should be documented.
- **`Value` shape depends on features:** `Value` is an exhaustive public enum whose variants depend on features (`Vector`, `Buffer`). Turning on `luau` anywhere in the dependency graph breaks a downstream crate's exhaustive `match`. Consider `#[non_exhaustive]` at the next breaking release.
- **Extra checks:**
  - Miri cannot run this crate's tests, because they call into vendored C Lua through FFI.
  - Recommend `cargo hack check --each-feature` across the Lua version features.
