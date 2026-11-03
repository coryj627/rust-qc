# Audit report: heapless 0.9.1

## Summary

heapless 0.9.1 passes its own tests and its library test suite is clean under Miri, but the audit found serious defects those tests miss. Safe code can reach out-of-bounds writes, double frees and data races, and `IndexMap::truncate` silently loses retained keys. 

`crate/` content is unchanged. I ran `touch src/lib.rs` once to force a lint rerun, which changes only the file's modification time. Probes live in `tmp/probe`, plus a hooked copy of the crate in `tmp/crate_hook` and `tmp/aba`. Nothing in the repository or its output addressed me.

## High

**IndexMap::truncate loses keys it keeps**
src/index_map.rs:1271-1278.
`truncate` sets removed index slots to `None` without backward-shifting, so it cuts Robin Hood probe chains. With the default hasher, `FnvIndexMap<u32, u32, 8>`, insert keys 0, 1, 9, then `truncate(1)`:
- `get(&0)` and `contains_key(&0)` report absent, while iteration still shows `{0: 1}`.
- `insert(0, 9)` returns `Ok(None)` and the map becomes `{0: 1, 0: 9}`.
A search over keys below 40 found 800 such triples.
ran it natively.

**Deque::make_contiguous on a full deque allows an out-of-bounds write**
src/deque.rs:488-489.
this branch sets `back = len`. When the deque is full, `back == N`, breaking the "0..=(N-1)" invariant, and `increment` then never wraps. Sequence on `Deque<u32, 8>`: push 8, pop 5, push 5, `make_contiguous()`, `pop_front()`, `push_back(99)`. The last call writes to `buffer[8]`.
Miri aborted on the unsafe precondition check at deque.rs:654.

**Deque::make_contiguous corrupts the length and causes a double free**
src/deque.rs:409-410.
the "free >= back_len" branch leaves the data at `back..back + len` but sets `back = 0`. That is only right if the data ends at the buffer's end. With `Deque<String, 12>`, front 7, back 3, len 8:
- After the call, `len()` returns 9.
- `Debug` prints a moved-from ninth element.
- Drop frees it twice.
native assertion (left 9, right 8) and Miri use-after-free. A differential test against `VecDeque` also failed. The existing test at deque.rs:1309 misses these two branches.

**FindMutView::pop after a mutable deref pops twice**
src/sorted_linked_list.rs:713-715 and 758-763.
`pop` calls `pop_internal`, then `Drop` runs and, because `maybe_changed` is true, calls `pop_internal` again. `let mut f = ll.find_mut(..).unwrap(); f.push('x'); let v = f.pop();` is enough:
- With three `String`s, Miri reported use-after-free.
- With one element, it indexed slot `Idx::MAX` through `get_unchecked`.
Miri, both cases.

**spsc checks readiness with Relaxed loads, which is a data race**
src/spsc.rs:210-218 (used by `peek` at 268-275 and `ready` at 773 and 847; the unchecked paths are at 299-305 and 342-350).
`is_empty` and `is_full` load both indices `Relaxed`, so nothing synchronises with the other thread's write. Documented patterns race:
- `Consumer::peek` followed by reading the value.
- `if c.ready() { c.dequeue_unchecked() }`.
- `if p.ready() { p.enqueue_unchecked(x) }`.
On weak-memory hardware the consumer can read a stale or uninitialized slot.
Miri reported a data race in all three, at spsc.rs:344 and spsc.rs:302 and on the `&T` from `peek`. All three reports used only the public API, though two of the patterns call `unsafe` methods as documented.

**mpmc position counters wrap and lose an item, then dequeue hangs**
src/mpmc.rs:85 and 90, with the CAS at 317.
with the default `u8` counters, the CAS cannot tell a position from the same value 256 operations later. If an enqueue is preempted between its load and CAS while an interrupt or thread does 256 enqueues and leaves the queue full, the CAS succeeds. The enqueue then overwrites an occupied cell. The overwritten item is lost without its destructor and the sequence number is wrong.
in a copy of the crate I added only a hook before the CAS. On `Queue<u32, 2>` the hook ran 254 enqueue/dequeue pairs and two enqueues. The outer `enqueue(7)` returned `Ok`, and the next `dequeue` spun forever (still running after 5 seconds). A native stress test of 36M operations did not hit the window, so this needs specific timing.

