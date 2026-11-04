The existing test suites pass, but additional local tests reproduced correctness failures in normal collection operations, concurrency, and panic cleanup. I found 19 findings: 5 High, 8 Medium, and 6 Low. No crate source files were changed; probes and logs are under `/work/tmp`.

## High

### 1. Concurrent SPSC peeking lacks synchronization

`src/spsc.rs:268`, `src/spsc.rs:823`.

Expected: a consumer may safely inspect an item published by its producer. Observed: `peek` checks availability with relaxed loads, then references the item without acquiring the producer’s release store.

Evidence: a safe two-thread enqueue/peek test passed natively but failed under Miri with a data race at `src/spsc.rs:273`. Ordinary callers using `Consumer::peek` across threads therefore lack the synchronization needed to read the item.

acquire the published tail before accessing its element. Add a concurrent peek regression test.

### 2. Making a full deque contiguous can corrupt its state

`src/deque.rs:489`.

Expected: `make_contiguous` preserves contents and permits subsequent queue operations. Observed: one rotation branch sets `back = capacity`, violating the ring-index invariant.

Evidence: with capacity 2, push `0,1`, pop `0`, push `2`, call `make_contiguous`, then pop `1,2`. The deque incorrectly reports length 2 and exposes `[1,2]` again. A broader ordinary-operation test also aborted in `push_back` at an unchecked-index precondition.

normalize `back` when the contiguous contents fill the buffer. Test draining and refilling after full-buffer rotations.

### 3. Popping a modified sorted-list entry removes it twice

`src/sorted_linked_list.rs:713`, `src/sorted_linked_list.rs:760`.

Expected: modifying a `find_mut` guard and then calling `pop` removes that entry once. Observed: `pop` leaves `maybe_changed` armed, so the guard’s destructor removes another entry and reinserts the removed value.

Evidence: starting with `[3,2,1]`, change the middle value to `20` and pop it. The call returns `20`, but remaining iteration produces `[20,3]` instead of `[3,1]`.

disarm automatic resorting before extracting the entry. Existing tests exercise modification and popping separately.

### 4. Valid writes that fill a vector panic

`src/bytes.rs:19`.

Expected: `BufMut` accepts writes up to `remaining_mut()`. Observed: `advance_mut` rejects `pos == capacity`.

Evidence: both `Vec::<u8,4>::new().put_slice(b"abcd")` and appending the fourth byte with `put_u8` panic with “Advance out of range”. These are safe public operations with sufficient advertised capacity.

permit equality with capacity and check the requested advance against remaining space. Add exact-capacity tests for owned vectors and views.

### 5. Panicking element destructors cause repeated drops

Locations: `src/vec/mod.rs:1527`, `src/deque.rs:237`, `src/history_buf.rs:318`.

Expected: unwinding must not run an element’s cleanup twice. Observed: these paths clear ownership metadata only after dropping elements.

Evidence: local tests used elements whose destructor increments a counter and panics once. After unwinding:

- `Vec::IntoIter` dropped every element twice, including an element already returned to the caller.
- `Deque::clear` and `HistoryBuf::clear` each dropped both elements twice.

Callers’ cleanup actions consequently run repeatedly.

relinquish ownership before invoking destructors, with guards where additional cleanup is necessary. Add panic-cleanup tests.

## Medium

### 6. Truncating an index map makes retained keys unreachable

`src/index_map.rs:1274`.

Expected: truncation preserves lookup of retained entries. Observed: removing index slots creates holes in probe chains without repairing them.

Evidence: insert `0u32`, `1`, and `5` into `FnvIndexMap<u32,u32,4>`, then truncate to 2. Iteration still contains key `0`, but `get(&0)` returns `None`.

rebuild the index or perform proper backward shifting. Test retained-key lookups with collisions.

### 7. Collecting a linear map preserves duplicate keys

`src/linear_map.rs:534`.

Expected: collection follows map insertion semantics, replacing an existing key’s value. Observed: `from_iter` directly extends the underlying vector.

