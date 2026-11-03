# Audit report: mlua 0.11.4 (crate/src)

Verdict: the default Lua 5.4 test suite passes, except one test that exceeds the container's memory limit. Probing still found memory-unsafety and process aborts reachable from safe Rust, and in several cases from script data alone. The crate's own README lists these behaviours as bugs: UB without `unsafe`, longjmp over Rust frames, and "mlua internal error" panics.

Findings per severity: 13 Blocker, 15 Major, 9 Minor.

All probes ran through `./run` from throwaway crates (`probes/`, `probes_luau/`, `probes_send/`). `crate/` was not changed. "Ran" means I executed it and saw the stated result. Debug builds of vendored Lua have API asserts on, so memory errors there show up as assertion aborts; I also ran release builds where noted. Nothing in the repository addressed me.

## Blocker

### 1. Scoped userdata methods stay callable after the scope ends (use-after-free)
Location: src/userdata/registry.rs:168-173 and 217-222; src/userdata/cell.rs:278-281; src/scope.rs:284-298.
- **Problem:** the `UserDataType::Unique` branch only compares the self pointer and never checks for "destructed". The scope frees the boxed `T`, but the method closures and the stale storage bytes remain.
- **User sees:** a script that saved `m = ud.get` during `Lua::scope` can call `m(ud)` afterwards and run Rust code on freed data.
- **Established:** ran `scope.create_userdata(S(..))`, then `m(ud)` after the scope. Output was "S dropped", then "method running on S: 2712693 cap 187606138813469", then SIGSEGV in debug and release. `create_any_userdata` also segfaults. The `create_userdata_ref` path correctly errors.
- **Fix:** check the destructed metatable in the Unique branch, or overwrite the storage on take.

### 2. `AnyUserData::destroy` during a method call frees the value in use
Location: src/userdata/cell.rs:257-259; src/userdata/util.rs:498-505.
- **Problem:** `is_safe_to_destroy` returns true when `strong_count > 1`, even while a scoped borrow is active.
- **User sees:** hold a `UserDataRef`, call a method, call `ud.destroy()` inside it, then drop the ref. The `&T` now dangles.
- **Established:** ran it. After a spray, `this.0.len()` printed 4702111234474983745, then SIGSEGV in debug. The release build read the freed memory without crashing.
- **Fix:** hold an `XRc` clone during scoped borrows, or refuse to destroy while any scoped borrow is active.

### 3. Re-registering a userdata type causes type confusion
Location: src/state.rs:1446-1448; src/state/raw.rs:1128-1130 and 1178-1181.
- **Problem:** `register_userdata_type` unrefs the old metatable but does not deregister it or reset `last_checked_userdata_mt`. A new metatable at the same address hits the stale fast path.
- **Established:** ran register A, borrow, register A again, drop, GC, then create B. `is::<A>()` was true on a B and `borrow::<A>()` read B's bytes ("CONFUSED ... 4242424242424242"). Using `String` as A gave SIGSEGV. Debug and release both reproduce it.
- **Fix:** call `deregister_userdata_metatable` when replacing a registration, and reset the cache in `register_userdata_metatable`.

### 4. `Table::set_metatable` accepts a table from another Lua state
Location: src/table.rs:507. The same missing check is at src/state/raw.rs:633 and src/thread.rs:317.
- **Problem:** it uses `metatable.0.index` on this state's ref thread with no instance check, unlike `push_ref`.
- **User sees:** an arbitrary value is installed as the metatable.
- **Established:** ran with a table from a second `Lua`. Debug hit the Lua assert "table expected"; release gave SIGSEGV. `create_thread` with another state's function ran the wrong object ("attempt to call a string value").
- **Fix:** assert the same `WeakLua`, as `push_ref` does.

### 5. `Table::clear` leaves the table on the Lua stack (non-Luau)
Location: src/table.rs:414-426 (no `StackGuard`).
- **User sees:** each call leaks one slot, and the leak corrupts enclosing iteration.
- **Established:**
  - `for_each(|_, _| other.clear())` gave the Lua assert in debug and SIGSEGV in release; `for_each_value` did the same.
  - Calling `clear()` 999,996 times at top level ended in `StackError`.