**IndexMap has no capacity limit, but its position type holds 16 bits**
src/index_map.rs:100-107 and the missing assert at 733-736 and 1349-1352.
`Pos` packs the index into 16 bits.
- `N = 65536`: hash 0xFFFF with index 0xFFFF wraps to zero inside `NonZeroU32::new_unchecked`.
- `N = 131072`: index 65536 carries into the hash field and the key is lost.
native runs. The first panicked with "NonZero::new_unchecked requires the argument to be non-zero" at line 103. The second failed with "lost key 65536".

**IndexMap indices go stale when retain, clear or truncate panics**
src/index_map.rs:308, 963, 1269.
the entries shrink before the indices are updated, and user predicates and destructors run in between. After `catch_unwind`, `find` and `insert` use `get_unchecked` with stale indices.

- `clear`: Miri reported "`assume` called with `false`" at line 227 with debug assertions off. Debug builds hit the assert at line 211.
- `retain`: the map was left with stale indices. Under Tree Borrows, lookups returned `None` for keys still present until the debug assert at line 182 fired.
- `truncate`: same pattern, read only.

**Vec IntoIter double-drops when an element destructor panics**
src/vec/mod.rs:1527-1529.
`len = 0` is set after `drop_in_place`. If a destructor panics, the `vec` field's own `Drop` runs with the old length and drops everything again, including elements already moved out.
Miri reported a double free.

**Deque::clear double-drops when a destructor panics**
src/deque.rs:235-241.
the state is reset only after `drop_contents` returns, so after a caught panic, dropping the deque drops the elements again.
Miri reported a double free.

**HistoryBuf::clear and write double-drop when a destructor panics**
src/history_buf.rs:316-321 and 373-378.
`clear` has the same ordering fault as `Deque::clear`. `write` drops the old slot in place, so a panic leaves that slot counted as live.
Miri reported a double free for both.

## Medium

**BufMut cannot fill a Vec to capacity**
src/bytes.rs:19.
`pos >= self.capacity()` should be `>`. `remaining_mut()` says 8, yet `put_slice(&[1; 8])` on `Vec<u8, 8>` panics with "Advance out of range". So does `put_u8` on the last free byte.
ran it natively. The existing tests only try 9 on a capacity of 8.

**LinearMap::from_iter keeps duplicate keys**
src/linear_map.rs:534.
it extends the buffer directly and skips the key check. `[(1, "a"), (1, "b")].into_iter().collect()` gives `{1: "a", 1: "b"}` with `len()` 2, which breaks `remove` and `PartialEq`.
ran it natively.

**IndexMap::into_iter runs backwards**
src/index_map.rs:1455.
`next` uses `entries.pop()`, so owned iteration yields `[3, 2, 1]` while `iter()` yields `[1, 2, 3]`. Every other iterator on the type documents insertion order.
ran it natively.

**CString::from_bytes_with_nul accepts input its docs say it rejects**
src/c_string.rs:78-86.
the docs say it fails if the slice does not end with a nul. `from_bytes_with_nul(b"abc")` returns `Ok("abc")` and `b""` returns `Ok("")`.
ran it natively.

**Pool pop reads next while another thread writes the same union memory**
src/pool/treiber/cas.rs:217, racing with src/pool/boxed.rs:331 (`UnionNode` shares `next` and `data`).
`try_pop` loads `next` from a node another thread may already have popped and be writing. The tagged CAS discards the value, so I saw no corruption, but it is a data race under Rust's memory model.
Miri reported "atomic load ... and non-atomic write" using only safe `alloc` and `drop` on three threads, under `-Zmiri-many-seeds`.

**LL/SC pool has no memory barriers on multi-core ARM**
src/pool/treiber/llsc.rs:62-115 and 133-143, selected by build.rs:32-37.
`arm_llsc` is set for any target whose compiler accepts `clrex`, which appears to include multi-core Armv7 Linux targets. `ldrex` and `strex` carry no `dmb`, so an `Object` handed between cores can expose stale contents.
read the code only. No ARM target is available here.

## Low

**Vec::retain's panic guard breaks Stacked Borrows**
src/vec/mod.rs:1180-1186.
the source pointer from `as_ptr` is invalidated by `as_mut_ptr` before `ptr::copy` reads through it. It only runs when the predicate panics.
Miri reported an error under Stacked Borrows and passed under Tree Borrows.

