# Audit report: mlua 0.11.4 (`crate/src/`)

**Method.** I read the source and ran throwaway probe crates through `./run`, in `tmp/probe`, `tmp/probe2` and `tmp/probe3`.
- Probe builds were debug builds.
- Feature sets: `lua54,vendored,serde,macros`; plus `async,send`; and separately `luau`.
- `crate/` was not changed.
- Nothing in the repository or its output addressed me.

**Baseline test suite.** The crate's own tests pass, with one exception. `test_table_with_large_capacity` gets SIGKILL because it exceeds the container's 2.5 GB limit, so I don't count it as a defect.

Unless marked "read only", every finding below was reproduced by running code.

## Critical / high: memory unsafety or data races from safe code

**1. Use-after-free calling a saved method on a scoped userdata after the scope ends** (`userdata/registry.rs:168-173`, `:217-222`; destructor at `scope.rs:284-298`)
- `UserDataType::Unique` methods compare `lua_touserdata(self)` with the captured pointer and then borrow the storage. They never check whether it was destructed.
- Calling `ud:peek()` after the scope correctly errors, but a saved method does not. With `peek = ud.peek` saved in Lua, `peek(ud)` after the scope reaches the freed value.
- Ran: `scope.create_userdata` gave a segfault. `scope.create_any_userdata(vec![1u8;32])` gave `len` = 8589934594, then a segfault on `push`.
- This contradicts the guarantee in the `Lua::scope` docs (`state.rs:1633`). Pure Lua can trigger it.

**2. Type confusion from a stale metatable cache** (`state.rs:1446-1448`; `state/raw.rs:1178-1181`)
- `register_userdata_type` unrefs the old metatable but leaves `registered_userdata_mt` and `last_checked_userdata_mt` in place. Once the address is reused, a different type's userdata matches the cached `TypeId`.
- Ran, using only safe API: register `T1`, create and borrow one, re-register, run GC, create `U1`. Then `uud.borrow::<T1>()` returned `Ok` with value `0x7`, which is `U1` data read as `T1`.
- Only one of the eight filler counts I tried hit it, so it depends on the allocator.

**3. `UserDataRef<T>` and `UserDataRefMut<T>` are `Sync` for `T: Send + !Sync` under `send`** (`userdata/cell.rs:152-155`)
- `UserDataRef<Cell<u64>>` compiles as `Sync`, so `&Cell` can be shared across threads.
- Ran: 4 threads doing 5M increments each. Expected 20,000,000, got 5,894,747.
- `UserDataRefMut` has the same auto-trait path, but I only read that side.

**4. Scope releases the Lua lock before invalidating scoped objects under `send`** (`scope.rs:18-21`, `:310`)
- Fields drop in declaration order, so `lua: LuaGuard` drops before `destructors`. That opens a window where another thread can run scoped callbacks.
- Ran: a closure capturing an `Rc` ran on a second thread 3 times within 54 iterations.

## High: process abort or undefined unwinding, reachable from ordinary use or scripts

**5. Nil or NaN table keys hit an unprotected `lua_rawset`** (`table.rs:258-261`; `state/raw.rs:572-576`)
- The fast path assumes only memory errors can occur, and `Table::set` routes here when there is no metatable.
- Ran, each aborts the process:
  - `t.set(Nil, 1)`
  - `t.raw_set(f64::NAN, 1)`
  - `create_table_from([(Nil, 1)])`
  - `to_value_with` on a map with a `None` key and `serialize_none_to_null(false)`
- Under Luau, `raw_set(Nil, 1)` aborts with "Rust cannot catch foreign exceptions".
- Inside a callback, Lua's `pcall(rs_set, {}, nil)` longjmps over Rust frames: the local `Drop` guard never ran.
- With a memory limit set, the same call correctly returns `Err`.

**6. An error object used as a table key aborts `pairs` and `from_value`** (`table.rs:1179-1185`; `state/raw.rs:703`, `:780`)
- The key becomes `Value::Error` (a clone) and is pushed back as a new userdata, so `lua_next` raises "invalid key" with no protection.
- Ran: script `local ok,e = pcall(bad); return {[e]=1,x=2}`. `t.pairs()` printed one pair and aborted. `from_value` also aborted.

**7. Changing a table during `pairs` or `for_each` aborts** (`table.rs:1181-1184`, `:627`)
- `lua_next` is unprotected, and the comment there calls this failure an "impossible scenario".
- Ran: removing the current key and inserting 200 new ones in the loop body aborted the process in both APIs.

