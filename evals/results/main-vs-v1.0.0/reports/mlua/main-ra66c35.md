I found five defects in `crate/src/`: four Major and one Minor. All five were reproduced through the public API from a throwaway crate in `tmp/probe/`. One more Major item, unsafe blocks with no `SAFETY:` comments, comes from the advisory lint pass and was not run. With the one test that needs too much memory for this container skipped, the crate's own test suite passes. The clippy and rustdoc gate failures come from the newer toolchain here and from checking only one feature set; they are not defects in the code.

# Audit report: mlua 0.11.4 (`crate/src/`)

## Verdict

mlua is a mature binding and the unsafe core I read looked sound: userdata borrow locks, scoped userdata, `UserDataRef`, app data and buffers. But five user-facing defects reproduce through the public API:
- `Table::clear` leaks a stack slot on every call.
- Comparing a table with a slice gives the wrong answer when the table is shorter.
- A boolean error object is reported with its value inverted.
- A nil field in a serialized tuple variant shifts the later fields.
- `get_path` rejects the smallest integer key.

Defect counts: 0 Blocker, 5 Major, 2 Minor. There are also 4 QA hygiene items.

## Major

### `Table::clear` leaks one Lua stack slot per call
Location: `src/table.rs:403-431` (`Table::clear`, non-Luau branch).
Problem: each call leaves the table on the main state's stack. After about a million calls every operation on that `Lua` fails with `StackError`, so the state is unusable. A program that clears a scratch table every frame or every request gets there in hours.
Cause: unlike every other method in the file, this branch has no `StackGuard`. `lua.push_ref(&self.0)` pushes the table, and when the `lua_next` loop ends the table is still on the stack.
Evidence (ran): a loop of `t.set(1, i)?; t.clear()?;` with `--release` fails at about 1,000,000 iterations with `called Result::unwrap() on an Err value: StackError`. The error comes from `t.set`. The same loop with 200,000 iterations passes, which is consistent with Lua's 1,000,000-slot stack limit.
Fix: add `let _sg = StackGuard::new(state);` before `check_stack(state, 4)?` in that branch.

### `Table == [T]` is true when the table is shorter than the slice
Location: `src/table.rs:888-915` (`impl PartialEq<[T]> for Table`).
Problem: `{1} == [1, 2]` evaluates to `true`. Any table that is a prefix of the slice compares equal.
Cause: the loop runs `0..lua_rawlen(table)` and returns `true` once it ends. Nothing checks that the slice has no further elements. The early `return i == other.len()` only applies when a nil is found inside the table's length.
Evidence (ran): `let t: Table = lua.load("return {1}").eval()?; assert!(t != [1, 2]);` fails, and the probe printed `{1} == [1,2]: true`.
Fix: before the loop, `if len != other.len() { return false; }`, or return `len == other.len()` after it.

### A boolean error object is reported with its value inverted
Location: `src/util/mod.rs:251` (`util::to_string`, `LUA_TBOOLEAN` branch). It is reached from `pop_error` (`src/util/error.rs:123`).
Problem: if the raw error value is `true`, the result is `Error::RuntimeError("false")`, and `false` becomes `"true"`. This path runs whenever an error reaches `pop_error` without going through the traceback handler. One example is an error raised in a `__close` handler during `Thread::reset` on Lua 5.4.
Cause: `(ffi::lua_toboolean(state, index) != 1).to_string()` negates the value; it should be `== 1` or `!= 0`.
Evidence (ran): in a coroutine that holds `local x <close> = setmetatable({}, {__close = function() error(true) end})`, yield once and then call `th.reset(f)`. The call returns `RuntimeError("false")`. With `error(false)` it returns `RuntimeError("true")`.
Fix: `(ffi::lua_toboolean(state, index) != 0).to_string()`.

### Serialized tuple variants lose positions when a field is nil
Location: `src/serde/ser.rs:452-457` (`SerializeTupleVariant::serialize_field`).
Problem: `E::T(None, 5)` serialized with `serialize_none_to_null(false)` gives `{T = {5}}` instead of `{T = {nil, 5}}`. The `5` moves to index 1, so it deserializes into the wrong field or fails. The same happens with any field that serializes to nil, such as a unit with `serialize_unit_to_null(false)`.
Cause: the field is appended with `raw_push`, which writes at `rawlen + 1`. After a nil, `rawlen` does not move forward. `SerializeSeq` avoids this by keeping an explicit `next` index and calling `raw_seti`. Plain tuples are serialized correctly; that probe passes.
Evidence (ran): `lua.to_value_with(&E::T(None, 5), SerializeOptions::new().serialize_none_to_null(false))` gives `[1]=Integer(5) [2]=Nil`.
Fix: add a `next: usize` counter to `SerializeTupleVariant` and write with `self.table.raw_seti(self.next + 1, value)`, as `SerializeSeq::serialize_element` does.

### Most unsafe blocks have no SAFETY comment
Location: across `src/`. The advisory lint counted 180 blocks; examples are `src/string.rs:113`, `src/buffer.rs:68-79` and `src/table.rs:152`.
Problem: the stack-count and slot-reservation reasoning these FFI blocks rely on is not written down. The `clear()` leak above is exactly the kind of mistake a written stack argument would have caught. The blocks I read appear sound.
Evidence (lint, not run): the advisory pass reports 180 `undocumented_unsafe_blocks` in `src/`.
Fix: add a `// SAFETY:` comment to each block, stating its stack-slot budget and any type invariants. Start with the table, string and userdata modules.

