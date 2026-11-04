## High

None found.

## Medium

**Comparing a table to a longer slice returns true**
`src/table.rs:888-915` (`impl PartialEq<[T]> for Table`).
`{1, 2} == [1, 2, 3]` is `true`. A table compares equal to any slice that starts with the table's contents.
the loop only runs over the table's length (`for i in 0..len`) and then returns `true`. It never checks that the slice has no extra elements.
the lua54 probe printed `t == [1,2,3]: true` for `{1, 2}`, and `{1} == [1,2]: true`. The Luau build gives the same result.

**Integer HashSet and BTreeSet don't survive a round-trip through Lua**
`src/conversion.rs:1065-1111`.
`HashSet<i32>::from([1, 2])` becomes the Lua table `{[1]=true,[2]=true}`. Converting it back fails with `error converting Lua boolean to i32`. Any set of integers starting at 1 fails the same way, and so does a hand-written set-style table like `{[1]=true,[5]=true}`.
`FromLua` treats any table with `raw_len() > 0` as a list and reads its values, which are the `true` flags, instead of its keys. The `IntoLua` side always writes a set as keys mapped to `true`.
`lua.unpack::<HashSet<i32>>(lua.pack(HashSet::from([1i32, 2]))?)` returned `Err(FromLuaConversionError { from: "boolean", to: "i32", .. })`.

**Fractional numbers are truncated silently into integers**
`src/conversion.rs:831-846` (`lua_convert_int!`, `FromLua::from_lua`).
converting `3.5` to `i32` gives `Ok(3)`, and the string `"3.5"` does too. `Lua::coerce_integer` documents that a float must have an exact integer representation. Lua itself rejects these values (`math.tointeger(3.5)` is `nil`).
`Value::Number(n) => cast(n)` uses `num_traits::cast`, which truncates. The fallback path calls `cast(coerce_number(..))`, which has the same effect. The fast path in `from_stack` uses `lua_tointegerx` and rejects 3.5, but then falls back to `from_lua`, which truncates.
`lua.unpack::<i32>(Value::Number(3.5))` returned `Ok(3)`.

**Error::context throws away the earlier context**
`src/error.rs:445-466`.
`Error::runtime("root").context("ctx1").context("ctx2")` prints `ctx2\nruntime error: root`, so `ctx1` is lost. The docs say the method "wraps the error value with additional context".
the `Error::WithContext { cause, .. } => Error::WithContext { context, cause }` arm replaces the existing context instead of wrapping it.
the probe printed `"ctx2\nruntime error: root"`.

**A None field in a tuple variant shifts the later fields**
`src/serde/ser.rs:452-457` (`SerializeTupleVariant::serialize_field`).
with `serialize_none_to_null(false)`, `E::Tup(None, 5)` serializes to `{Tup = {5}}`, so the `5` moves into slot 1. Deserializing it back fails with `invalid length 1`. A tuple struct with the same fields correctly gives `{[2] = 5}`.
tuple variants append with `raw_push`, which uses the current border, so nil fields leave no slot. `SerializeSeq` writes with `raw_seti(self.next + 1, ..)` instead.
the probe printed the table `Tup = { 5, }` and `Err(DeserializeError("invalid length 1, expected tuple variant E::Tup with 2 elements"))`.

**detect_mixed_tables turns a two-entry table into a huge array**
`src/table.rs:799-806` (`encode_as_array`), used by both serialization and `Deserializer::deserialize_any`.
with `detect_mixed_tables(true)`, `{[1]=1, [10000000]=2}` is encoded as a 10,000,000-element array. That is 50 MB of JSON, built in 0.24 s, and the deserializer creates the same 10M-element array. A larger key such as `[1e12]` will exhaust memory. A two-entry Lua table is an easy input for a script to supply.
`if len < 10 || len * 2 >= max_idx { return Some(max_idx) }` treats any table with fewer than 10 entries as dense, however large its maximum index. lua-cjson's similar rule bounds the maximum index instead of the entry count.
the probe printed `ser {[1]=1,[10000000]=2} detect_mixed -> 49999995 bytes` and `de ... -> array len Some(10000000)`.

**Table::clear leaks a Lua stack slot on every call**
`src/table.rs:412-427` (the non-Luau branch of `Table::clear`).
each call leaves the table on the current Lua stack. Inside a callback, after about 1M calls, `clear()` and every later stack operation fail with `StackError`. Outside callbacks the main state's stack grows without limit.
unlike the other table methods, this branch has no `StackGuard`. `lua_next` pops the key when traversal ends, but the pushed table is never popped.
calling `t.clear()` 1.2M times in a callback failed at iteration 999,992 with "out of Lua stack". A control loop of `raw_set` ran 1.2M times without error.