**8. Script-triggered abort from a coroutine with a failing `__close`** (`thread.rs:167`, also `:200`, `:564`, `:600`; `util/mod.rs:81-83`)
- `StackGuard::with_top(thread, 0)` calls `lua_settop` on the errored coroutine's frame. That runs pending `__close` handlers unprotected.
- Ran: `local x <close> = setmetatable({}, {__close=function() error('boom') end}); local y = nil + 1`. `thread.resume()` aborted before returning.

**9. A panic in a userdata `Drop` during GC unwinds through Lua's C frames** (`userdata/util.rs:498-507`)
- `destroy_userdata_storage` has no `catch_unwind`.
- Ran, with the panic caught by the host:
  - Memory after 20,000 tables plus `gc_collect` stayed at 1,142,867 bytes, against 22,417 in the control. GC appears to stay off; I attribute that to Lua's internal stop flag from reading its source.
  - When GC is triggered inside a callback, the result is a segfault. A stale Lua error jump is my likely explanation, not something I confirmed.

**10. Handles misbehave in `__gc` callbacks during `lua_close`** (`state/raw.rs:246-249`, `:63-79`)
- The internal `Lua` has had its strong count decremented, so the count is 0 while finalizers run.
- Ran:
  - `lua.clone()` or `lua.app_data_ref()` in a finalizer aborted with "unsafe precondition(s) violated" in `rc.rs`.
  - `lua.globals().set(..)` panicked with "Lua instance is destroyed", and the finalizer silently did not finish.

**11. `yield_with` treats the preallocated failure userdata as a real value** (`state.rs:2170-2172`; `state/util.rs:59-60`, `:89-94`; requires `async`)
- `nvals = lua_gettop` includes the slot `callback_error_ext` inserted, and `with_top(state, 0)` then removes that slot.
- This needs the failure pool to be empty, for example when resuming from inside a Rust callback.
- Ran:
  - `yield_with::<i32>` returned "userdata is not expected type", and the coroutine died.
  - `yield_with::<Value>` received an internal `AnyUserData`, then Lua's assertion "not enough elements in the stack" aborted the process.

**12. No depth limit on serialization, deserialization or pretty Debug** (`table.rs:1072-1147`, `:828-876`; `value.rs:559`, `:728-767`; `serde/de.rs:171-218`, `:287-379`)
- A script builds `t={t}` in a loop and the host overflows its Rust stack, which is fatal.
- Ran: `serde_json::to_string` was fine at depth 1,000 and overflowed at 5,000 (debug build). `from_value` and `{:#?}` overflowed at 500,000; I did not look for their thresholds.

**13. "Normal" coroutines are treated as new or finished** (`thread.rs:246-259`, `:167`, `:332-336`)
- A coroutine that has resumed another one reports `Finished` or `Resumable` depending on its stack, where Lua says `normal`.
- Ran:
  - From an inner coroutine, `main.resume()` and `main.reset(f)` wiped the live frame of a Rust callback on main.
  - That produced the panic "mlua internal error: N too many stack values popped" and a bogus `UserDataTypeMismatch`.
  - `reset` returned `Ok(())` on a thread that was still running.

**14. Locals are lost if GC runs while a coroutine is yielded from a hook** (`thread.rs:167`, `:564`, `:600`; `state/raw.rs:418-421`; Lua 5.4)
- The stack guard truncates the Lua frame to top 0, so GC no longer sees the registers.
- Ran: `VmState::Yield` on `EVERY_LINE` with `gc_collect` between resumes gave "attempt to index a nil value (local 't')". The control without GC returned the right result.
- Luau's `set_interrupt` yield was fine in the same test.

**15. An error from the warning callback escapes protection** (`state.rs:829`)
- `warn_proc` uses `rawlua.state()` rather than the running thread.
- Ran:
  - `lua.warning("x", false)` with a callback returning `Err` aborted the process.
  - Lua's `warn()` inside a coroutine jumped past `coroutine.resume`, so the whole chunk failed instead of `resume` returning `false`.

## Medium: wrong behaviour or denial of service

**16. `Table::clear` leaks a stack slot per call on non-Luau builds** (`table.rs:414-417`, no `StackGuard`)
- Ran: after 999,996 calls on the main state it failed with "out of Lua stack".

**17. `detect_mixed_tables` misclassifies tables** (`table.rs:778`, `:801-806`)
- The `len < 10` rule ignores sparsity. `{[1e12]=1}` is encoded as a huge array and timed out at 15 s (exit 124). `{[5]=1}` gives `[null,null,null,null,1]`.
- `{}` encodes as `[]` even though `encode_empty_tables_as_array` is off.
- `setmetatable({a=5}, array_mt)` encodes as `{"a":5}`, despite the "systematically" wording in the `array_metatable` docs.

