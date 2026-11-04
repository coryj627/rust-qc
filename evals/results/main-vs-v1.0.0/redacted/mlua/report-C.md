# Audit report: mlua 0.11.4 (`crate/src`)

## Summary

The core of mlua 0.11.4 holds up well: conversions, threads, serde round-trips, userdata borrowing, scopes and registry handling all behaved correctly when probed. The real problems are narrower:
- `Table::clear` leaks one Lua stack slot on every call. After about a million calls the Lua state stops working.
- The Rust allocator aborts the whole process when an allocation fails, instead of returning `Error::MemoryError`.
- Serializing a sparse table with `detect_mixed_tables` can produce output vastly larger than the table.

## Medium

**`Table::clear` leaks one Lua stack slot per call**
`src/table.rs:403-431` (`Table::clear`, non-Luau branch).
The non-Luau branch pushes the table and has no `StackGuard`, so the table stays on the stack every time. In a probe, a loop of `t.set(1, 1)` then `t.clear()` failed with `Error::StackError` after about 1.1 million calls. From then on, every operation that needs stack space on that `Lua` state fails. A long-running host that clears tables per request will eventually lose its Lua state.

**The allocator aborts the process when an allocation fails**
`src/memory.rs:150-153` and `159-162` (`allocator`).
When `alloc::alloc` or `alloc::realloc` returns null, the code calls `handle_alloc_error`, which aborts the process. Lua expects a NULL return so it can raise `LUA_ERRMEM`, which mlua turns into `Error::MemoryError`. Two probes confirmed the abort (exit 134, "memory allocation of … bytes failed"):
- `lua.create_table_with_capacity(i32::MAX as usize, 0)` aborts on an ordinary machine (a 32 GB request).
- Under `ulimit -v 800000`, the one-line script `string.rep("x", 2^30)` aborts the host. That limit is a common sandboxing setup, so an untrusted script can kill the embedding process.

With `set_memory_limit`, both cases return an error correctly. The realloc branch also reports the old layout instead of the new one.

**`detect_mixed_tables` turns a tiny sparse table into an enormous array**
`src/table.rs:799-806` (`encode_as_array`), `761-784` (`find_array_len`). Reached from `SerializableValue` and from the deserializer at `src/serde/de.rs:189`.
Any table with fewer than 10 integer keys is treated as an array no matter how large its largest key is. In a probe, `{[100000000] = 1}` serialized to 499,999,998 bytes of JSON and took 21 s. A key such as `2^60` or `math.huge` saturates `k as usize` to `usize::MAX`, so the work is effectively unbounded. Script-controlled data can therefore exhaust memory and CPU.

**About 180 `unsafe` blocks have no SAFETY comment**
Workspace-wide; the count from the advisory clippy pass includes `mlua-sys`. Examples: `src/string.rs:113`, `src/buffer.rs:68-79`, `src/userdata/cell.rs:50`.
The blocks I read look sound, but none states the invariants it depends on (stack depth, value type at the index, lock held). Those invariants are exactly where the `clear` leak above slipped in.

## Low

**Internal `to_string` helper inverts booleans and ignores its index for numbers**
`src/util/mod.rs:251` and `src/util/mod.rs:257`.
`lua_toboolean(...) != 1` prints `true` as "false", and the number branch calls `lua_tointegerx(state, -1, …)` instead of using `index`. Today every caller passes -1, and the error handler turns error objects into strings first (`error(true)` correctly reports "true"), so users don't see either bug. Any new caller would.

**`get_path` rejects the smallest integer key**
`src/util/path.rs:79-93`.
`t.get_path("[-9223372036854775808]")` fails with "integer overflow", although that key is valid in Lua. The number is built up as positive and only negated at the end.

**Floats are silently truncated when converted to integers**
`src/conversion.rs:832` and `837`.
A Rust callback that takes an `i32` and is called from Lua as `f(1.5)` receives `1`. `"1.5"` also becomes `1`. Lua itself rejects these ("number has no integer representation"), and the docs for `Lua::coerce_integer` describe Lua's rule. No test covers this behaviour.

**`TableSequence::next` reserves one stack slot but pushes two**
`src/table.rs:1231-1236`.
It calls `check_stack(state, 1)` and then pushes both the table and the element.

**`Buffer` read and write panic without a `# Panics` section (Luau only)**
`src/buffer.rs:41-57`.
An out-of-range `offset` panics, and `offset + N` can overflow. Neither is documented.

**Clippy fails on this newer toolchain only**
`src/error.rs:300`.
`useless_borrows_in_formatting` on `&full_traceback[pos..].trim_end()`. This is toolchain drift: the lint is new in Rust 1.99.

**Rustdoc fails with `-D warnings`**
`src/lib.rs:42-44`, `src/thread.rs:447`, `src/state.rs:880`.

- The links in `lib.rs` (to `create_async_function`, `call_async` and `AsyncThread`) only resolve when the `async` feature is on.
- The link to `Lua::sandbox` in `thread.rs` only resolves with the `luau` feature.
- The `Debug` link in `state.rs` is ambiguous between the struct and the derive macro on every feature set.

**Documentation gaps**
Workspace-wide.
185 functions without docs, 116 functions returning `Result` with no `# Errors` section, and 76 undocumented constants. Most of these are in `mlua-sys`.

**Public types without `Debug`**
For example, `src/serde/ser.rs:502` and `551` (`SerializeStruct`, `SerializeStructVariant`); 19 in total.

## Low

**Miri cannot check this crate**
The crate's `unsafe` code calls into the vendored C Lua through FFI, which Miri cannot run. Consider Miri tests for the Rust-only pieces: `userdata/lock.rs`, `types/app_data.rs` and `util/path.rs`.

**One test needs more memory than this container has**
`test_table_with_large_capacity` allocates several GB on purpose and was SIGKILLed under the 2.5 GB container limit. That is the environment, not a bug.
