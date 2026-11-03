# Audit of heapless 0.9.1

## Summary

Do not rely on this release as sound. Safe code reaches memory corruption, double frees and data races in `Deque`, `SortedLinkedList`, `mpmc`, `spsc`, `pool`, `Vec`'s owned iterator, `HistoryBuf` and `IndexMap`. The existing test suite and Miri run both pass because none of them exercise these paths.

Probes are in `probe/` and `probe2/` in the workspace. `crate/` source was not changed. Nothing in the repository addressed me as an agent.

## High

**1. `make_contiguous` on a full deque causes a write past the buffer**
`crate/src/deque.rs:489`
when `free == 0` and `front_len <= back_len`, `back` is set to `len`, which equals `N`. The field's own invariant is `0..=N-1`. The next `pop_front` and `push_back` write to index `N`. A caller sees silent corruption of whatever follows the deque.
on a `Deque<u64, 4>` rotated by 2 and filled, I called `make_contiguous`, `pop_front`, then `push_back(0xDEADBEEF)`.
- Release build: a neighbouring `repr(C)` canary field read back `0xdeadbeef`.
- Debug build: the `get_unchecked_mut` precondition check aborted at `deque.rs:654`.
- Miri also reports an error at `deque.rs:654`.

**2. `make_contiguous` corrupts the length in its second branch**
`crate/src/deque.rs:410`
`back` is set to 0, but the correct value is `back + len`, wrapped. `len()` then grows and includes an uninitialized slot, which `iter` and `Drop` treat as an element.
an exhaustive run over `Deque<u32, 8>` found wrong lengths at (front, len) = (3, 6) and (4, 5), each reporting 7. With `Deque<Box<u32>, 12>` and front 7, len 8, `len()` became 9 and the process aborted with `free(): double free detected`.

**3. `FindMut::pop` after mutation pops twice**
`crate/src/sorted_linked_list.rs:713`, with the `Drop` at `:758`
`pop` leaves `maybe_changed` set, so `Drop` runs `pop_internal` and `push_unchecked` again. For a non-head node this links the free list into the live list and duplicates the value. For a single-element list it reads node index `MAX`.
`let mut f = ll.find_mut(|x| **x == 2).unwrap(); **f = 20; let v = f.pop();` on `Box<u32>` aborted with "null pointer dereference occurred". A single-element list panicked at `sorted_linked_list.rs:290`. The Miri run also errored, but my output filter dropped its message.

**4. 8-bit MPMC positions wrap during a thread stall (ABA)**
`crate/src/mpmc.rs:85` and `:90`, used in `dequeue` and `enqueue`
without `mpmc_large`, positions are `u8`. A thread stalled between reading `seq` and its compare-exchange succeeds once the position has wrapped 256 operations later. It can then overwrite a full cell or read an empty one. Callers see lost items, duplicated items (a double free for owning types) and a permanently wedged queue. The module docs say threads can use this safely.
`Queue<u64, 2>` with 12 spinning producers and 12 spinning consumers on 4 CPUs, tracking every value:
- 180 s run: 2 values lost, all 12 producers stuck.
- 40 s run: 1 lost, 1 delivered twice, all 12 producers stuck.
- A second 40 s run hit its 100 s timeout with no result.
- Control with `mpmc_large`: 69,233,830 values, none lost or duplicated, none stuck.

**5. `Consumer::peek` is a data race with the producer**
`crate/src/spsc.rs:211`, reached through `:269` and `:822`
`peek` decides the slot is ready from a Relaxed load of `tail`, then reads the slot. There is no happens-before with the producer's write, so a consumer on a weakly ordered CPU can see an unwritten value.
Miri with 16 seeds on a two-thread `split()` test reports "Data race detected between non-atomic write and retag read" at `spsc.rs:273`.

**6. `Deque::clear` double-drops if an element's `Drop` panics**
`crate/src/deque.rs:237`
elements are dropped before the indices are reset, so a panic leaves them all marked live.
`catch_unwind(|| d.clear())` followed by `drop(d)` aborted with `free(): double free detected`.

**7. `vec::IntoIter::drop` double-drops on panic**
`crate/src/vec/mod.rs:1527`
the remaining elements are dropped before `vec.len` is zeroed. On unwind, the inner `Vec` drops everything from index 0 again.
dropping an `IntoIter` whose element panics in `Drop` aborted with `free(): double free detected`.

**8. `HistoryBuf::write` double-drops on panic**
`crate/src/history_buf.rs:376`
the old value is dropped in place while its slot still counts as initialized.
`HistoryBuf<_, 1>`, with a panicking overwrite under `catch_unwind` and then `drop`, aborted with a double free.

**9. `HistoryBuf::clear` double-drops on panic**
`crate/src/history_buf.rs:318`
this is the same ordering fault as finding 6.
the same probe aborted with a double free.

**10. `IndexMap` indices go stale when user code panics during removal**
`crate/src/index_map.rs:308` (retain), `:963` (clear), `:1269` (truncate)
entries shrink, but the index rebuild or reset is skipped on unwind. Lookups then call `get_unchecked(i)` with `i >= len` at `:188`. `IndexSet::retain` reaches the same code.

- Debug build: a panic in the `retain` closure, then `remove(&3)`, tripped `debug_assert` at `index_map.rs:182`.
- Miri in release with Tree Borrows: UB at `index_map.rs:188` for both `retain` and a panicking `truncate`.
- `clear`: my probe only called `get`, which returns early on an empty map, and Miri passed. The stale read through `insert` there comes from reading the code only.

