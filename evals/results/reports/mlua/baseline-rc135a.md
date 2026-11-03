# Audit report: mlua 0.11.4 (`crate/src/`)

**Method.** I read all of `crate/src`. Throwaway probe crates live in `tmp/probe`, `tmp/p2`, `tmp/p3` and `tmp/p4`, and everything ran through `./run`. `crate/` was not modified.

**Default features, Lua 5.4.** All tests pass except `tests/table.rs::test_table_with_large_capacity`, which is SIGKILLed. It allocates more than the 2.5 GB container limit, so I don't count it as a defect.

"Ran" below means I executed a probe and saw the quoted result. Probes use the default `lua54,vendored` features unless noted.

## High: memory unsafety or process death reachable from safe Rust or from scripts

**1. Use-after-free via a saved method of a scoped userdata. `userdata/registry.rs:168`, `:217`**
- For `Scope::create_userdata` and `create_any_userdata`, methods dispatch on `lua_touserdata(self) == target_ptr` and never check that the userdata is destructed.
- A script does `saved = ud.name` inside the scope and `saved(ud)` after it. mlua then dereferences the freed `Box` held in the stale storage. The method closures are not invalidated at scope end (`scope.rs:284-298` only swaps the metatable). `ud:name()` correctly errors.
- Ran: the method saw `this.name` with `ptr=0x0 len=11584890137046528`, then aborted on a huge allocation. An ASan nightly build reported `heap-use-after-free READ of size 31`.

**2. Handles from another `Lua` are used without the documented panic. `table.rs:507`, `state/raw.rs:633`, `:642`, `thread.rs:317`**
- These sites use `x.0.index` directly on this state's ref thread and skip the `push_ref` same-state assert. `value.rs:29` promises a panic.
- Ran: `a.set_metatable(Some(table_from_other_lua))` gave a **segfault in release** (it treats a string as a table) and a C assert abort in debug.
- Ran: `lua.create_thread(f_from_lua2)` silently ran the wrong function: `Ok("function from lua1")`.

**3. `UserDataRef<T>` and `UserDataRefMut<T>` are `Sync` for `T: Send + !Sync` under `send`. `userdata/cell.rs:152-155`, `userdata/ref.rs:34`, `:244`**
- `UserDataCell<T>: Sync` only requires `T: Send`, and `Deref` hands out `&T`.
- Ran (`send`): `UserDataRef<Cell<u64>>` compiles as `Sync`. Four threads doing `r.set(r.get()+1)` gave `expected 80000000 got 22579600`, a data race from safe code.

**4. The stale userdata type cache causes type confusion. `state.rs:1446-1448`, `state/raw.rs:1128`, `:1178-1181`**
- `register_userdata_type` unrefs the old metatable but leaves `last_checked_userdata_mt` and the `registered_userdata_mt` entry in place. `register_userdata_metatable` doesn't clear the cache either.
- Once the address is reused, the fast path returns the old `TypeId`.
- Ran, debug and release: `Big.is::<A>() = true`, `Big.is::<Big>() = false`, `Big borrowed as A: A.0 = 0x4242424242424242`. The probe used `exec_raw` only to read the old address and line up allocations; the bug itself needs no unsafe code.

**5. Nil or NaN table keys raise Lua errors outside protection. `table.rs:258-259` (also reached by `Table::set` through `table.rs:66`), `table.rs:395`, `state/raw.rs:575`, `serde/ser.rs:492`**
- When no memory limit is set, `lua_rawset` runs unprotected.
- Ran, all **process abort**: `t.set(Nil, 1)`, `t.raw_set(f64::NAN, 1)`, `t.raw_remove(Nil)`, `create_table_from(vec![(Nil,1)])`, and `to_value_with` of a `None` map key with `serialize_none_to_null(false)`.
- Luau aborts too: `Rust cannot catch foreign exceptions`.
- With a memory limit set, the same call returns `Err`.
- Inside a Rust callback the error longjmps across Rust frames, skipping destructors and `StateGuard`. Ran: after `pcall(rust_set, {}, nil)` in a coroutine, `lua.current_thread()` stayed on the dead coroutine (`status Running`) back on the main thread.

