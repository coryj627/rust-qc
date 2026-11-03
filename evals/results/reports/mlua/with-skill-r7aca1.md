# Audit report: mlua 0.11.4 (crate/src/)

## Verdict

The crate is not sound from safe code. I demonstrated use-after-free, type confusion and data races, and I triggered process aborts both from safe Rust calls and from plain Lua scripts. The existing tests miss all of these: most of the suite passes, with two exceptions listed under Verification.

Findings per severity: 11 Blocker, 21 Major, 15 Minor, 0 Note.

Unless a finding says otherwise, I ran it with `--features lua54,vendored,serde,macros,async`, in debug and/or release. The probes are throwaway crates under `<workspace>/` in `probe/`, `p53/`, `pluau/` and `psend/`. `crate/` was not changed. Nothing in the repository or its output addressed me.

## Blocker

### Scoped userdata methods still run after the scope ends (use-after-free)
Location: src/userdata/registry.rs:168-173 and 217-222; src/scope.rs:155-238.
Problem: for `Scope::create_userdata` and `create_any_userdata`, a method only checks that `lua_touserdata(self)` equals the saved pointer. It never checks that the userdata is still live, and the method closures survive the scope in Lua.
- A script that does `m = ud.get` inside the scope and calls `m(ud)` after it reads the dropped value.
- Ran: the call segfaults in both debug and release.
- Ran: after the old block is collected, `m(o)` on a new userdata of another type was accepted at the same address and read 0x2.
Fix: in the Unique path, also verify the metatable pointer or destructed state, or invalidate the metatable's callbacks when the scope ends.

### Stale metatable cache gives type confusion after re-registering a type
Location: src/state.rs:1446-1448; src/state/raw.rs:1128-1130 and 1178-1181.
Problem: `register_userdata_type` unrefs the old metatable but leaves `last_checked_userdata_mt` and the map entry in place. Once the metatable is collected and a new type's metatable lands on the same address, the fast path returns the old `TypeId`.
Ran: `Other<3>`'s `borrow::<Victim>()` succeeded, `String::len` returned 0x4141414141414141, and reading it segfaulted (debug and release).
Fix: remove the entry and reset the cache when unreffing, and reset the cache in `register_userdata_metatable`.

### AnyUserData::destroy frees a value that a running method is using
Location: src/userdata/cell.rs:257-262; src/userdata/util.rs:498-507.
Problem: `is_safe_to_destroy` returns true when `strong_count > 1`, even if a method holds a scoped borrow with no reference of its own.
Ran: `add_method` doing `let r = ud.borrow()?; ud.destroy()?; drop(r);` frees `this`. The method then returned bytes from a later allocation (`"ZZZZ…ve"`).
Fix: refuse while any borrow is held, or make scoped borrows hold an `XRc` clone.

### Handles from another Lua skip the documented panic and corrupt memory
Location: src/table.rs:506-511; src/state/raw.rs:633 and 642; src/thread.rs:317; the promise is at src/value.rs:29-30.
Problem: these paths use `.0.index` directly, without the `push_ref` check, so they read whatever sits at that slot in this Lua's reference stack.
Ran:
- `t_a.set_metatable(Some(t_b))` with a string at that slot segfaults in release and hits a Lua API assertion in debug.
- `a.create_thread(f_b)` silently ran A's function ("function from lua A ran").
Fix: assert `weak() == vref.lua` on these paths, or go through `push_ref`.

### Nil or NaN table key aborts the process (default configuration)
Location: src/table.rs:258-259 (reached from `Table::set` at 65-66); src/state/raw.rs:575; src/serde/ser.rs:492.
Problem: with no memory limit, `lua_rawset` runs unprotected, and a nil or NaN key raises a Lua error.
Ran:
- `t.set(Nil, 1)`, `t.set(f64::NAN, 1)`, `t.raw_set(Nil, 1)` and `create_table_from([(Nil, 1)])` all abort (exit 134).
- So does `to_value_with` on `BTreeMap<Option<i32>, _>` with `serialize_none_to_null(false)`.
- Luau aborts with "Rust cannot catch foreign exceptions".
- Inside a callback run on a coroutine, the error longjmps over Rust frames, skipping `StateGuard`. `lua.current_thread()` then reported the dead coroutine's pointer instead of the main thread.
The README calls longjmp over Rust frames a bug. That coroutine was unreferenced and two full collections had run, so later use of the pointer should be a use-after-free; that last step is inference from reading.
Fix: return an error for nil or NaN keys before the call, or always protect the call.