## Minor

### `get_path` rejects the smallest valid integer key
Location: `src/util/path.rs:79-93`.
Problem: `a[-9223372036854775808]` fails with "integer overflow in path at position 2", even though `i64::MIN` is a valid Lua integer key.
Cause: the parser builds the magnitude as a positive `Integer` and only negates it at the end, so `i64::MIN`'s magnitude overflows.
Evidence (ran): `t.get_path::<Value>("a[-9223372036854775808]")` returns that error even though the table has that key. The exhaustive fuzz of every input up to 6 characters over `a.[]'"\?-01 é` (about 5.8 million paths) found no panics.
Fix: build the value negatively when there is a leading `-` (`checked_mul(10)` then `checked_sub(digit)`), or parse into `i128` and narrow with `try_from`.

### `util::to_string` reads the number from the stack top instead of `index`
Location: `src/util/mod.rs:257`.
Problem: in the `LUA_TNUMBER` branch, `lua_tointegerx(state, -1, ...)` ignores the `index` argument. Every current caller passes `-1`, so this is a latent bug, not a live one.
Fix: pass `index`.

## QA hygiene

### Toolchain drift: 1 clippy error
`cargo clippy -D warnings` fails on `clippy::useless_borrows_in_formatting` at `src/error.rs:300` (`&full_traceback[pos..].trim_end()`). That lint is newer than the project's toolchain (MSRV 1.79; CI uses stable). Fix: drop the `&`.

### Rustdoc: 5 errors, 1 of them real
Four of the errors are unresolved links to async-only or Luau-only items (`src/lib.rs:42-44`, `src/thread.rs:447`). They appear only because the gate built `lua54,vendored,serde,macros` without `async` and `luau`. The real one is the ambiguous `[`Debug`]` link at `src/state.rs:880`, which is both a struct and a derive macro. Write it as ``[`Debug`](crate::Debug)``.

### Documentation and `Debug` gaps
- 116 public functions return `Result` with no `# Errors` section, for example the `Table` methods.
- 16 public types in `src/` have no `Debug`, for example the `#[doc(hidden)]` serializer state types in `src/serde/ser.rs`.
- 19 public items are undocumented.
- In mlua-sys (outside scope), about 280 FFI items are undocumented.

### Error and lint hygiene
- 13 `map_err(|_| ...)` calls drop the original error, for example `src/userdata/cell.rs:49` and `:57`. These are deliberate mappings to borrow errors.
- 3 asserts have no message.
- 1 bare TODO, at `src/util/mod.rs:28`.
- The scan's other hits (glob re-exports, TODOs) are all in mlua-sys, which is outside scope.

## Unverified

- `Table::raw_remove` and `raw_insert` read `raw_len()` before shifting elements. On a table with holes, `raw_len` can return any border, so elements past that border are not shifted. This matches Lua's own `table.remove` behaviour; I did not decide whether it counts as a defect.
- Miri was not run. Almost every path calls into the C Lua VM, and Miri cannot execute foreign functions.

## Verification commands

All were run inside the container, from `crate/` unless noted.

- `cargo fmt --all --check`: skipped. `rustfmt.toml` uses nightly-only options and nightly rustfmt is not installed.
- `cargo clippy --workspace --all-targets --features lua54,vendored,serde,macros -- -D warnings`: fail, 1 error (the toolchain-drift item above).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features lua54,vendored,serde,macros`: fail, 5 errors (four feature-subset artifacts, one real).
- `cargo test --locked --features lua54,vendored,serde,macros --lib --tests`: the `tests/table.rs` binary was killed (SIGKILL). The cause is `test_table_with_large_capacity`, which asks for 2^26 array plus 2^26 hash slots and exceeds this container's 2.5 GB limit. It is an environment limit, not a defect.
- Same command with `-- --skip test_table_with_large_capacity`: pass, 24 test binaries and 0 failures.
- Probe crate, from `tmp/probe/`: `cargo test --offline --release --test t1 --test t2 --test t3 --no-fail-fast -- --nocapture --test-threads=1`.
  - Four probes fail as expected, one per Major defect above: `table_clear_stack`, `table_eq_slice_shorter`, `reset_close_error_bool`, `tuple_variant_none`.
  - `get_path_cases` passes but prints the `i64::MIN` error.
  - All other probes pass, including integer range conversion, serde round-trips, raw insert, remove, push and pop, embedded NULs and the `get_path` fuzz.
- `cargo +nightly-2026-10-01 miri test`: skipped (FFI-heavy, see Unverified).

Nothing in the repository or its output addressed me as an agent. I made no changes to `crate/`.

## Closing summary

I covered the audit gate, the heuristic scan, and a close reading of the table, string, buffer, conversion, serde, value, multi, chunk, function, thread, scope and userdata code (cell, lock, ref and `AnyUserData`), plus app data and the util and error helpers. The findings above came from probes against that code, and the path parser got an exhaustive short-input fuzz. I did not examine:
- `state.rs` and `state/raw.rs` beyond the call sites I traced
- `userdata/registry.rs`
- the Luau-only modules (`luau/require.rs`, `vector.rs`, Luau buffer behaviour), because the build used `lua54`
- the async paths
- Miri

I stopped because the main modules were covered and further reading was finding only code that matched the expected invariants, not new defects. The clock is well short of the limit.

Last `./clock` reading: 4 minutes used of 45.