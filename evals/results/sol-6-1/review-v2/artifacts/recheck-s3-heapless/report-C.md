The existing tests pass, but added regression probes confirmed ordinary-use failures and broken recovery after panics. I found 13 issues: 5 Blocker, 7 Major and 1 Minor. No release files were edited; all added tests and compile probes are in `/work/tmp/probe`.

## Blocker

### 1. Full-deque normalization leaves an invalid write index

Location: `crate/src/deque.rs:489`, used at `crate/src/deque.rs:654`.

Expected: `make_contiguous` preserves the deque’s ability to accept elements after removal.

Observed: For a full wrapped deque, one branch sets `back = len = capacity`, outside the buffer. Filling capacity four, removing two front elements, appending two, calling `make_contiguous`, removing the front and appending again reaches unchecked index four.

Evidence: Executed `deque_can_push_after_full_make_contiguous` under Miri; it aborted on the invalid `get_unchecked_mut` index. Existing tests inspect normalized contents but omit subsequent insertion.

Consequence: Ordinary queue reuse reaches an invalid unchecked write.

Fix: Normalize `back` into the buffer’s index range, including when the deque is full. Add post-normalization insertion tests across wrap positions.

### 2. Popping a mutated list entry removes it twice

Location: `crate/src/sorted_linked_list.rs:713`, `crate/src/sorted_linked_list.rs:760`.

Expected: Mutating a `find_mut` result and calling its consuming `pop` returns that element once.

Observed: `pop` removes the element, then the guard’s destructor sees `maybe_changed` and calls `pop_internal` again. With a single element, the second removal indexes the empty-list sentinel.

Evidence: Executed `sorted_list_pop_mutated_entry` under Miri; it aborted on invalid unchecked indexing. Existing tests cover mutation followed by `finish`, and popping an unmodified guard.

Consequence: Combining two supported operations can abort or invalidate list ownership.

Fix: Disable destructor reinsertion before consuming the entry. Test mutated head and non-head removal.

### 3. Valid writes through `BufMut` panic at exact capacity

Location: `crate/src/bytes.rs:19`.

Expected: A write of exactly `remaining_mut()` bytes succeeds.

Observed: `advance_mut` rejects `pos == capacity` because it uses `>=`.

Evidence: Executed tests for `Vec<u8, 3>::put_slice(b"abc")`, the equivalent `VecView` operation and `Vec<u8, 1>::put_u8(7)`; all panic with “Advance out of range.”

Consequence: Callers cannot use the final advertised byte through this integration.

Fix: Permit equality, retaining rejection of advances beyond remaining capacity. Add exact-fill tests.

### 4. Destructor panics cause duplicate element destruction

Locations: `crate/src/deque.rs:237`, `crate/src/history_buf.rs:318`, `crate/src/vec/mod.rs:1527`.

Expected: After a destructor panics, unwinding must not destroy already-destroyed or already-yielded elements again.

Observed: These paths update ownership metadata after dropping elements. A panic bypasses that update.

Evidence: Executed drop-counter tests with one destructor that panics once:

- `Deque::clear` followed by container destruction drops both elements twice.
- `HistoryBuf::clear` does the same.
- Dropping a partially consumed vector iterator destroys an element still owned by its caller.

The counter tests use safe Rust and demonstrate duplicate destruction; implications for resource-owning elements follow from source inspection.

Consequence: Panic recovery can repeat resource cleanup or destroy a value whose ownership has already transferred.

Fix: Establish unwind-safe ownership state before invoking destructors, with guards where needed. Add regression tests for destructor failure.

### 5. A panicking retention predicate leaves stale map indices

Location: `crate/src/index_map.rs:307`; subsequent unchecked lookup at `crate/src/index_map.rs:188`.

Expected: A predicate panic may interrupt retention, but the map must preserve valid internal indexing.

Observed: The vector compacts entries during unwinding, while rebuilding the hash indices occurs only after normal return.

Evidence: Executed `index_map_retain_keeps_lookup_consistent_after_unwind`: insert keys `0, 1, 2`, discard `0`, panic while visiting `1`, catch the panic. Iteration retains `(1, 11)`, but `get(&1)` returns `None`. Source inspection also shows stale positions can exceed the shortened entries vector before unchecked lookup.