Evidence: collecting `[(1,10),(1,20)]` creates two entries instead of one. With capacity 1, the same input panics despite containing only one distinct key. Callers receive inconsistent lookup, length, and capacity behavior.

construct through `insert`. Add duplicate-key and distinct-capacity tests.

### 8. Default resizing clones one value into every slot

`src/vec/mod.rs:773`.

Expected: each additional slot receives `Default::default()`, as documented. Observed: the implementation creates one default value and clones it.

Evidence: resizing `Vec<Rc<Cell<u8>>,3>` to three elements makes all slots share one cell. Changing slot 0 also changes slots 1 and 2.

invoke `Default::default()` separately for each new element and remove the unnecessary `Clone` requirement.

### 9. CString construction accepts input its contract rejects

`src/c_string.rs:80`.

Expected: `from_bytes_with_nul` rejects missing terminators. Observed: it delegates to the permissive append operation.

Evidence: both `b"abc"` and `b""` return `Ok`, contrary to the documented error contract. Callers cannot use this constructor to validate terminated input.

validate construction separately and represent missing termination in the error API, accounting for the exhaustive public error enum.

### 10. Owning index-map iteration reverses the collection’s order

`src/index_map.rs:1455`.

Expected: owning iteration preserves the insertion order exposed by the index-map API. Observed: it repeatedly pops the final entry.

Evidence: insertion order `[1,2,3]` becomes `[3,2,1]` through `into_iter`, while borrowed iteration remains `[1,2,3]`. Consuming a map therefore changes processing order. The existing owning-iterator test checks membership but misses order.

use the underlying vector’s forward owning iterator.

### 11. A panicking retain predicate leaves stale map indices

`src/index_map.rs:307`.

Expected: after a caller catches a predicate panic, remaining entries stay usable. Observed: vector compaction completes during unwinding, but index rebuilding is skipped.

Evidence: insert keys `[0,1,2]`; remove `0` through `retain`, then panic while visiting `1`. After catching the panic, iteration contains `(1,1)` and `(2,2)`, but `get(&1)` returns `None`.

rebuild indices through an unwind guard after entry compaction. The same implementation serves `IndexSet::retain`.

### 12. Accepted map capacities exceed the packed index representation

`src/index_map.rs:115`; constructors at `src/index_map.rs:744` and `src/index_map.rs:1345`.

Expected: accepted capacities support every insertion up to capacity. Observed: positions decode entry indices through `u16`, while constructors accept larger powers of two.

Evidence: a capacity-131072 map accepted 65537 integer entries and reported that length, but lookup of the last inserted key, `65536`, returned `None`. The test used a larger thread stack to accommodate the legitimate container size.

enforce a compile-time limit consistent with the packed representation and reserved sentinel, or widen the representation.

### 13. LinearMap implements Eq for non-equivalence value types

`src/linear_map.rs:634`.

Expected: `Eq` is implemented only when values also implement `Eq`. Observed: its bound is merely `V: PartialEq`.

Evidence: `LinearMap<u8,f32,1>` satisfies a generic `T: Eq` bound, yet a map containing `NaN` compares unequal to itself. Generic callers receive an invalid equality guarantee.

require `V: Eq`; retain the broader bound on `PartialEq`.

## Low

### 14. VecView cannot be serialized

`src/ser.rs:52`.

Expected: capacity-erased vector views remain usable with the serde integration. Observed: the storage parameter has an implicit `Sized` bound.

Evidence: a compile probe accepts the owned vector but rejects its view with E0277. Callers using the documented capacity-erasure API must convert to a slice or retain the concrete capacity.

add `?Sized` to the serialization implementation’s storage bound.

### 15. Strict Clippy fails on the installed toolchain

Representative locations: `src/index_map.rs:178`, `src/string/mod.rs:353`, `src/de.rs:46`, `src/mpmc.rs:129`.

Expected: the strict lint gate completes. Observed: Rust/Clippy 1.99.1 reports 13 library errors and 17 errors including the library test target.