- **Fix:** add `let _sg = StackGuard::new(state)`.

### 6. `Lua::yield_with` wipes the callback's preallocated failure slot
Location: src/state.rs:2170-2172; src/state/util.rs:59-60 and 86-92.
- **Problem:** `StackGuard::with_top(state, 0)` and `gettop` assume no preallocated `WrappedFailure` at index 1. When the failure pool is empty (after any caught Rust error), `release` then rotates and moves the wrong stack value.
- **User sees:** resume args include an internal userdata, and the Lua stack is corrupted.
- **Established:** ran an async generator with `co()`, `pcall(fail)`, `co('x')`. It printed `resumed with ["userdata","string"]`. Debug then hit the Lua assert "not enough elements in the stack"; release raised a bogus "attempt to compare function with number".
- **Fix:** record the frame base on entry and read and clear only above it.

### 7. `UserDataRef<T>` is `Sync` for non-`Sync` `T` (feature `send`)
Location: src/userdata/cell.rs:152-155; src/userdata/ref.rs:34-47.
- **Problem:** `unsafe impl<T: Send> Sync for UserDataCell<T>` makes `UserDataRef<Cell<u64>>` `Sync`, and `Deref` then hands `&Cell` to several threads.
- **Established:** ran two threads incrementing through one ref with no `unsafe`. Expected 40000000, got 20016477.
- **Fix:** implement `Send`/`Sync` for the ref types manually and require `T: Sync` for `Sync`.

### 8. Nil or NaN table keys escape unprotected
Location: src/table.rs:258-261; src/state/raw.rs:572-576; reached from src/serde/ser.rs:492.
- **Problem:** with no memory limit set, `lua_rawset` runs outside `pcall`, but it raises "table index is nil" or "table index is NaN".
- **User sees:** `t.set(Nil, 1)`, `t.raw_set(f64::NAN, 1)` and `create_table_from([(f64::NAN, 1)])` abort the process. Luau reports "Rust cannot catch foreign exceptions". `to_value_with` on a unit-keyed map with `serialize_unit_to_null(false)` also aborts. Inside a callback the error longjmps over Rust frames.
- **Established:** ran all of these (exit 134). The callback case returned `Err` through a longjmp.
- **Fix:** reject nil and NaN keys before the fast path, or always protect.

### 9. `Table::pairs` and `for_each` call `lua_next` unprotected
Location: src/table.rs:627 and 1184. The comment calls the failure an "impossible scenario".
- **User sees:** adding and removing keys during traversal, plus a GC, makes `next` fail. That aborts the process at top level and longjmps over the iterator in a callback.
- **Established:** ran both. Top level gave SIGABRT. In a callback, `pcall` returned "invalid key to 'next'" after jumping over Rust frames.
- **Fix:** protect the call and return `Err`.

### 10. Iterating a table keyed by a caught Rust error aborts (script data only)
Location: src/table.rs:1179-1185; src/state/raw.rs:701-704 and 780.
- **Problem:** the key becomes a cloned `Value::Error`, and `push_value` creates a new userdata, so `next` cannot find it.
- **User sees:** script `local ok, e = pcall(rust_fn); return {[e]=1, x=2}`, then `t.pairs()` from Rust. `{:#?}` uses the same iterator.
- **Established:** ran it. One item came out, then abort.
- **Fix:** keep the key on the ref thread instead of re-pushing a converted value.

### 11. A panic in a userdata `Drop` unwinds through Lua's C frames
Location: src/userdata/util.rs:473-477 and 498-507.
- **Problem:** there is no `catch_unwind` in `__gc`. The unwind passes `luaD_rawrunprotected` and `GCTM`, leaving `L->errorJmp` dangling, `ci` wrong and GC stopped.
- **User sees:** the README promises instances stay usable after a panic. They do not.
- **Established:** ran `lua.gc_collect()` under `catch_unwind` with a panicking `Drop`.
  - A 300k-table loop grew memory from 23 KB to 16 MB.
  - Tracebacks then show "in metamethod '__gc'".
  - The next unprotected error gave SIGSEGV in debug and a bus error in release.
  - Running this inside a callback hit a Lua assert.
  - `drop(lua)` exits with code 101 partway through `lua_close`.
