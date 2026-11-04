# Audit report: moka 0.12.11 (`crate/src/`)

This replaces my earlier report. The only change is the Miri result, which has now finished.

## Summary

moka is a mature library. Its own test suite passes with `sync,future`, and every behavior probe of the normal API matched what a user should get, except one. The serious defect is a use-after-free in the shared timer wheel, which I triggered from safe public API and confirmed with AddressSanitizer. 

## High

**Timer wheel frees a node that the cache entry still points to**
`src/common/timer_wheel.rs:493-526` (`TimerEventsIter::next`). Both `sync::Cache` and `future::Cache` use this code.

when a popped timer node has no expiration time, the code skips the whole `if let Some(t) = expiration_time` block. The `Box` is then dropped at the end of the branch. Nothing calls `unset_timer_node_in_deq_nodes()`, so the entry's `DeqNodes` keeps a dangling timer pointer. When the entry's pending `ReadOp` or `WriteOp` is applied later, or the entry is removed, `deschedule` runs on freed memory.

An entry's expiration can be cleared while it is still scheduled. This happens when `Expiry::expire_after_read` or `expire_after_update` returns `None` on another thread while housekeeping is advancing the wheel. It also happens when the `ReadOp` is dropped or not recorded, a case the comment at `src/sync/base_cache.rs:333-345` says is fine.

Reproduction: `tmp/probe/src/main.rs` (a throwaway crate outside `crate/`) uses only the public API:
- Build a `sync::Cache` with `expire_after`. `expire_after_create` returns 300 ms for key "w" and 1.9 s for key "y". `expire_after_read` returns `None`.
- Add an eviction listener that blocks on w's `Expired` notification until a second thread has called `get(&"y")`.
- Insert both keys, sleep 1.3 s, then call `run_pending_tasks()` twice.

Under `-Zsanitizer=address` this gives "heap-use-after-free ... READ of size 1", freed by the timer iterator and then read by `deschedule`. In a release build it silently corrupts memory.

```rust
if let Some(node) = self.timer_wheel.pop_timer_node(self.level, i as usize) {
    if let Some(t) = node.element.entry_info().expiration_time() {
        if t <= self.current_time {
            node.element.unset_timer_node_in_deq_nodes();
            return Some(TimerEvent::Expired(node));
        }
    }
    let node_p = NonNull::new(Box::into_raw(node)).expect("Got a null ptr");
    // ... existing schedule_existing_node match, unchanged
}
```
Add the probe as a regression test using the mock clock, plus a variant where the cleared expiration comes from `expire_after_update`.

## Medium

**`is_old_value_replaced()` is false after a replacement**
`src/common/entry.rs:84-91` (docs). Behavior comes from `src/sync/cache.rs:1021` (`insert_with_hash_and_fun`) and the matching future path.

the docs say it "returns true if an old value existed in the cache and was replaced". After `entry(k).or_insert(5)`, calling `entry(k).or_insert_with_if(|| 7, |v| *v == 5)` returns an `Entry` with value 7 but `is_old_value_replaced() == false`. Only the compute and upsert paths set the flag.

**Unsafe code with no SAFETY justification, and overly broad `Send` impls**
76 `unsafe` blocks without a SAFETY comment, for example `src/common/deque.rs:56,128,139,169`, `src/common/concurrent/deques.rs:79-133` and `src/common/timer_wheel.rs:238,244,257`. Overly broad `Send` impls are `unsafe impl<K> Send` for `TimerWheel<K>` (`timer_wheel.rs:167`), `Deques<K>` (`deques.rs:23`) and `DeqNodes<K>` (`concurrent.rs:156`).

the deque and timer code hands raw `NonNull` pointers between `ValueEntry`, the deques and the timer wheel. The invariants that keep this sound, such as single ownership under the deques/timer lock and pointers being unset before a node is freed, are not written down anywhere. The High above is exactly a broken instance of that unwritten invariant. The three `Send` impls have no `K: Send` bound. They are sound today only because every constructor requires `K: Send + Sync`.

**Sync entry API docs link to the wrong (missing) example**
`src/sync/entry_selector.rs:201` and `:743`.

the docs for `sync::OwnedKeyEntrySelector` and `sync::RefKeyEntrySelector` say "See [`try_append_value_async.rs`]", but the link defined there is for `try_append_value_sync.rs`. The link is broken on docs.rs and names the async example in sync docs. This is one of the four `cargo doc -D warnings` errors.

## Low

**Other rustdoc warnings in private docs**
`src/common/concurrent/arc.rs:83` (unresolved `ptr::eq`) and `:99` (bare URL).

with `--document-private-items -D warnings` the doc build fails. Only the entry-selector link above affects public docs.

**Expiry trait docs name the wrong parameter and type**
`src/policy.rs:149-150`.

the docs say the defaults return "`current_duration: Option<Instant>`". The parameter is `duration_until_expiry: Option<Duration>`.

**Documentation and Debug hygiene**
across the crate, according to the advisory lint pass.

- Missing crate-level and other docs: 9 instances.
- Functions returning `Result` with no `# Errors` section: 20, for example `try_get_with` and `invalidate_entries_if`.
- Public types with no `Debug`: 8, for example `common::iter::Iter` at `src/common/iter.rs:22`.
- `# Parameters` lists in `src/policy.rs:159,190,236` work like parameter tables.

**Lint hygiene**
across the crate.

- `assert!` without a message: 29, for example `assert!(num_segments != 0)` at `src/sync/builder.rs:111`.
- `unwrap()` outside tests: 13, for example `deque.peek_back().unwrap()` at `timer_wheel.rs:328` and `run_after.instant().unwrap()` at `housekeeper.rs:102`.
- Many `#[allow]` attributes with no reason.

## Low

**Bare TODOs and a missing lint baseline**
There are about 40 TODOs without an owner or issue number, for example `src/common/concurrent/deques.rs:206`, which says "Add tests and run Miri with them". `[lints]` only configures `unexpected_cfgs`. Recommend adding clippy lints for undocumented unsafe, `unwrap_used` and `missing_errors_doc`.

**Miri does not cover the code path with the bug**
The existing deque, timer-wheel, frequency-sketch and `MiniArc` unit tests are clean under Miri. None of them exercises a timer node whose expiration was cleared, so Miri cannot catch the High. Recommend adding a mock-clock unit test for that case and running it under Miri in CI.