**11. Box and Arc pools race on the node's `next` word**
`crate/src/pool/treiber/cas.rs:217`, against `crate/src/pool/boxed.rs:331` and `crate/src/pool/arc.rs:180`
`try_pop` atomically loads `next` from a node that another thread may have already popped and be writing as data, because `UnionNode` overlaps the two. The tag check discards the bad value, but the access is still a data race. `object` pools use `StructNode` and do not overlap (read only).
Miri with 3 threads on `box_pool!` reports a data race between the atomic load and the non-atomic write at `boxed.rs:331`, on every seed shown. The Arc side is by reading the code.

**12. `IndexMap` breaks at capacity 65536 and above, with no limit enforced**
`crate/src/index_map.rs:103` and `:104`
`Pos` packs a 16-bit hash and a 16-bit index, but `new()` only asserts that `N` is a power of two greater than 1.

- `N = 65536`, last entry with hash and index both `0xffff`: `NonZeroU32::new_unchecked(0)`. Debug aborts on the precondition. In release the entry vanishes (`len = 65536`, `get = None`).
- `N = 131072` with 100,000 keys, release: 34,464 lookups were wrong. Debug panics with add overflow at `:104`.

## Medium

**13. `IndexMap::truncate` makes live keys unfindable**
`crate/src/index_map.rs:1269`
index slots are cleared without backward-shifting, which leaves holes in Robin Hood probe chains.
763 of 2,000 random `FnvIndexMap<u32, _, 8>` runs left a key in `keys()` with `get()` returning `None`. A direct case with an identity hash, inserting keys 1, 8, 16 and then `truncate(2)`, gives `get(&1) == None`.

**14. `BufMut` cannot fill the last byte**
`crate/src/bytes.rs:19`
`pos >= capacity` rejects a legal advance to exactly full.
`Vec<u8, 8>` with `put_slice` of 8 bytes panics with "Advance out of range". `Vec<u8, 2>` with two `put_u8` calls panics the same way, even though `remaining_mut` reported 1.

**15. `LinearMap::from_iter` keeps duplicate keys**
`crate/src/linear_map.rs:534`
it extends the buffer directly and skips `insert`.
collecting `[(1, 'a'), (1, 'b')]` gives `len == 2`, whereas a map should have one entry. `get` then returns the first value rather than the last.

**16. `IndexMap::into_iter` yields reverse order**
`crate/src/index_map.rs:1455`
it pops from the back, while `iter()` yields insertion order and nothing documents the difference.
for keys 1, 2, 3, `iter` gives `[1, 2, 3]` and `into_iter` gives `[3, 2, 1]`.

**17. `ready()` followed by an `_unchecked` call races**
`crate/src/spsc.rs:773`, `:847`, `:299`, `:342`
`ready` uses Relaxed loads and the unchecked paths never load the other index with Acquire, so the documented pattern has no synchronization.
Miri reports data races at `spsc.rs:344` (`dequeue_unchecked`) and `:302` (`enqueue_unchecked`).

**18. `Vec::retain`'s panic guard violates Stacked Borrows**
`crate/src/vec/mod.rs:1181`
`as_ptr()` is taken and then `as_mut_ptr()` invalidates it, because the buffer is inline. This is only reached when a predicate or `Drop` panics after a deletion.
Miri reports an error at `mod.rs:1180` for both a panicking predicate and a panicking `Drop`. An `IndexMap::retain` run under Tree Borrows got past this point, so the violation is specific to the Stacked Borrows model.

**19. `CString::from_bytes_with_nul` contradicts its docs**
`crate/src/c_string.rs:80`
the docs say it fails if the slice does not end with a nul. It appends one instead.
`from_bytes_with_nul(b"abc")` returns `Ok("abc")`, and `b""` is also accepted.

## Low

**20. `HistoryBuf::new_with` lacks the `N > 0` check**
`crate/src/history_buf.rs:276`
`new` has the check and `new_with` does not.
`HistoryBuf::<u8, 0>::new_with(1)` builds, reports full, and `recent()` panics on subtraction overflow at `:432`.

**21. `as_view` and `as_mut_view` require `T: Copy`**
`crate/src/history_buf.rs:288`
they sit in the `Copy`-bounded impl block, though the docs offer them generally.
calling `as_view` on `HistoryBuf<String, 4>` fails to compile with E0599.

**22. `LinearMap` implements `Eq` for `V: PartialEq`**
`crate/src/linear_map.rs:634`
the bound is too weak for `Eq`.
`LinearMap<u32, f32, 2>` satisfies `Eq`, yet a map holding NaN has `m != m`.

**23. `VecView` cannot be serialized**
`crate/src/ser.rs:52`
the impl lacks `?Sized`, unlike `BinaryHeapInner` and the other view types.
read only, not compiled.

**24. Numeric `TryFrom` uses `()` as its error**
`crate/src/string/mod.rs:1081`
`()` is not an error type callers can work with, although `CapacityError` exists.
read only.

**25. Undocumented `unsafe`**
crate-wide, for example `crate/src/deque.rs:274` and `crate/src/pool/treiber/cas.rs:217`
113 unsafe blocks and 19 unsafe impls have no `SAFETY` comment.
audit clippy lints.

**26. Missing `Debug`**
for example `crate/src/spsc.rs:597`, `:737`, `:746`
43 public types do not implement `Debug`.
audit clippy lints.

**27. Missing doc sections**
crate-wide
32 functions lack `# Errors` and 3 lack `# Panics`.
audit clippy lints.

**28. Lint failures**
for example `crate/src/de.rs:46`, `crate/src/mpmc.rs:129`, `crate/src/string/mod.rs:353`
`clippy -D warnings` fails with 13 library errors on Rust 1.99.1. The crate pins no toolchain, so this is probably toolchain drift. Separately, 10 `allow` attributes have no reason.
clippy run.