- **Fix:** `catch_unwind` in the destructors and report the panic safely.

### 12. Coroutines in the "normal" state are treated as resumable
Location: src/thread.rs:246-260, 167 and 332-335.
- **Problem:** `status_inner` sees `LUA_OK` with top > 0 and returns `New`. `resume` then wipes the active coroutine's frame via `StackGuard::with_top(thread, 0)` when `lua_resume` refuses, and `reset` does `lua_settop(0)` on it.
- **User sees:** when B runs inside A, `A.status()` says `Resumable`.
- **Established:** ran `A.resume()` and `A.reset(f)` from B. `reset` returned `Ok`. Both panicked with "mlua internal error: N too many stack values popped", and Lua's `print` then failed. Luau `reset` behaves the same. From a coroutine, `main_thread.reset(f)` returned `Ok` and left a stray function on the main stack.
- **Fix:** detect active frames with `lua_getstack(thread, 0)` and report `Running`.

### 13. Deeply nested tables overflow the native stack (script data only)
Location: src/table.rs:828-876 and 1072-1148; src/value.rs:728-745; src/serde/de.rs:171-218 and 345-379.
- **User sees:** a script builds `t={t}` in a loop. The host then aborts on `serde_json::to_string(&v)`, on `format!("{v:#?}")` and on `lua.from_value`. `to_value` on a deep `serde_json::Value` aborts too.
- **Established:** ran all four at depth 1,000,000 (200,000 for `to_value`). Each gave "has overflowed its stack" and SIGABRT.
- **Fix:** add a recursion limit that returns an error.

## Major

### 14. `Lua::warning` aborts when the warning callback returns `Err`
Location: src/state.rs:866-875 and 827-838.
- **Problem:** `lua_warning` is unprotected, and `warn_proc` raises `lua_error`.
- **Also:** `warn_proc` uses `rawlua.state()`, not the running coroutine.
- **Established:** ran `set_warning_function(|..| Err(..)); lua.warning("m", false)`, which gave exit 134. A read of the coroutine path confirms the wrong-state use.

### 15. A script can panic the host by exhausting the auxiliary stack
Location: src/state/extra.rs:283.
- **User sees:** `Vec<Value>` from a table of 1.1 million strings panics with "cannot create a Lua reference". So does `{:#?}` on a 1.1 million-key table. Inside `pcall(f, t)` the panic still reaches the host.
- **Established:** ran all three (exit 101).
- **Fix:** return `Err` instead of panicking.

### 16. `Value::sort_cmp` is not a total order
Location: src/value.rs:525-526; used at src/table.rs:839 and src/serde/de.rs:515.
- **Problem:** the integer-to-float comparison is lossy near 2^63.
- **User sees:** script-chosen keys make `{:#?}` and `sort_keys(true)` panic with "user-provided comparison function does not correctly implement a total order".
- **Established:** ran seeded tables mixing `math.maxinteger - n` and `2^63*k` keys; 6 panics were caught over 20 seeds.

### 17. `detect_mixed_tables` encodes very sparse tables as arrays
Location: src/table.rs:803; `len < 10` short-circuits the sparsity check. The doc says sparse arrays become maps.
- **Established:** ran `{[50000000]=1}`, which produced 249,999,998 bytes of JSON. `{[1e12]=1}` and `{[math.huge]=1}` did not finish within 20 s.

### 18. `Table == [T]` is true when the table is shorter
Location: src/table.rs:902-914.
- **Established:** ran it. `{1,2} == [1,2,3]` and `{} == [1,2,3]` both return true.
- **Fix:** compare lengths at the end.

### 19. `deny_unsupported_types(false)` silently drops `null`
Location: src/serde/de.rs:753-761.
- **Established:** ran it. `{1,null,3}` gives `[1,3]`, `{a=null,b=2}` gives `{"b":2}`, and `from_value` gives `[Some(1),Some(3)]`. With the default option the output is `[1,null,3]`.
- **Fix:** exclude the null light userdata from the skip list.

