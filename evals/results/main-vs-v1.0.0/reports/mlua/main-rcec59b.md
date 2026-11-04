I found no Blockers in mlua 0.11.4's `crate/src/`. It has 11 Major defects, 4 Minor defects and 6 QA hygiene items. The worst problems hit realistic code, and I reproduced each with a probe: comparing a table to a longer slice returns `true`, an integer `HashSet` doesn't survive a round-trip through Lua, fractional numbers are truncated silently into integers, and `Table::clear` leaks one Lua stack slot on every call. The test suite passes once one memory-heavy test is skipped. That test is killed by this machine's 2.5 GB memory limit.

## Blocker

None found.

## Major

### Comparing a table to a longer slice returns true
Location: `src/table.rs:888-915` (`impl PartialEq<[T]> for Table`).
Problem: `{1, 2} == [1, 2, 3]` is `true`. A table compares equal to any slice that starts with the table's contents.
Cause: the loop only runs over the table's length (`for i in 0..len`) and then returns `true`. It never checks that the slice has no extra elements.
Evidence (ran): the lua54 probe printed `t == [1,2,3]: true` for `{1, 2}`, and `{1} == [1,2]: true`. The Luau build gives the same result.
Fix: after the loop, return `len == other.len()` instead of `true`.

### Integer HashSet and BTreeSet don't survive a round-trip through Lua
Location: `src/conversion.rs:1065-1111`.
Problem: `HashSet<i32>::from([1, 2])` becomes the Lua table `{[1]=true,[2]=true}`. Converting it back fails with `error converting Lua boolean to i32`. Any set of integers starting at 1 fails the same way, and so does a hand-written set-style table like `{[1]=true,[5]=true}`.
Cause: `FromLua` treats any table with `raw_len() > 0` as a list and reads its values, which are the `true` flags, instead of its keys. The `IntoLua` side always writes a set as keys mapped to `true`.
Evidence (ran): `lua.unpack::<HashSet<i32>>(lua.pack(HashSet::from([1i32, 2]))?)` returned `Err(FromLuaConversionError { from: "boolean", to: "i32", .. })`.
Fix: read the table as a list only when its values are not all `true`, or always read set-style tables by key. Add a round-trip test for integer sets.

### Fractional numbers are truncated silently into integers
Location: `src/conversion.rs:831-846` (`lua_convert_int!`, `FromLua::from_lua`).
Problem: converting `3.5` to `i32` gives `Ok(3)`, and the string `"3.5"` does too. `Lua::coerce_integer` documents that a float must have an exact integer representation. Lua itself rejects these values (`math.tointeger(3.5)` is `nil`).
Cause: `Value::Number(n) => cast(n)` uses `num_traits::cast`, which truncates. The fallback path calls `cast(coerce_number(..))`, which has the same effect. The fast path in `from_stack` uses `lua_tointegerx` and rejects 3.5, but then falls back to `from_lua`, which truncates.
Evidence (ran): `lua.unpack::<i32>(Value::Number(3.5))` returned `Ok(3)`.
Fix: accept a float only when `n.fract() == 0.0` and it is within range. Otherwise return the existing "out of range" or a "number has no integer representation" conversion error.

### Error::context throws away the earlier context
Location: `src/error.rs:445-466`.
Problem: `Error::runtime("root").context("ctx1").context("ctx2")` prints `ctx2\nruntime error: root`, so `ctx1` is lost. The docs say the method "wraps the error value with additional context".
Cause: the `Error::WithContext { cause, .. } => Error::WithContext { context, cause }` arm replaces the existing context instead of wrapping it.
Evidence (ran): the probe printed `"ctx2\nruntime error: root"`.
Fix: always wrap with `cause: Arc::new(self)`. If dropping the old context is intended, document it as "replaces any existing context".

### A None field in a tuple variant shifts the later fields
Location: `src/serde/ser.rs:452-457` (`SerializeTupleVariant::serialize_field`).
Problem: with `serialize_none_to_null(false)`, `E::Tup(None, 5)` serializes to `{Tup = {5}}`, so the `5` moves into slot 1. Deserializing it back fails with `invalid length 1`. A tuple struct with the same fields correctly gives `{[2] = 5}`.
Cause: tuple variants append with `raw_push`, which uses the current border, so nil fields leave no slot. `SerializeSeq` writes with `raw_seti(self.next + 1, ..)` instead.
Evidence (ran): the probe printed the table `Tup = { 5, }` and `Err(DeserializeError("invalid length 1, expected tuple variant E::Tup with 2 elements"))`.
Fix: keep a counter in `SerializeTupleVariant` and use `raw_seti(next + 1, value)`, the same way `SerializeSeq` does.