**Boolean error values are reported with the opposite value**
`src/util/mod.rs:251` (`to_string`).
`(ffi::lua_toboolean(state, index) != 1).to_string()` prints `false` for `true` and `true` for `false`. Most errors are converted to text earlier by the traceback handler, but `Thread::reset` on Lua 5.4 goes through `pop_error` and reaches this code. Errors that bypass the handler, such as when the stack is exhausted, also reach it.
a coroutine with a `<close>` variable whose `__close` calls `error(true)` was yielded and then reset. `Thread::reset` returned `RuntimeError("false")`, and `error(false)` produced `RuntimeError("true")`.

**Buffer::read_bytes and write_bytes panic on an out-of-range offset without saying so**
`src/buffer.rs:41-57` (Luau only).
`buf.read_bytes::<4>(10)` on an 11-byte buffer panics with "range end index 14 out of range". Neither method has a `# Panics` section. Offsets often come from script data, and `offset + N` can also overflow.
the Luau probe confirmed the panic under `catch_unwind`.

**"Safe" Lua states accept precompiled bytecode by default**
`src/state/raw.rs:352-356` (`load_chunk` uses mode `"bt"` when no mode is set) and `src/chunk.rs:747-762`.
`Lua::new()` is documented as having "some safety guarantees". However, `lua.load(bytes)` automatically detects and runs binary chunks, and the base library's `load` stays able to load bytecode too. Your own docs on `Chunk::set_mode` say crafted bytecode "can crash the interpreter". So safe code that loads untrusted script bytes can reach memory corruption in the Lua VM.
the `safe` flag (`state/raw.rs:267`) only gates the `debug`, `ffi` and C-module loading. Chunk mode is not restricted.
`mark_safe` and `load_std_libs` are the only places that consult `safe`. `load_chunk` passes `"bt"` when the mode is `None`.

**180 unsafe blocks have no SAFETY comment**
across `src/`. Dense examples: `src/state/raw.rs`, `src/table.rs`, `src/userdata.rs`, `src/conversion.rs:124,132,168,176` (lifetime-extending `transmute`s).
clippy's `undocumented_unsafe_blocks` reports 180 blocks. The code I read looks sound: string borrow lifetimes, userdata locking, the scope destructors and the metatable type cache. But the invariants live only in nearby prose, such as "Uses N stack spaces, does not call checkstack", or nowhere.

## Low

**Seeking a Buffer cursor to a huge offset reports a negative position**
`src/buffer.rs:135`. `SeekFrom::Start(u64::MAX)` is cast `as i64` and wraps around, so the error says "invalid seek to a negative position" (ran). Fix: convert with `i64::try_from` and report "beyond the end".

**Pretty Debug output shows shared subtables as pointers**
`src/table.rs:834`. In `{a = x, b = x}`, `b` prints as `table: 0x…` even though nothing is recursive (ran). Cause: `fmt_pretty` adds tables to `visited` and never removes them. Fix: remove the pointer when leaving the table.

**to_string reads numbers from the top of the stack instead of the given index**
`src/util/mod.rs:257`. `lua_tointegerx(state, -1, ..)` ignores `index`. It is latent: every current caller passes -1. Fix: use `index`.

**inspect_stack truncates large levels**
`src/state.rs:888`. `level as c_int` wraps for levels above `i32::MAX`. Fix: use `c_int::try_from(level)` and return `None` on overflow.

## Other notes

**Documentation: 139 gaps**
- 116 fallible public functions have no `# Errors` section.
- 22 public items have no docs.
- 1 function that can panic has no `# Panics` section.

**Missing Debug implementations: 16 public types**

**Lint findings: 33**
- 16 bare `unwrap()` calls. The notable one is `src/table.rs:727`, where `raw_seti` panics for indices above `i64::MAX`.
- 13 `map_err(|_| ..)` calls that discard the source error.
- 3 asserts without a message.
- 1 wildcard import.
- 1 bare TODO, at `src/util/mod.rs:28`.

**New clippy lint fails on this toolchain (toolchain drift)**
Clippy 1.99's `useless_borrows_in_formatting` rejects `src/error.rs:300`. Your CI runs clippy without `-D warnings`.

**Rustdoc warnings**
- `src/state.rs:880` has an ambiguous ``[`Debug`]`` link: both a struct and a derive macro have that name.
- Four links in `src/lib.rs:42-44` and `src/thread.rs:447` don't resolve. They point at async or Luau items that don't exist without those features, which is an artifact of the feature set I checked with.

**A test assumes the OS overcommits memory**
`tests/table.rs:66` (`test_table_with_large_capacity`) allocates two 2^26-slot table parts. Under a 2.5 GB limit it gets SIGKILL and takes down the rest of `tests/table.rs`.

## Other notes

- The Luau `TextRequirer` path normalisation (`src/luau/require.rs:141-167`, `to_parent` at 256-273). It looks right when read, but I didn't probe it with real files.
- `TableSequence::next` (`src/table.rs:1231`) calls `check_stack(state, 1)` but pushes two values. I couldn't build a case where this overflows.