**6. `lua_next` is called unprotected in iteration. `table.rs:1184`, `:627`, plus `state/raw.rs:780`**
- The `table.rs:1184` comment calls "key not found" impossible, but it is reachable two ways.
- Ran: mutating a table during `pairs` or `for_each` (removing the key, inserting new ones) aborted.
- Ran: **script-triggered** abort. `local ok,e = pcall(fail); return {[e]=1,x=2}` stores an error object as a key. `stack_value` clones it to `Value::Error`, and `push_value` then creates a new userdata that isn't in the table. Aborted via `t.pairs()`, `{:#?}`, and `from_value_with(..deny_unsupported_types(false))`.

**7. Unbounded recursion lets script-built data kill the host. `table.rs:1072` (Serialize), `serde/de.rs:287`/`:345`, `table.rs:828` (`fmt_pretty`)**
- Ran: with `local t={} for i=1,200000 do t={t} end`, `serde_json::to_string(&value)`, `lua.from_value::<serde_json::Value>` and `{:#?}` each ended in `stack overflow, aborting` (rc 134).

**8. A panic in a userdata `Drop` during GC corrupts the VM. `userdata/util.rs:498-507`, also `:473-477`**
- There is no `catch_unwind`, so the panic unwinds through C frames (`GCTM`, `luaD_pcall`).
- Ran: after `catch_unwind(gc_collect)`:
  - `collectgarbage('count')` returns nil, because `gcstp` is stuck and the GC stays disabled.
  - Tracebacks permanently show `[C]: in metamethod '__gc'`.
  - The next unprotected error is a **segfault** (rc 139), consistent with `errorJmp` left dangling.

**9. `Lua::warning` is unprotected. `state.rs:874`, `:829`**
- Ran: a warning callback returning `Err` aborted (rc 134). A panicking callback also aborted instead of propagating. Only the path through Lua's `warn()` is protected.

## Medium

**10. Coroutines in the "normal" state are misreported and corrupted. `thread.rs:246-260`, `:167`, `:313-317`**
- `status_inner` treats any `LUA_OK` thread as New or Finished.
- Ran:
  - `Thread::status` returned `Resumable` while Lua said `normal`.
  - `resume` on it returned an error, then `StackGuard::with_top(thread, 0)` wiped the active frame.
  - `reset` returned `Ok(())` and wiped it too.
  - Both produced `mlua internal error: N too many stack values popped`.

**11. Dropping a pending `call_async` future resumes the script. `thread.rs:530-533`**
- Plain `coroutine.yield()` then receives the terminate lightuserdata and keeps running.
- Ran (`async`): after `drop(fut)`, the script had set `ran_after_drop="userdata: 0x…"`. A script with `while true do end` after the yield hung `drop` forever (timeout, rc 124).

**12. `is_sync` always returns false on this toolchain. `userdata/util.rs:33-41`**
- Ran a verbatim copy on Rust 1.99.1: `is_sync::<String>()`, `::<u32>()` and `::<Cell<u32>>()` are all false.
- Under `send`, every shared borrow therefore takes the exclusive lock, so concurrent reads fail with `UserDataBorrowError`.
- Repo tests fail: `tests/send.rs::test_userdata_multithread_access_sync`, and `test_userdata_wrappers` (`Error: UserDataBorrowError`).

**13. `Table == [T]` ignores extra elements on the right. `table.rs:902-914`**
- Ran: `{} == [1,2,3]` and `{1,2} == [1,2,3]` both returned `true`. The loop only covers the table's length and never compares lengths.

**14. `detect_mixed_tables` doesn't treat small sparse tables as sparse. `table.rs:803`**
- The `len < 10` test lets a table with few keys and a huge index through as an array.
- Ran: `{[50000000]=true}` serialized to 250,000,001 bytes of `null`.
- Also, `{}` always encodes as `[]` in this mode, ignoring `encode_empty_tables_as_array=false` (`table.rs:801-805`).

**15. Integer sets don't round-trip. `conversion.rs:1078`, `:1102`**
- `IntoLua` produces `{[k]=true}`, but `FromLua` treats any table with `raw_len > 0` as a sequence.
- Ran: `HashSet<i32>` and `BTreeSet<i32>` of `{1,2,3}` gave `Err("error converting Lua boolean to i32")`, while `{5,7}` works.
- `{1,2,a=true}` as `HashSet<String>` silently loses `a`.

**16. `Table::clear` leaks a stack slot (non-Luau). `table.rs:414-426`**
- There is no `StackGuard`, so the table stays on the stack.
- Ran: failed with `out of Lua stack` at iteration 999,996 of repeated clears on the main state.