### detect_mixed_tables turns a two-entry table into a huge array
Location: `src/table.rs:799-806` (`encode_as_array`), used by both serialization and `Deserializer::deserialize_any`.
Problem: with `detect_mixed_tables(true)`, `{[1]=1, [10000000]=2}` is encoded as a 10,000,000-element array. That is 50 MB of JSON, built in 0.24 s, and the deserializer creates the same 10M-element array. A larger key such as `[1e12]` will exhaust memory. A two-entry Lua table is an easy input for a script to supply.
Cause: `if len < 10 || len * 2 >= max_idx { return Some(max_idx) }` treats any table with fewer than 10 entries as dense, however large its maximum index. lua-cjson's similar rule bounds the maximum index instead of the entry count.
Evidence (ran): the probe printed `ser {[1]=1,[10000000]=2} detect_mixed -> 49999995 bytes` and `de ... -> array len Some(10000000)`.
Fix: test the maximum index, for example `max_idx <= 10 || len * 2 >= max_idx`, so that output size is bounded by a constant times the entry count.

### Table::clear leaks a Lua stack slot on every call
Location: `src/table.rs:412-427` (the non-Luau branch of `Table::clear`).
Problem: each call leaves the table on the current Lua stack. Inside a callback, after about 1M calls, `clear()` and every later stack operation fail with `StackError`. Outside callbacks the main state's stack grows without limit.
Cause: unlike the other table methods, this branch has no `StackGuard`. `lua_next` pops the key when traversal ends, but the pushed table is never popped.
Evidence (ran): calling `t.clear()` 1.2M times in a callback failed at iteration 999,992 with "out of Lua stack". A control loop of `raw_set` ran 1.2M times without error.
Fix: add `let _sg = StackGuard::new(state);` before `check_stack` in that branch.

### Boolean error values are reported with the opposite value
Location: `src/util/mod.rs:251` (`to_string`).
Problem: `(ffi::lua_toboolean(state, index) != 1).to_string()` prints `false` for `true` and `true` for `false`. Most errors are converted to text earlier by the traceback handler, but `Thread::reset` on Lua 5.4 goes through `pop_error` and reaches this code. Errors that bypass the handler, such as when the stack is exhausted, also reach it.
Evidence (ran): a coroutine with a `<close>` variable whose `__close` calls `error(true)` was yielded and then reset. `Thread::reset` returned `RuntimeError("false")`, and `error(false)` produced `RuntimeError("true")`.
Fix: `(ffi::lua_toboolean(state, index) != 0).to_string()`.

### Buffer::read_bytes and write_bytes panic on an out-of-range offset without saying so
Location: `src/buffer.rs:41-57` (Luau only).
Problem: `buf.read_bytes::<4>(10)` on an 11-byte buffer panics with "range end index 14 out of range". Neither method has a `# Panics` section. Offsets often come from script data, and `offset + N` can also overflow.
Evidence (ran): the Luau probe confirmed the panic under `catch_unwind`.
Fix: add `try_read_bytes` and `try_write_bytes` that return `Option` or `Result` using `checked_add` and `get(..)`. At minimum, document the panic.

### "Safe" Lua states accept precompiled bytecode by default
Location: `src/state/raw.rs:352-356` (`load_chunk` uses mode `"bt"` when no mode is set) and `src/chunk.rs:747-762`.
Problem: `Lua::new()` is documented as having "some safety guarantees". However, `lua.load(bytes)` automatically detects and runs binary chunks, and the base library's `load` stays able to load bytecode too. Your own docs on `Chunk::set_mode` say crafted bytecode "can crash the interpreter". So safe code that loads untrusted script bytes can reach memory corruption in the Lua VM.
Cause: the `safe` flag (`state/raw.rs:267`) only gates the `debug`, `ffi` and C-module loading. Chunk mode is not restricted.
Evidence (read): `mark_safe` and `load_std_libs` are the only places that consult `safe`. `load_chunk` passes `"bt"` when the mode is `None`.
Fix: in safe states, default to text-only (`"t"`) unless the caller opts in with `set_mode(ChunkMode::Binary)`. Also wrap the base `load` to force mode `"t"`. If keeping today's behaviour is intended, say so on `Lua::new` and `Lua::load`.

### 180 unsafe blocks have no SAFETY comment
Location: across `src/`. Dense examples: `src/state/raw.rs`, `src/table.rs`, `src/userdata.rs`, `src/conversion.rs:124,132,168,176` (lifetime-extending `transmute`s).
Problem: clippy's `undocumented_unsafe_blocks` reports 180 blocks. The code I read looks sound: string borrow lifetimes, userdata locking, the scope destructors and the metatable type cache. But the invariants live only in nearby prose, such as "Uses N stack spaces, does not call checkstack", or nowhere.
Fix: add `// SAFETY:` comments, starting with the `transmute`s in `conversion.rs` and `userdata/ref.rs`, and the `ptr::write` into uninitialized userdata in `scope.rs:183,231`.