### 20. `HashSet` and `BTreeSet` do not round-trip
Location: src/conversion.rs:1078 and 1102.
- **Established:** ran it. `HashSet<i32>{1,2,3}` packs to `{[1]=true,..}`, and unpacking fails with "boolean to i32". Any integer set whose packed table has a non-zero length hits this.

### 21. Hooks rotate the Lua function's registers
Location: src/state/util.rs:59-60; src/state/raw.rs:434 and 466.
- **Problem:** with an empty failure pool, `lua_insert(state, 1)` runs inside the hooked Lua frame and shifts every register until release.
- **Established:** ran a line hook calling a closure over local `b=222`, after `pcall(fail)`. It returned 111 on that line and 222 on the others. Writes through open upvalues would hit the wrong slot.

### 22. Dropping a `call_async` future resumes the script
Location: src/thread.rs:530-536.
- **Problem:** a thread at a plain `coroutine.yield()` looks like `Yielded(0)`, so the terminate marker is delivered to user code.
- **Established:** ran it. After drop, the Lua code continued and printed "userdata: 0x...", exposing the internal marker. A long-running continuation would block inside `Drop` (read only, not run).

### 23. Stack space is not reserved before reading results
Location: src/function.rs:112-127; src/table.rs:1231 (reserves 1, needs 2); src/function.rs:270; src/util/mod.rs:300.
- **User sees:** when a call returns many values, top equals `ci->top`, so the next push overruns the checked area.
- **Established:** ran a call returning 31 values ending in a userdata, `Vec<i32>` from `{1,2,3},2,...,30`, and `Function::environment` in that state. All hit the Lua "stack overflow" assert in debug builds. The two release builds I ran returned correct results (the `environment` case was debug only), writing one slot past the checked area. `Thread::resume` already reserves `nresults + 1`.

### 24. Allocation failure aborts instead of raising a Lua error
Location: src/memory.rs:150-153 and 159-162.
- **Established:** ran a script under `ulimit -v 900000`. It printed "memory allocation of 268435482 bytes failed" and aborted. It should be a catchable "not enough memory".

### 25. Lua 5.x `Lua::new()` lets scripts load crafted bytecode
Location: src/state.rs:209-232; src/state/raw.rs:355. Luau `loadstring` is forced to text at src/luau/mod.rs:92.
- **Established:** ran a script that mutates `string.dump` output and runs it via `load`. It crashed with an illegal instruction in debug and a bus error in release, from script alone.
- **Fix:** restrict `load` to text mode in safe states.

### 26. Luau: a script loaded from a file cannot `require` anything
Location: src/luau/require.rs:172-201 and 227-240.
- **Problem:** the chunk name keeps its extension, and `resolve_module` only tries appended extensions (`main.luau.luau`).
- **Established:** ran `lua.load(Path::new("main.luau"))` with `require("./y")` next to it. It failed with "could not reset to requiring context".

### 27. Luau: `TextRequirer` loses `..` in the relative path
Location: src/luau/require.rs:264-269.
- **Problem:** popping from `.` gives `./`, so a module reached through `../x` is named `./x`.
- **Established:** ran chunk `@main` requiring `../x`, where x requires `./y`. The traceback shows `./x:1`, and the nested require fails.

### 28. `is_sync` relies on a std internal that no longer holds
Location: src/userdata/util.rs:33-41.
- **User sees:** on Rust 1.99 it returns false for `Sync` types, so with `send` a second shared borrow fails.
- **Established:** ran `ud.borrow::<u64>()` twice, which gave `UserDataBorrowError`. `tests/send.rs::test_userdata_multithread_access_sync` and `tests/userdata.rs::test_userdata_wrappers` both fail for this feature set, the first panicking with this error and the second returning it.

## Minor

### 29. `gc_gen` and `gc_inc` hit `unreachable!()` inside a finalizer
Location: src/state.rs:1065 and 1084. `lua_gc` returns -1 there.
Established: ran from a `__gc`; it panicked with "internal error: entered unreachable code".