**18. Under a memory limit, reference-stack growth panics instead of returning `MemoryError`** (`state/extra.rs:271-284`)
- Ran: with a 512 KB limit, repeated `create_string("x")` panicked with "cannot create a Lua reference, out of auxiliary stack space (used 20474 slots)".

**19. `Table == [T]` returns true when the table is a shorter prefix** (`table.rs:901-914`)
- Ran: `{1,2} == [1,2,3]` is true and `{} == [1]` is true.

**20. `pairs()` stops silently after the first conversion error** (`table.rs:1170`, `:1202`)
- The key is taken and not restored on `Err`.
- Ran: 10 entries gave ok=4, err=1, with the rest lost. `sequence_values` continues past errors.

**21. `Thread::reset` on an errored thread returns garbage instead of the original error** (`thread.rs:229`, `:355-357`)
- The error object was already moved off the thread, and the `reset` docs promise the original error.
- Ran: every variant returned `runtime error: <function 0x...>`.

**22. `util::to_string` inverts booleans** (`util/mod.rs:251`)
- Ran: `__close` doing `error(true)` followed by `reset` gave `runtime error: false`.
- `:257` also reads index `-1` instead of `index`, but current callers pass -1, so that part is latent (read only).

**23. `deny_unsupported_types(false)` drops `null`** (`serde/de.rs:753-761`)
- The NULL lightuserdata is treated as unsupported and skipped, even though it is a supported value.
- Ran: `{1,null,3}` gives `[1,3]`, and `{a=null,b=1}` gives `{"b":1}`.
- Array positions shift, and `serialize_seq(Some(len))` is given a wrong length (read only).

**24. Tuple-variant `None` shifts later fields** (`serde/ser.rs:456`)
- `raw_push(nil)` does not advance the index.
- Ran: `T(None, 7)` with `serialize_none_to_null(false)` gives `t[1]=7, t[2]=nil`.

**25. Luau: a script loaded by file path cannot use relative `require`** (`luau/require.rs:227-240`, `:175-200`)
- `lua.load(Path::new("req/main.luau"))` names the chunk with its extension, and `resolve_module` then fails to find it.
- Ran: relative and absolute paths both gave "could not reset to requiring context". The same code named `@req/main` works.

**26. Luau: any tuple struct named `Vector` with the vector's arity is hijacked** (`serde/ser.rs:272`)
- Ran: `struct Vector(String,String,String)` failed with "error converting Lua string to f32". The identically shaped `Other` serializes to a table.

**27. `catch_rust_panics(false)` makes `pcall` and `xpcall` non-yieldable** (`util/mod.rs:161`, `:205`)
- Ran: `pcall` around `coroutine.yield` inside `coroutine.wrap` gave `("1","true")` with the default and `("false","false")` with the option. I ran only `pcall`; `xpcall` has the same `lua_pcall` call (read only).

## Low

**28. `Value` equality between integers and floats is lossy** (`value.rs:617-618`)
- Ran: `Integer(i64::MAX) == Number(9223372036854775807.0)` is true in Rust and false in Lua.

**29. GC APIs mishandle Lua's -1 return inside a finalizer** (`state.rs:1081-1085`, `:1062-1066`, `:940`)
- Ran: `gc_gen` panicked with "entered unreachable code", and Lua swallowed the panic.
- `gc_inc` and `gc_is_running` (which would report true) are read only.

**30. Closure captures leak when `create_function` fails on memory** (`util/userdata.rs:17-21`; `state/raw.rs:1226`)
- `t` is captured by the protected closure and skipped by the longjmp.
- Ran: with `set_memory_limit(1)`, `Rc::strong_count` stayed at 2 after the failure and GC. The upvalue's `extra` handle is captured the same way (read only).

**31. `BufferCursor::seek` overflows on large offsets** (`buffer.rs:136-137`)
- Ran (Luau): `SeekFrom::End(i64::MAX)` panicked with "attempt to add with overflow". `:137` has the same pattern (read only).

**32. `get_path` rejects the valid key `[-9223372036854775808]`** (`util/path.rs:79-93`)
- Ran: "integer overflow in path".

**33. Pretty Debug prints invalid keys** (`table.rs:854-857`)
- Ran: `{[""]=1, ["end"]=2}` prints ` = 1` and `end = 2`.

**34. Stack checks are one slot short** (`table.rs:1231`; `function.rs:270`)
- `TableSequence::next` checks 1 slot and pushes 2. `Function::environment` asserts 1 and pushes 2.
- Read only. I saw no user-visible effect, and I expect Lua's extra stack reserve to hide it.

**Findings: 34. Last `./clock` reading: 22 minutes used of 45.**