### Changing a table during iteration aborts the process
Location: src/table.rs:1184 (`TablePairs`) and 627 (`for_each`).
Problem: `lua_next` is unprotected, on the comment that "invalid key" is an "impossible scenario". It is reachable: remove the current key and insert new ones so the table rehashes.
Ran: both `pairs()` and `for_each` abort (exit 134).
Fix: call `lua_next` through `protect_lua!` and return the error.

### An erroring __gc aborts the host on Lua 5.3
Location: src/state/raw.rs:867-878 (`unlikely_memory_error`), for example the unprotected path at 543-545.
Problem: the unprotected fast path assumes allocation cannot raise. On Lua 5.3 an error in `__gc` propagates out of the GC step.
Ran with `lua53,vendored`: a script set `setmetatable({}, {__gc = function() error('boom') end})`, then a `create_table` loop aborted. Lua 5.2 uses the same code path, which I read but did not run.
Fix: always protect allocations on Lua versions where `__gc` errors propagate.

### Lua::yield_with breaks the callback stack when resumed inside a callback (async)
Location: src/state.rs:2170-2172; src/state/util.rs:86-98.
Problem: `yield_with` treats the whole frame as its arguments and calls `settop(0)`. When the failure pool is empty, slot 1 holds the preallocated `WrappedFailure`.
Ran `co.resume` from inside a Rust callback:
- The async function received an internal `AnyUserData` as an extra argument.
- Lua failed with "attempt to compare function with number".
- Debug hit a Lua API assertion. In release the probe segfaulted at the next step, which raises an error through a callback, so that error path is the likely trigger.
Fix: tell `yield_with` how many slots the callback reserved, and never settop below them.

### Yielding from a hook, then a GC, wipes coroutine locals
Location: src/thread.rs:167, 564 and 600.
Problem: the guard calls `settop(0)` on a yielded thread. After a hook yield the current frame is the Lua function itself, so its registers fall above the top and the collector clears them.
Ran: `VmState::Yield` with `gc_collect()` between resumes made correct code fail with "attempt to concatenate a nil value (constant 'hello')". The existing hook test runs no GC.
Fix: pop only `nresults` values rather than setting an absolute top.

### send: a scoped !Send closure can run on another thread
Location: src/scope.rs:18-21.
Problem: the `lua` guard field drops before `destructors`, so the lock is released while scoped callbacks are still callable.
Ran with feature `send`: a worker thread holding a `Function` handle ran an Rc-capturing scoped closure 3762 times out of 200000.
Fix: declare the guard last, or hold it through both destructor phases.

### send: UserDataRef and UserDataRefMut are Sync for !Sync types
Location: src/userdata/ref.rs:34-38 and 244-248; src/userdata/cell.rs:154-155.
Problem: with `send` and without `userdata-wrappers`, `UserDataRef<Cell<u64>>` is Sync through auto traits and derefs to `&Cell`.
Ran: four scoped threads incrementing through a shared `&UserDataRef` got 20023450 instead of 80000000. The `RefMut` version gave 20078354.
Fix: add a marker or explicit bound requiring `T: Sync`.

## Major

### Deeply nested tables overflow the Rust stack
Location: src/table.rs:1072-1147 (serialize) and 828-876 (`{:#?}`); src/serde/de.rs:287-379.
Problem: there is no depth limit. A script that nests a table 1,000,000 levels (`t = {t}`) crashes the host.
Ran: `serde_json::to_string(&v)`, `lua.from_value::<serde_json::Value>` and `format!("{v:#?}")` each abort with a stack overflow.
Fix: add a depth limit that returns an error.

### Allocator aborts on allocation failure instead of returning NULL
Location: src/memory.rs:151-153 and 160-162.
Problem: `handle_alloc_error` kills the process instead of letting Lua raise "not enough memory". Line 161 also reports the old layout rather than the new one.
Ran under `ulimit -v 1000000`: `string.rep('x', 1<<30)` and `create_table_with_capacity(usize::MAX, 0)` both abort.
Fix: return null.