Evidence: the baseline gate failed on suggestions including `question_mark`, `ref_as_ptr`, redundant conversions, and test initialization. This is pre-existing and toolchain-specific; older toolchains were not checked.

address the diagnostics or deliberately configure applicable lints.

### 16. Capacity panics lack public conversion documentation

Representative locations: `src/string/mod.rs:831`, `src/deque.rs:890`, `src/index_map.rs:1385`, `src/vec/mod.rs:1428`.

Source inspection confirmed seven primary conversion/extension implementations without capacity-panic documentation. Callers using generic collection APIs must discover the failure behavior at runtime. The advisory pass additionally reported 32 missing `# Errors` headings, often where prose already describes the error.

document capacity panics and available fallible alternatives.

### 17. Public handles and iterators lack Debug

Representative locations: `src/index_map.rs:383`, `src/spsc.rs:737`, `src/spsc.rs:746`.

The advisory pass emitted 43 missing-`Debug` diagnostics. Confirmed public examples include map entries and SPSC endpoints, preventing ordinary diagnostic formatting of those values.

provide suitable conditional or structural `Debug` implementations.

### 18. Unsafe ownership and synchronization assumptions lack local explanations

Representative locations: `src/bytes.rs:9`, `src/spsc.rs:741`, `src/spsc.rs:750`.

The advisory pass emitted 113 unsafe-block and 19 unsafe-implementation comment diagnostics. Some existing legacy comments explain operations, but confirmed examples leave trait bounds, publication ordering, or initialization assumptions unstated. Future changes must reconstruct these invariants.

document the actual preconditions and why each relevant call or trait implementation satisfies them.

### 19. Lint suppressions lack structured reasons

Representative locations: `src/storage.rs:22`, `src/storage.rs:25`, `src/history_buf.rs:70`.

The advisory pass identified nine `allow` attributes and one `expect` without structured reasons. Some have surrounding explanations, but broad suppressions such as `unused` obscure which target configuration requires them.

record specific reasons and narrow suppressions where practical.

## Verification

Commands used `RUSTUP_AUTO_INSTALL=0` and offline dependency resolution. Define:

```sh
F=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes
```

From `/work/crate`:

- `cargo test --locked --features="$F"` — passed: 220 unit tests, 9 integration tests, 184 doctests.
- `cargo test --locked` — passed: 186 unit tests, 9 integration tests, 177 doctests.
- `cargo clippy --workspace --all-targets --features="$F" -- -D warnings` — failed as described above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features="$F"` — passed.
- `MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri test --locked --features="$F"` — passed: 218 unit tests, 6 integration tests, 184 doctests; 5 tests ignored.
- Project formatting — blocked: its configuration requires nightly rustfmt, which is absent. The explicit dated-nightly formatting command confirmed the missing component.

The [tooling] gate was executed with `--audit` and the feature list above. Its advisory pass produced 299 library warnings. The heuristic scan produced five TODO/FIXME candidates; none independently warranted another finding.

From `/work/tmp/review-probes`:

- `cargo test --test contracts` — 13 failed, 2 passed.
- `cargo test --test unwind` — all 3 regression tests failed.
- `cargo test --test large_map` — failed.
- `cargo test --test peek` — passed natively.
- `cargo +nightly-2026-10-01 miri test --test peek` — failed with the reported data race.
- `cargo check --example serialize_vec_view` — failed with E0277.
- Initial unfiltered model testing aborted in deque insertion. Subsequently, `cargo test --test models -- --skip deque_wrap_and_contiguity` passed all 9 remaining model tests.

The separate compile-fail runner was attempted in a scratch copy; offline resolution blocked it because `trybuild` was not prefetched.

Total: 19 findings. Coverage was targeted rather than exhaustive; cross-target checks, ThreadSanitizer instrumentation, and a feature powerset were not run. Only the aarch64 target was installed, and cargo-hack was unavailable. No security assessment was performed. Last `/work/clock` reading: 797.3 active seconds, or 13.285833333333333 of 45 minutes used.