**17. Allocation failure aborts instead of raising a Lua memory error. `memory.rs:151-153`, `:160-162`**
- Ran under `ulimit -v 400000`: `pcall(string.rep,'x',1<<30)` printed `memory allocation of 1073741824 bytes failed` and aborted (rc 134). Returning NULL would let Lua raise a catchable error.

**18. Recycled threads don't inherit the global hook. `state/raw.rs:640-651`**
- Only `create_thread` calls `set_thread_hook`.
- Ran (`async`, `thread_pool_size(4)`): the hook fired 0 times on a recycled `call_async` thread and 200 times on a new thread. An instruction-limit hook is bypassed.

**19. Async callback setup trusts the mutable `coroutine` global. `state/raw.rs:1358`, `:1363`**
- Ran (`async`):
  - After `coroutine = nil`, `create_async_function` returned `error converting Lua nil to table`.
  - A script-replaced `coroutine.yield` was called by the internal poller (`hijacked = true`).

**20. Luau `TextRequirer` loses `..` above the working directory. `luau/require.rs:264-269`**
- `to_parent` pops `./` to `./` rather than `..`.
- Ran (Luau, cwd `sub`, chunk `@./main`): `require('../lib/z')` gets chunk name `./lib/z`. A nested `require('./y')` from `../lib/x` fails with `could not reset to requiring context`. The same tree works from the parent directory.

**21. Luau chunks loaded from a file path can't `require`. `chunk.rs:95-96`, `:105-106`, `luau/require.rs:227-239`**
- The chunk name keeps the extension, and `resolve_module` then looks for `main.luau.luau`.
- Ran: `lua.load(PathBuf::from("./main.luau"))` calling `require` fails with `could not reset to requiring context`.

**22. `Thread::reset` on an errored Lua 5.4 thread returns a garbage error. `thread.rs:352-356`, with `util/error.rs:281`**
- The error object was already moved off the thread, so `lua_closethread` copies a stale slot. Docs promise the original error.
- Ran: `Err("runtime error: <function 0xaaaac26327cc>")`, and no function was installed.

**23. `pairs` stops at the first conversion error. `table.rs:1170`, `:1202`**
- The key is `take()`n and not restored on `Err`. `sequence_values` continues past errors.
- Ran: with 50 entries and the first bad, `ok=0 err=1`.

**24. With `catch_rust_panics(false)`, `pcall` isn't yieldable. `util/mod.rs:161`, `:205`**
- `safe_pcall` and `safe_xpcall` use `lua_pcall` with no continuation.
- Ran: a yield inside `pcall` ends the coroutine (`cannot resume dead coroutine`), while the default state returns `1, true, 2`.

## Low

**25. Inverted boolean in error text. `util/mod.rs:251`**
- Ran: an `error(true)` in `__close` during `reset` gives `runtime error: false`, and `error(false)` gives `true`.
- Line 257 also reads index `-1` rather than `index`. That is harmless today because all callers pass -1 (read only).

**26. `gc_inc` and `gc_gen` panic inside a finalizer. `state.rs:1062-1066`, `:1081-1085`, also `:940`**
- `lua_gc` returns -1 there.
- Ran: `gc_inc` in `__gc` hit `unreachable!()` at `state.rs:1065`, and `gc_is_running()` reported `true`.

**27. A Luau tuple struct named `Vector` is hijacked. `serde/ser.rs:272`**
- Ran (Luau): a user's `struct Vector(String,String,String)` gives `error converting Lua string to f32` from `to_value`.

**28. `Value` equality is imprecise across int and float. `value.rs:617-618`**
- Ran: `Integer(2^53+1) == Number(2^53)` is `true`, while Lua says `false`.

**29. Pretty-printing replaces repeated subtables with a pointer. `table.rs:834`**
- `visited` is never popped, so non-cyclic repeats are affected.
- Ran: `{a=s,b=s}` printed `b = table: 0x…`.

**30. `BufferCursor::seek` overflows. `buffer.rs:135-137`**
- Ran (Luau): `SeekFrom::End(i64::MAX)` panicked in debug with `attempt to add with overflow`, and `Start(u64::MAX)` reported "negative position".
- Release behaviour is read only: the addition would wrap.

**31. Sequence length hint is wrong when elements are skipped. `table.rs:1093`, `:1098-1100`**
- `serialize_seq(Some(len))` announces `len`, but skipped elements are not emitted when `deny_unsupported_types(false)` or `deny_recursive_tables(false)` is set. Length-prefixed formats would get a wrong count.
- Read only, not run.

**Findings: 31. Last `./clock` reading: 19 minutes used of 45.**