### Scripts in the safe Lua::new state can load bytecode and crash the host
Location: src/state.rs:185 and 209.
Problem: the safe state keeps `load` in binary mode and keeps `string.dump`, although `Chunk::set_mode` warns that crafted bytecode can crash the interpreter.
Ran: a pure Lua script mutating dumped bytes segfaulted in release.
Fix: force text mode on `load` in safe states, or state plainly in the docs that scripts can do this.

### Panic when more than about 1M references are live
Location: src/state/extra.rs:283.
Problem: legitimate data sizes trigger "cannot create a Lua reference, out of auxiliary stack space".
Ran: a 1.1M-element table collected to `Vec<Value>`, and 600k string keys with `sort_keys(true)`, both panic.
Fix: make reference creation fallible.

### A panic in a userdata Drop during GC permanently breaks the collector
Location: src/userdata/util.rs:473-477 and 498-507.
Problem: there is no `catch_unwind`, so the panic unwinds through Lua's C frames and skips its cleanup. The README promises instances stay usable after panics.
Ran after catching the panic: memory grew to 112 MB and `gc_collect` no longer reclaimed it (112024535 bytes before and after). Later tracebacks show a stale `[C]: in metamethod '__gc'` frame.
Fix: catch the panic inside the destructor.

### Lua::warning aborts when the warning callback fails
Location: src/state.rs:829 and 874.
Problem: `warn_proc` calls `lua_error`, but `lua_warning` is called with no protection.
Ran: a callback returning `Err` with `lua.warning("hello", false)` aborts. The same callback through Lua's `warn()` returns a normal error.
Fix: protect the call.

### Function::call has no stack check before converting results
Location: src/function.rs:126-127; src/traits.rs:126-132; src/state/raw.rs:762. Related: src/table.rs:1231, which checks 1 slot and pushes 2 (read only).
Problem: `lua_xpush` needs a free slot on the source state, and a `MULTRET` pcall leaves none.
Ran: a function returning 20 strings to `MultiValue` aborts debug builds with a Lua API assertion. Release writes past `ci->top` without any check; that write is inferred from reading.
Fix: call `check_stack(state, 1)` or more after the pcall, as `Thread::resume` does.

### Table::clear leaks one stack slot per call
Location: src/table.rs:414-427.
Problem: there is no `StackGuard`, so a slot leaks on each call.
Ran: `clear()` in a loop fails with `StackError` at call 999,996.
Fix: add the guard.

### Thread::status and reset mishandle active threads
Location: src/thread.rs:246-260 and 330-336.
Problem: only the current thread counts as Running.
Ran:
- A coroutine that resumed the current one reports `Resumable`, and the main thread reports `Finished` from inside a coroutine.
- `reset` on such a thread returns `Ok` and clears its active frame, which produced "mlua internal error: 3 too many stack values popped" and a wrong result. Luau behaves the same way, including `reset` returning `Ok`.
Fix: detect non-suspended threads and refuse.

### Dropping a call_async future resumes the script
Location: src/thread.rs:530-537.
Problem: the "terminate" signal is also delivered to plain `coroutine.yield()` calls, so user code continues running during drop.
Ran: `coroutine.yield(); side_effect = ...` had set the global after the drop.
Fix: send the signal only to threads suspended in the async poller.

### Table == slice is true when the table is only a prefix
Location: src/table.rs:902-914.
Problem: the loop runs over the table length, and nothing compares lengths at the end.
Ran: `{1,2} == [1,2,3]` and `{} == [9]` are both true.
Fix: return `len == other.len()`.

### HashSet and BTreeSet do not round-trip
Location: src/conversion.rs:1078 and 1102.
Problem: `into_lua` writes `{[v]=true}`. `from_lua` treats any table with `raw_len > 0` as a sequence of values.
Ran: `HashSet<i64>{1,2,3}` fails with "error converting Lua boolean to i64". `BTreeSet` fails the same way.
Fix: decide by the value type, or fall back to keys.

### detect_mixed_tables turns small sparse tables into giant arrays
Location: src/table.rs:803 and 775-778.
Problem: `len < 10` bypasses the sparsity check.
Ran:
- `{[2^40]=1}` serialized to JSON ran past a 30 s timeout unconstrained, and aborted on allocation under the 2 GB limit.
- The `from_value` path was killed.
- `{[1000]=1}` produced 4998 bytes.
By reading, `math.huge` as a key saturates to `usize::MAX`.
Fix: apply the sparsity check always, and reject huge indexes.