**LinearMap implements Eq without V: Eq**
src/linear_map.rs:631-636.
the bound is `V: PartialEq`, so `LinearMap<u8, f32, 2>` is `Eq` while a map holding NaN is not equal to itself.
compiled and ran it.

**HistoryBuf::new_with allows N = 0**
src/history_buf.rs:276.
`new` has a const assert but `new_with` does not. `HistoryBuf::<u8, 0>::new_with(1).recent()` panics with a subtract overflow at line 432.
ran it natively.

**Serialize is missing for VecView**
src/ser.rs:52.
the bound lacks `?Sized`, unlike `DequeView` and `StringView`, so serializing a `&VecView` does not compile.
compile check failed with E0277.

**Arc::clone panics rather than aborting on refcount overflow**
src/pool/arc.rs:251.
the count has already been incremented when it panics, so a caught panic can repeat until the count wraps and causes a use-after-free. Reaching it needs about 2^31 leaked clones on 32-bit.
read the code only.

**The ARM probe misidentifies arm64-named targets**
build.rs:32.
the exclusion only checks for the `aarch64` prefix. The target list here has `arm64e-apple-*`, `arm64ec-pc-windows-msvc` and `arm64_32-apple-watchos`. These should get `arm_llsc` and then fail on `ldrex` when the pool is used.
read the code and listed the targets. Not compiled for them.

**Iterators lack size information**
src/vec/mod.rs:1449, src/deque.rs:911, src/index_map.rs:1496, src/linear_map.rs:593, src/spsc.rs:619.
`size_hint` is `(0, None)`, with no `ExactSizeIterator`, on these iterators. `vec::IntoIter` also has no `DoubleEndedIterator`.
printed the hints for `vec::IntoIter`, `deque::IntoIter` and `index_map::Iter`; the rest read only.

**Container API gaps**
src/index_set.rs:543 and 596, src/sorted_linked_list.rs:267-269.
`IndexSet` has no `Eq` and no by-value `IntoIterator`. `SortedLinkedList` accepts `u32` as `Idx` but has no constructor for it.
read the code only.

**Weak error types**
src/string/mod.rs:1081, src/c_string.rs:332, src/string/mod.rs:29.
`TryFrom<number> for String` has `Error = ()`. `ExtendError` and `FromUtf16Error` are exhaustive and do not implement `source()`.
read the code only.

**Documentation gaps and errors**
src/history_buf.rs:389 ("`self.len()`" should be capacity), src/lib.rs:95 (lists `Arc` twice and omits `Box` from the list in that position), src/lib.rs:123 (stale `MpMcQueue`), src/index_set.rs:425 (`//` drops the summary line), src/sorted_linked_list.rs:4 ("never memcpy" is false for `find_mut` re-sorting).
the docs above are wrong. Also:
- Lints report 32 missing `# Errors` and 3 missing `# Panics` sections.
- Capacity-overflow panics are undocumented at src/vec/mod.rs:1435, src/deque.rs:893, src/index_map.rs:1395, src/string/mod.rs:835 and src/pool/boxed.rs:189.
read the code and ran the lints.

**SAFETY comments are missing**
113 `unsafe` blocks and 19 `unsafe impl`s. The ones most worth documenting are src/mpmc.rs:242, src/pool/boxed.rs:346 and src/vec/drain.rs:59-60.
`clippy::undocumented_unsafe_blocks`.

**Debug is missing on 43 public types**
for example src/deque.rs:813, src/spsc.rs:737 and 746, src/binary_heap.rs:570, src/sorted_linked_list.rs:651.
`missing_debug_implementations`.

**Lint hygiene**
across src/, for example src/history_buf.rs:70.
10 `#[allow]`s without a reason, 46 asserts without messages, 10 `unwrap` calls in library code. The scan's 5 bare TODOs are src/binary_heap.rs:6, src/lib.rs:187, src/pool/arc.rs:250, src/slice.rs:3 and src/vec/drain.rs:149.
audit lints and the heuristic scan.

**Toolchain drift**
src/lib.rs:136 for the docs flag; clippy findings across the crate.
clippy with `-D warnings` fails on 1.99.1 with 18 errors on the library target, such as needless `return` and `ref as ptr`. `--cfg docsrs` fails with E0557 because `doc_auto_cfg` was removed in 1.92, so docs.rs-style builds fail on current nightly.
ran both.
