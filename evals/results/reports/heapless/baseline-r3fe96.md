# Audit report: heapless 0.9.1 (`crate/src/`)

**Baseline:** the crate's own suite passes: `cargo test --locked` with the brief's features (220 unit tests, 184 doctests and the rest), and Miri on lib tests (196 passed, 2 ignored). All 22 defects below get past that suite.

**Probes:**
- They are in `probe/tests/{p,q,bal,diff,pool}.rs` and `probe/src/bin/mpmc.rs`.
- They ran through `./run`, natively in debug and release, and under Miri where stated.
- `crate/` was not changed. One probe (finding 7) uses a patched copy at `tmp/hook/crate`.
- No repository text or program output addressed me.

## Memory unsafety from safe code, no panic needed

**1. `sorted_linked_list.rs:713-715` with `:758-763`: `FindMutView::pop` after a mutable access pops twice. Critical.**
- `pop()` never clears `maybe_changed`, so `Drop` runs `pop_internal()` again and re-pushes the result.
- Calling `*find = x; find.pop()` corrupts the list.
  - Non-head element, `Box` items: the list becomes `[3, 3, 0]` and the program double-frees.
  - Only element: index `MAX` is read out of bounds and the program segfaults in release.
- Established by running: Miri reports UB (uninitialized `Box` read; out-of-bounds `get_unchecked` at `:290`). Native release gives a double free and a SIGSEGV.

**2. `deque.rs:488-489`: `make_contiguous` on a full, wrapped deque sets `back = N`. Critical.**
- This happens when `front_len <= back_len` and `free == 0`, and it breaks the `0..=N-1` invariant.
- A following `pop_front` then `push_back` writes to `buffer[N]`.
- Popping never empties the deque: repeated `pop_front` returned `2 3 4 5 2 3 4 5 2 …`, handing out duplicates of moved-out values.
- Established by running: Miri reports "indexing out of bounds: the len is 4 but the index is 4" at `deque.rs:654`. Debug builds abort on the precondition check.

**3. `deque.rs:409-410`: `make_contiguous` sets `back = 0` when `free > back_len`. Critical.**
- The correct value is `back_len + len`. Afterwards `len()` is too large and the extra slots hold stale copies.
- `Deque<u32,16>` with 9 items reports `len=15`, contents `[1..9, 3,4,5,6,7,8]`.
- `Deque<u32,12>` with 8 items reports `len=9`, and `pop_back` returns `5` instead of `8`.
- With `Box` items, `make_contiguous` then `drop` is a double free.
- Established by running: Miri, and the differential test against `VecDeque`, which fails only on this operation.

## Logic and concurrency defects

**4. `index_map.rs:1268-1279`: `IndexMap::truncate` blanks index slots without a backward shift. High.**
- This breaks Robin Hood probe chains, so surviving keys become unreachable.
- Probe result after `truncate(2)` with colliding hashes:
  - `get(x)` and `get(y)` both return `None`, though `len()` is 2.
  - `insert(x, …)` returns `Ok(None)`.
  - Keys become `[257, 258, 257]`, a duplicate.
- Established by running.

**5. `index_map.rs:100-108, 115`, with no bound at `:733-735, :1349-1352`: capacities of 65536 and above break `Pos`. High.**
- `N == 65536`: `{hash 0xffff, index 0xffff}` makes `NonZeroU32::new_unchecked(0)`. This is UB and aborts in debug. In release the slot reads as `None`, so `get(65535)` returned `None` with `len` 65536.
- `N > 65536`: the index is truncated to `u16`. With `N = 131072` and 70000 inserts, 4465 lookups failed to return the inserted value.
- No compile-time limit exists.
- Established by running.

**6. `spsc.rs:269-273` (via `:211`, reached from `Consumer::peek` at `:822`): `peek` checks emptiness with `Relaxed` loads, then reads the slot. High.**
- There is no Acquire pairing with the producer's Release store, so the consumer can read a slot that is not yet visible as written.
- Established by running: Miri reports "Data race … non-atomic write … retag read" at `spsc.rs:273`.
- Native runs pass, as expected on this check.

**7. `mpmc.rs:85-95, 317-326, 288-290`: `u8` sequence counters (no `mpmc_large`) allow ABA after 256 enqueues. High.**
- A caller stalled between loading `seq` and the CAS succeeds once `enqueue_pos` wraps to the same value. It then overwrites an unread item.
- Simulating that stall in a patched copy:
  - `enqueue(777)` on a full queue returned `Ok(())`.
  - The next `dequeue()` spun forever (`Greater` branch), still not finished after 5s.
- Established by the patched copy, because the unpatched API has no way to inject the stall. A 60s stress run of the real crate (8 threads, 89M operations) did not hit it.

**8. `bytes.rs:19`: `advance_mut` uses `pos >= capacity`, an off-by-one. High.**
- `BufMut` cannot fill the buffer. `put_slice(&[1,2,3,4])` on `Vec<u8,4>`, or `put_u8` into the last slot, panics with "Advance out of range".
- Established by running.

## Double drops and stale state when `Drop` or a predicate panics