### detect_mixed_tables ignores other documented options
Location: src/table.rs:800-806.
Ran with the option on: `{}` gives `[]` even though `encode_empty_tables_as_array` is false, and `setmetatable({a=5}, array_mt)` gives `{"a":5}`, despite the array metatable docs saying "systematically".
Fix: check the array metatable and the empty case first.

### from_value rejects floats with integer values
Location: src/serde/de.rs:181; the doc at src/value.rs:42 also claims otherwise.
Ran: `{a = 10/2}` into `struct { a: u32 }` gives "invalid type: floating point `5.0`", while `unpack::<u32>` of the same value returns 5.
Fix: visit integer-valued floats as integers.

### Loading a chunk from a Path rejects shebang and BOM files
Location: src/chunk.rs:94-112.
Ran: a file starting `#!/usr/bin/env lua`, and a UTF-8 BOM file, both give syntax errors through `lua.load(path)`. `dofile` runs both.
Fix: skip them the way `luaL_loadfile` does.

### equals disagrees with Lua's == and its own docs
Location: src/userdata.rs:954-957; src/table.rs:229-238.
Ran:
- For userdata of different types, Lua's `a == b` is true while `Value::equals` is false, though the docs at src/value.rs:110-113 say the second value's metamethod is tried.
- A callable-table `__eq` gives `Err` from `Table::equals`.
Fix: follow Lua's lookup and call rule.

### Error text inverts booleans
Location: src/util/mod.rs:251. Also line 257, which reads index -1 instead of `index` (read only).
Ran: a `__close` doing `error(true)` makes `Thread::reset` return `RuntimeError("false")`.
Fix: compare `!= 0`.

### is_sync no longer works, so send breaks shared borrows of Sync types
Location: src/userdata/util.rs:33-41.
Problem: the trick relies on a private std specialization.
Ran: on rustc 1.99.1 a copy of it returns false for `u64`. With `send`, `tests/send.rs::test_userdata_multithread_access_sync` fails because a second `UserDataRef` is refused. The result is restrictive rather than unsound.
Fix: replace the trick, for example by storing the fact from a `T: Sync` constructor.

### send: AnyUserData::take can panic on a racing drop
Location: src/userdata/cell.rs:81.
Problem: `UserDataRef` unlocks before dropping its `XRc`, so `unwrap` can see a second owner.
Ran: `take` racing a drop on another thread panicked on `Option::unwrap()`.
Fix: handle the `None` case, or drop the reference before unlocking.

### Most unsafe has no SAFETY comment
Location: throughout src/.
Problem: clippy reports 198 unsafe blocks and 8 unsafe impls without one. This is the count from the advisory pass with `lua54,vendored,serde,macros,async,send`. Given the findings above, those justifications are needed.
Fix: add them, starting with the unprotected Lua API calls.

## Minor

### try_cache loses chunk names
Location: src/chunk.rs:673.
Ran: errors show `[string "?"]:13` instead of `__mlua_async_poll`.
Fix: compile with the name.

### Value equality loses precision
Location: src/value.rs:617-618.
Ran: `Integer(2^53+1) == Number(2^53)` is true, while Lua says false.
Fix: compare exactly.

### Value::to_string formats numbers unlike Lua
Location: src/value.rs:174.
Ran: `1.0` gives `"1"`, and `1e100` gives 101 digits rather than Lua's `1e+100`.
Fix: use Lua's format, or document the difference.

### Buffer seek overflow and wrong message (Luau)
Location: src/buffer.rs:135-137; also 41-57.
Ran:
- `SeekFrom::End(i64::MAX)` panics on add overflow in debug, and `Current` does the same.
- `Start(u64::MAX)` reports "negative position".
- `read_bytes` and `write_bytes` panic with no `# Panics` doc; I ran `read_bytes` only, and `write_bytes` is from reading.
Fix: use checked arithmetic and document the panics.

### gc_inc and gc_gen hit unreachable! during a finalizer
Location: src/state.rs:1065 and 1084.
Ran: `gc_inc` called from a `__gc` callback panics, because `lua_gc` returns -1.
Fix: handle -1.