## Minor

### Seeking a Buffer cursor to a huge offset reports a negative position
Location: `src/buffer.rs:135`. `SeekFrom::Start(u64::MAX)` is cast `as i64` and wraps around, so the error says "invalid seek to a negative position" (ran). Fix: convert with `i64::try_from` and report "beyond the end".

### Pretty Debug output shows shared subtables as pointers
Location: `src/table.rs:834`. In `{a = x, b = x}`, `b` prints as `table: 0x…` even though nothing is recursive (ran). Cause: `fmt_pretty` adds tables to `visited` and never removes them. Fix: remove the pointer when leaving the table.

### to_string reads numbers from the top of the stack instead of the given index
Location: `src/util/mod.rs:257`. `lua_tointegerx(state, -1, ..)` ignores `index`. It is latent: every current caller passes -1. Fix: use `index`.

### inspect_stack truncates large levels
Location: `src/state.rs:888`. `level as c_int` wraps for levels above `i32::MAX`. Fix: use `c_int::try_from(level)` and return `None` on overflow.

## QA hygiene

### Documentation: 139 gaps
- 116 fallible public functions have no `# Errors` section.
- 22 public items have no docs.
- 1 function that can panic has no `# Panics` section.

### Missing Debug implementations: 16 public types

### Lint findings: 33
- 16 bare `unwrap()` calls. The notable one is `src/table.rs:727`, where `raw_seti` panics for indices above `i64::MAX`.
- 13 `map_err(|_| ..)` calls that discard the source error.
- 3 asserts without a message.
- 1 wildcard import.
- 1 bare TODO, at `src/util/mod.rs:28`.

### New clippy lint fails on this toolchain (toolchain drift)
Clippy 1.99's `useless_borrows_in_formatting` rejects `src/error.rs:300`. Your CI runs clippy without `-D warnings`.

### Rustdoc warnings
- `src/state.rs:880` has an ambiguous ``[`Debug`]`` link: both a struct and a derive macro have that name.
- Four links in `src/lib.rs:42-44` and `src/thread.rs:447` don't resolve. They point at async or Luau items that don't exist without those features, which is an artifact of the feature set I checked with.

### A test assumes the OS overcommits memory
`tests/table.rs:66` (`test_table_with_large_capacity`) allocates two 2^26-slot table parts. Under a 2.5 GB limit it gets SIGKILL and takes down the rest of `tests/table.rs`.

## Unverified

- The Luau `TextRequirer` path normalisation (`src/luau/require.rs:141-167`, `to_parent` at 256-273). It looks right when read, but I didn't probe it with real files.
- `TableSequence::next` (`src/table.rs:1231`) calls `check_stack(state, 1)` but pushes two values. I couldn't build a case where this overflows.

## Verification commands

All were run from `crate/`.

- `cargo fmt --all --check`: **skipped**. `rustfmt.toml` uses nightly-only options and nightly rustfmt is not installed.
- `cargo clippy --workspace --all-targets --features lua54,vendored,serde,macros -- -D warnings`: **fail**. The only error is the new lint above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features lua54,vendored,serde,macros`: **fail**. These are the rustdoc warnings above.
- `cargo test --locked --features lua54,vendored,serde,macros --lib --tests`: **killed** (SIGKILL) in `test_table_with_large_capacity`.
- `cargo test --locked --features lua54,vendored,serde,macros --lib --tests -- --skip test_table_with_large_capacity`: **pass**.
- `cargo test --locked --features luau,vendored,serde,macros --lib --tests`: **killed** in the same test. The test binaries that ran before it passed.
- `cargo +nightly miri test`: **not run**. Every code path calls into the C Lua VM, which Miri cannot execute.
- Probes: `cargo run` in `tmp/probe` (lua54) and `cargo run --release` in the same crate for timing, plus `cargo run` in `tmp/probe_luau` (Luau). Each path depends on `crate/`. All evidence quoted above comes from these runs.

I didn't change `crate/`. Nothing in the repository or in the output addressed me with instructions.

## Closing summary

I covered tables, strings, conversions, multi-value handling, functions, threads, scope, userdata cells, locks, the registry, metatable generation, app data, the error type, the memory allocator, serde in both directions, chunk loading, path lookup and Luau buffers. I probed each feature through the public API on lua54, and on Luau for the buffer code. I did not run a systematic fuzz harness. The only parser of untrusted text is the path parser; I tried it by hand and found nothing. I also skipped the async code, the `send` and `userdata-wrappers` feature builds, LuaJIT and Lua 5.1–5.3, and `mlua-sys`. I stopped at about 15 of 45 minutes because the main modules had been read or probed and further reading was turning up only minor items.

Last clock reading: 13 minutes used of 45.