Consequence: A recovered callback panic leaves incorrect lookups and invalid internal indexing.

Fix: Rebuild indices using an unwind guard. Apply the same consistency discipline to other operations that modify entries before updating indices.

## Major

### 6. Truncation makes retained keys unreachable

Location: `crate/src/index_map.rs:1268`.

Expected: `truncate(2)` preserves lookup of the first two entries.

Observed: Removed entries’ hash slots become empty without repairing probe chains.

Evidence: An executed small-key regression found that inserting integer keys `0, 1, 5` into `FnvIndexMap<_, _, 4>`, then truncating to two entries, makes `get(&0)` return `None` although iteration still contains it.

Consequence: Ordinary truncation makes retained data inaccessible through key lookup.

Fix: Rebuild the index table or repair probe chains during removal. Test collisions and displaced entries.

### 7. Collecting a `LinearMap` preserves duplicate keys

Location: `crate/src/linear_map.rs:534`.

Expected: Collection constructs a map with unique keys, consistent with `insert` replacement semantics.

Observed: `FromIterator` appends directly to the backing vector.

Evidence: Executed collection of `[("a", 1), ("a", 2)]`; the resulting map has length two instead of one. Source inspection shows lookup returns the first value.

Consequence: Callers get duplicate entries, stale values and capacity exhaustion based on input pairs rather than unique keys.

Fix: Build through `insert`. Add duplicate-key collection tests, including repeated keys at capacity.

### 8. Accepted map capacities exceed the index representation

Locations: `crate/src/index_map.rs:115`, `crate/src/index_map.rs:732`, `crate/src/index_map.rs:1350`.

Expected: Every successfully inserted entry is retrievable within the declared capacity.

Observed: Entry positions decode through `u16`, while constructors accept larger power-of-two capacities.

Evidence: Executed a capacity-131,072 map test on a thread with sufficient stack. All 65,537 insertions succeed; length is 65,537, but lookup of key `65536` returns `None`.

Consequence: Larger legitimate maps silently lose lookup correctness. `IndexSet` inherits the representation limit.

Fix: Enforce a representable capacity in both constructors or widen the position encoding, including its nonzero sentinel constraints.

### 9. `resize_default` clones one default value

Location: `crate/src/vec/mod.rs:769`.

Expected: As documented, each added slot is initialized with `Default::default()`.

Observed: The method calls `Default` once and clones that value. It also calls `Default` when shrinking.

Evidence: An executed test whose defaults produce successive identifiers expected `[0, 1, 2]` and observed `[0, 0, 0]`.

Consequence: Stateful defaults behave incorrectly, shrinking can invoke unnecessary failing initialization, and callers face an unnecessary `Clone` bound.

Fix: Construct each additional element independently and truncate directly when shrinking.

### 10. Nul-terminated constructor accepts unterminated input

Location: `crate/src/c_string.rs:80`.

Expected: The documented constructor rejects input without a terminating nul byte.

Observed: It delegates to the more permissive append operation, which supplies a terminator.

Evidence: Executed tests show both `b""` and `b"abc"` return `Ok`.

Consequence: Callers relying on this constructor for validation accept malformed input contrary to its contract.

Fix: Validate termination separately and represent the missing-terminator error explicitly. Preserve permissive append behavior separately.

### 11. `HistoryBuf::new_with` accepts an unusable zero capacity

Location: `crate/src/history_buf.rs:276`.

Expected: Construction enforces the nonzero-capacity invariant already enforced by `new`.

Observed: `new_with` permits capacity zero and marks the buffer full.

Evidence: Executed `HistoryBuf::<u8, 0>::new_with(42).oldest()`; it panics on index zero of an empty buffer. Source inspection shows `recent_index` also subtracts one from zero capacity.

Consequence: A successfully constructed value has panicking ordinary accessors.

Fix: Apply the same compile-time capacity assertion as `new`.

### 12. `VecView` loses Serde serialization support

Location: `crate/src/ser.rs:52`.

Expected: Erasing vector capacity into its public view preserves serialization support.

Observed: The serialization implementation implicitly requires sized storage.

Evidence: Executed `cargo check --example serde_view`: passing an owned vector to `T: Serialize + ?Sized` compiles, but passing its `&VecView<u8>` fails with E0277.