### Handles panic on Debug after the Lua instance is dropped
Location: src/types/value_ref.rs:42; src/string.rs:119.
Ran: `format!("{:?}", t)` panics with "Lua instance is destroyed". This is not documented on the handle types.
Fix: print a placeholder.

### Any 3-field tuple struct named Vector becomes a Luau vector
Location: src/serde/ser.rs:272.
Ran: `struct Vector(String, String, String)` fails to serialize, and the same fields under the name `Foo` work.
Fix: use a private marker name.

### get_path cannot express i64::MIN
Location: src/util/path.rs:81-92.
Ran: `[-9223372036854775808]` reports integer overflow. An exhaustive run of 5.2M short paths found no panics.
Fix: parse the value negatively.

### Sequence length is declared before elements are skipped
Location: src/table.rs:1093-1100.
Ran: `{1, print, 3}` with `deny_unsupported_types(false)` produces `[1,3]`. By reading, the seq was declared with length 3, which would corrupt length-prefixed formats.
Fix: count first.

### create_async_function needs the coroutine global
Location: src/state/raw.rs:1358.
Ran: with the global removed it fails with "error converting Lua nil to table". By reading, it would also use a script's replacement `yield`.
Fix: fetch `yield` without going through globals.

### Internal errors leak for non-string userdata keys
Location: src/userdata/util.rs:360 and 411.
Ran:
- `g[{}]` gives "__mlua_index:33: attempt to concatenate a table value" (reading a non-string key).
- `g.zz` errors on a getters-only type, while the same access returns nil once the type has methods.
By reading, the same concatenation is in the setter path.
Fix: use `tostring` and consistent behaviour.

### Memory limit also rejects shrinks
Location: src/memory.rs:135.
Problem: read only. After lowering the limit, shrinking reallocations can return NULL, and Lua's allocator contract says shrinks do not fail.
Fix: skip the limit check when `mem_diff <= 0`.

### One bad compiler name silently disables the whole list
Location: src/chunk.rs:415-420.
Problem: read only. A NUL in one entry makes `unwrap_or_default` drop every entry.
Fix: return an error.

### Documentation and lint hygiene
Location: throughout src/, advisory clippy pass.
Problem: missing docs on 185 functions, 76 constants, 18 struct fields and 11 variants, plus smaller counts on other items; 122 missing `# Errors`; 21 types without `Debug`; 14 `map_err(|_|`; 18 `unwrap`; 1 `allow` without a reason; 1 bare TODO at src/util/mod.rs:28.
Fix: fill these in, starting with the public `Result`-returning functions.

### Clippy and rustdoc failures
Location: src/conversion.rs:534 and 667, src/error.rs:300, src/string.rs:133, src/util/mod.rs:279; src/thread.rs:447; src/state.rs:880.
Problem:
- Five clippy `-D warnings` errors, which look like drift on this newer toolchain.
- Rustdoc `-D warnings` fails on an unresolved `Lua::sandbox` link at thread.rs:447, because `Lua` is not in scope there.
- Rustdoc also fails on an ambiguous `Debug` link at state.rs:880.
Fix: fix the two links and update for the new lints.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked --features lua54,vendored,serde,macros,async,send --all-targets -- -D warnings`: fail, 5 errors, listed above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --features lua54,vendored,serde,macros,async,send`: fail, 2 errors, listed above.
- `cargo test --locked --no-fail-fast --features lua54,vendored,serde,macros --lib --tests`: only the `table` test binary failed, killed with SIGKILL. `test_table_with_large_capacity` is the one test that did not report a result, and it requests 2^26 + 2^26 slots. The container limit is 2.5 GB, so I treat this as an environment artifact, not a finding.
- The same with `async,send` added: that SIGKILL, plus `test_userdata_multithread_access_sync` failing. The second is reported above.
- Probes: `cargo build` and `cargo build --release` in each probe crate, then `probe <name>`, `p53`, `pluau <name>` and `psend <name>`. All were run with `lua54`, except `p53` (`lua53`), `pluau` (`luau`) and `psend` (`lua54,send`).
- `cargo miri test`: skipped, because the crate's core runs through the C Lua FFI, which Miri cannot execute.
- Not run: the `userdata-wrappers` paths beyond a compile check, LuaJIT and Lua 5.1 or 5.2. The Luau `require` code was read only.

Findings: 47. Last clock reading: 28 minutes used of 45.