**9. `deque.rs:237`, `history_buf.rs:318`, `history_buf.rs:376`: state is reset after the drop, not before. High.**
- This affects `Deque::clear`, `HistoryBuf::clear` and `HistoryBuf::write`.
- If an element's `Drop` panics and the panic is caught, the container's own `Drop` drops the elements again.
- Established by running: all three give "free(): double free" natively, and Miri reports the `Deque::clear` case as a dangling box (use-after-free).

**10. `vec/mod.rs:1525-1529`: `vec::IntoIter::drop` zeroes `vec.len` only after `drop_in_place`. High.**
- On a panic, `Vec`'s `Drop` drops everything again, including items already yielded by the iterator.
- Established by running: native double free, Miri dangling box. The cause is the same as finding 9.

**11. `index_map.rs:307-308`, `:963`, `:1269`: indices are not updated if the entry operation unwinds. High.**
- `retain` is affected when its predicate panics. `clear` is affected when a drop panics, and `truncate` has the same statement order (read only).
- Later lookups call `get_unchecked(i)` with `i >= len` and can read dropped keys.
- Established by running: Miri reports UB at `index_map.rs:188` for both the `retain` and `clear` cases. Debug builds hit the `debug_assert` at `:182`.
- The `retain` case needs `-Zmiri-disable-stacked-borrows`, because Stacked Borrows stops earlier on finding 20.

**12. `pool/treiber/cas.rs:217` with the `UnionNode` at `treiber.rs:47`: data race in `try_pop`. Medium.**
- `try_pop` reads `next` from a node that another thread may have popped and be using as `T` storage.
- Established by running: with `--features nightly`, Miri reports "Data race … retag write … atomic load" at `cas.rs:82` on `box_pool!` with 3 threads.

## Medium and low

**13. `linear_map.rs:534`: `LinearMap::from_iter` pushes without checking keys. Medium.**
- `[(1,10),(1,20)]` gives `len` 2 and `get(&1) == Some(&10)`.
- `insert` and the `Deserialize` impl do deduplicate.
- Established by running.

**14. `c_string.rs:80-86`: `from_bytes_with_nul` contradicts its docs. Medium.**
- The docs at `:78-79` say it fails when the slice has no trailing nul. `b"abc"` returns `Ok("abc")` and `b""` returns `Ok("")`.
- Established by running.

**15. `index_map.rs:1455`: `IndexMap::into_iter` yields reverse insertion order. Low.**
- `iter` gives `[0,1,2,3]` and `into_iter` gives `[3,2,1,0]`, while the map's iterators are documented as insertion-ordered.
- Established by running.

**16. `history_buf.rs:276-285`: `new_with` lacks the `N > 0` assert that `new` has. Low.**
- `HistoryBuf::<u8,0>::new_with(1).recent()` panics: subtract overflow at `:432` in debug, index out of bounds at `:414` in release.
- Established by running.

**17. `linear_map.rs:631-636`: `Eq` is implemented with only `V: PartialEq`. Low.**
- `LinearMap<u32, f32, 4>` is `Eq` even though a NaN value makes `m != m`.
- Established by running.

**18. `history_buf.rs:288-299`: `as_view` and `as_mut_view` require `T: Copy`. Low.**
- Calling them on a non-`Copy` element type fails with E0599. The bound serves no purpose, and `Deque` and `Vec` have no such limit.
- Established by compiling.

**19. `ser.rs:52`: `Serialize for VecInner` lacks `?Sized`. Low.**
- `VecView` cannot be serialized, while `DequeView` can.
- Established by compiling.

**20. `vec/mod.rs:1180-1186`: `BackshiftOnDrop` builds `src` with `as_ptr()`, then `dst` with `as_mut_ptr()`, which invalidates `src`. Low.**
- `Vec::retain` with a panicking predicate after a deletion is an aliasing violation under Stacked Borrows.
- Established by running: Miri reports it there, and it passes under `-Zmiri-tree-borrows`.

**21. `string/mod.rs:730`: `String::insert` makes `&mut [u8]` over uninitialized spare capacity. Low.**
- This breaks the `from_raw_parts_mut` contract.
- Established by running: Miri with `-Zmiri-recursive-validation` reports it. Default Miri does not.

**22. `pool/treiber/llsc.rs:62-116, 131-149`: the LDREX/STREX paths have no `dmb`. Low to medium.**
- Node contents and `next` are therefore not ordered across cores on multi-core ARM targets, and `build.rs` enables this path wherever `clrex` assembles.
- Established by reading the code only. This aarch64 container cannot run it.

## No defects found by probing
- Differential tests against std (300k operations each) matched for `Vec`, `String`, `IndexMap`, `LinearMap`, `HistoryBuf`, `BinaryHeap`, `SortedLinkedList` without the pop-after-mutate path, `spsc`, `mpmc`, and `Deque` without `make_contiguous`.
- `Box`-element drop-balance runs under Miri were clean, including threaded `spsc` and `mpmc`.

**Findings: 22. Last `./clock` reading: 15 minutes used of 45.**