### 30. Boolean error values are inverted
Location: src/util/mod.rs:251.
Established: ran a thread doing `error(true)`, then `reset`; it gave `RuntimeError("false")`. Line 257 also reads index -1 instead of `index` (read only).

### 31. `Value` integer/float equality is lossy
Location: src/value.rs:617-618.
Established: ran it. `Integer(2^53+1) == Number(2^53)` is true; Lua says false.

### 32. `Value::equals` on userdata does not match its docs
Location: src/userdata.rs:954-957.
Established: ran it. `io.stdout` vs `io.stderr` gives `Err(UserDataTypeMismatch)`. With `__eq` only on the second operand and a different metatable, it returns false without checking the second value as documented.

### 33. Pretty `Debug` output is wrong in two cases
Location: src/table.rs:834 and 854-857.
Established: ran it. A shared non-cyclic table prints as `b = table: 0x...`, because `visited` is never cleared. An empty-string key prints as ` = 1`.

### 34. `Table::push` overflows with a script-controlled `__len`
Location: src/table.rs:160.
Established: ran with `__len` returning `math.maxinteger`. It panicked with an arithmetic overflow in debug, inside the protected closure that must not panic.

### 35. Luau `Buffer` offset arithmetic
Location: src/buffer.rs:45, 56 and 135-137.
Established: ran it. `seek(End(i64::MAX))` and `read_bytes(usize::MAX)` panic with overflow in debug, with no `# Panics` docs. `seek(Start(u64::MAX))` reports "negative position".

### 36. Array serialization declares a length, then skips elements
Location: src/table.rs:1093-1101.
With `deny_unsupported_types(false)`, length-prefixed formats would get a wrong count. JSON shows `[1,3]`. Established by reading; I had no length-prefixed format to run.

### 37. Hygiene and toolchain drift
Advisory lint counts on `lua54,vendored,async,serde,macros`:
- 198 `unsafe` blocks without a SAFETY comment.
- About 304 missing-doc warnings.
- 122 missing `# Errors` sections.
- 21 public types without `Debug`.
- 18 `unwrap` warnings.
- 13 `map_err` calls that discard the error.

On Rust 1.99, clippy `-D warnings` fails with 5 lints and rustdoc `-D warnings` fails with 2 errors (`Lua::sandbox` unresolved link, ambiguous `Debug` link). I could not run the project's own toolchain.

## Verification

- `cargo test --locked --features lua54,vendored,serde,macros --lib --tests --no-fail-fast`: all pass except `test_table_with_large_capacity`, killed by SIGKILL. It needs more than the container's 2.5 GB, so I do not count it as a defect.
- Six runs of `cargo test --features <set> --lib --tests --no-fail-fast -- --skip test_table_with_large_capacity --skip test_compilation`:
  - `lua53`, `lua52`, `lua51` and `luau`, each with `vendored,async,serde,macros`: pass.
  - `lua54,vendored,async,send,serde,macros,anyhow,userdata-wrappers`: the 2 failures in finding 28.
  - `luajit,vendored,async,serde,macros`: `hooks::test_function_calls` fails. An extra "C" hook entry appears, matching the test's `force_memory_limit` expected value. That fits LuaJIT rejecting the custom allocator on aarch64 (inferred from reading, not confirmed), so I do not count it.
- `bash scripts/check.sh --audit --features "lua54,vendored,async,serde,macros" crate`: format skipped (needs nightly rustfmt); clippy, rustdoc and tests fail as described above.
- `python3 -I scripts/audit_scan.py crate`: 27 Major candidates and 17 Notes, all glob re-exports in `mlua-sys` or bare TODOs; none reported as findings.
- Path parser harness: `get_path` over all 271,453 strings up to length 5 on a 12-character alphabet, no panics.
- Miri: skipped. The crate links vendored C Lua, which Miri cannot run.

Number of findings: 37 (13 Blocker, 15 Major, 9 Minor).
Last `./clock` reading: 24 minutes used of 45.