Consequence: Callers using views must add serialization adapters or expose storage details.

Fix: Add `?Sized` to the storage bound and add compile coverage for serialization through views.

## Minor

### 13. Current toolchains fail strict linting and the docs.rs configuration

Locations include `crate/src/lib.rs:133`, `crate/src/index_map.rs:178`, `crate/src/string/mod.rs:353`, `crate/src/de.rs:46`.

Expected: The configured checks complete on supported toolchains.

Observed:

- Rust 1.99.1 strict Clippy fails, reporting 13 library errors and 17 library-test errors, including `question_mark`, `ref_as_ptr`, unnecessary conversions and needless returns.
- On installed `nightly-2026-10-01`, enabling `docsrs` fails with E0557 because `doc_auto_cfg` was removed and merged into `doc_cfg`.

Evidence: Both failures were executed against the unchanged release. Ordinary documentation generation passes.

Consequence: Current lint CI and docs.rs-style documentation builds fail. These are release/toolchain compatibility failures, separate from the behavioral defects above.

Fix: Remove the obsolete feature gate and address the current lint diagnostics.

## Verification

Commands ran offline with `RUSTUP_AUTO_INSTALL=0`.

From `/work/crate`:

```sh
cargo test --locked --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes
```

Passed: 220 unit tests, 9 integration tests and 184 doctests.

```sh
cargo test --locked
```

Passed: 186 unit tests, 9 integration tests and 177 doctests.

The skill gate was executed:

```sh
bash /work/skill/scripts/check.sh --audit --features alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes /work/crate
```

Its underlying checks produced:

- `cargo clippy --workspace --all-targets --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes -- -D warnings`: failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes`: passed.
- `cargo test --workspace --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes`: passed.
- Formatting: blocked. Stable rustfmt cannot honor the nightly-only configuration; explicit `cargo +nightly-2026-10-01 fmt --all --check` reports that nightly rustfmt is not installed.

Additional executed checks:

```sh
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri test --locked --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes
```

Passed existing enabled tests; five tests were ignored. Miri warned that integer-to-pointer casts in the pool implementation can limit pointer-error detection. Leak checking was disabled.

```sh
RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo +nightly-2026-10-01 doc --locked --no-deps --document-private-items --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes
```

Failed with the removed-feature error described above.

`cargo check --locked --no-default-features` passed, as did separate checks adding each of `alloc`, `bytes`, `defmt`, `mpmc_large`, `portable-atomic-critical-section`, `serde` and `ufmt`.

From `/work/tmp/probe`:

- `cargo test --test contracts`: 2 passed, 5 failed.
- `cargo test --test defaults`: 2 failed.
- `cargo test --test unwind`: 4 failed.
- `cargo test --test large`: 1 failed.
- `cargo test --test state linear_map_collect_replaces_duplicate_keys`: failed.
- `cargo test --test state index_map_truncate_keeps_earlier_keys_findable`: failed.
- `cargo test --test state sorted_list_drops_owned_elements`: passed.
- `cargo +nightly-2026-10-01 miri test --test state deque_can_push_after_full_make_contiguous`: aborted on invalid indexing.
- `cargo +nightly-2026-10-01 miri test --test state sorted_list_pop_mutated_entry`: aborted on invalid indexing.
- `cargo check --example serde_view`: failed with E0277.
- `cargo test --test models`: all 7 comparison tests passed. Coverage includes Unicode edits, vector drain/retain boundaries, set algebra, queue counter wraparound, history ordering, map operations and deque operations excluding the separately reproduced full-normalization defect.

The advisory scan reported 132 unsafe-comment diagnostics, 43 missing-`Debug` diagnostics, 32 missing-`Errors` headings, 3 missing-`Panics` headings and 10 suppression-reason diagnostics. These are diagnostic counts, not additional proven behavioral defects; some have existing prose explanations. The heuristic scan produced five TODO/FIXME candidates without a confirmed caller-impacting finding.

Total: 13 findings. Coverage was limited to aarch64 Linux and the configurations above; cross-target checks, the separate compile-fail suite, sanitizers and the full feature powerset were not run. No dedicated security testing was performed. Last clock reading: 654.5/2700 active seconds, or 10.906/45 minutes.