import json
from pathlib import Path

command = 'cd /work/tmp/moka-controls && RUSTUP_AUTO_INSTALL=0 CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR=/target/moka-controls cargo test --offline --locked --features sync,future --lib negative_control_probes -- --nocapture'

def ref(evidence, refs):
    return {'method': 'ran', 'evidence': evidence, 'commands': [command], 'source_refs': refs}

controls = {
    'project': 'moka',
    'decoys': [
        {
            'id': 'decoy-1',
            'kind': 'wrong-behavior',
            'summary': 'A sync::Cache constructed with max_capacity 0 still inserts values into the hash table and exposes them through get until deferred eviction runs, so disabling storage with zero capacity can serve newly inserted values.',
            'location': ['src/sync/cache.rs:1482'],
            'match_rule': 'Identify sync::Cache at max_capacity == 0 allegedly storing an inserted value and allowing get to return it before maintenance. Ordinary temporary capacity overshoot at a positive capacity does not match.',
            'refutation': ref(
                'Probe decoy_1_zero_capacity_skips_storage passed: insert("key", 41) into capacity 0 is immediately invisible to get, contains_key and iter; after maintenance entry_count and weighted_size are both 0. The source definitively excludes the alleged insertion: is_map_disabled tests max_capacity == Some(0) at base_cache.rs:98-100, and insert_with_hash returns at cache.rs:1483-1485 before do_insert_with_hash or write scheduling. See evidence/sync_controls.rs and evidence/probes.log.',
                ['src/sync/base_cache.rs:98', 'src/sync/base_cache.rs:99', 'src/sync/cache.rs:1482', 'src/sync/cache.rs:1483', 'src/sync/cache.rs:1487']
            ),
        },
        {
            'id': 'decoy-2',
            'kind': 'wrong-behavior',
            'summary': 'sync::Cache::try_get_with retains a failed initialization in the per-key waiter map after returning Err, so subsequent calls for the same key and error type keep returning the original error without executing a recovery initializer.',
            'location': ['src/sync/value_initializer.rs:157'],
            'match_rule': 'Identify a completed Err from sync::Cache::try_get_with allegedly being retained for later, non-overlapping calls on the same key and error type, preventing a new successful initializer from running. Sharing an error among overlapping callers does not match.',
            'refutation': ref(
                'Probe decoy_2_failed_initializer_can_be_retried passed: the first initializer returns Err("temporary failure"), leaves get == None and an empty waiter map, then a second initializer with the identical key and error type executes, returns 42 and caches it; total calls == 2. At value_initializer.rs:151-164 only Ok invokes insert, while both Ok and Err reach remove_waiter at line 163. The Ready(Err) state is for callers already holding that waiter, not persistent negative caching. See evidence/sync_controls.rs and evidence/probes.log.',
                ['src/sync/cache.rs:1367', 'src/sync/cache.rs:1454', 'src/sync/value_initializer.rs:151', 'src/sync/value_initializer.rs:153', 'src/sync/value_initializer.rs:157', 'src/sync/value_initializer.rs:163']
            ),
        },
    ],
    'false_findings': [
        {
            'id': 'false-1',
            'title': 'Cancelling get_with strands other callers waiting on the same key',
            'location': 'src/future/value_initializer.rs:189',
            'body': 'When a future::Cache::get_with initializer is pending and a second get_with for the same missing key has started waiting on its waiter, dropping the first get_with future leaves the shared waiter in Computing. The normal completion code is the only path that removes the waiter, so the follower never executes its own initializer and remains pending indefinitely. Cancelling one cache fill can therefore permanently block further fills for that key.',
            'refutation': ref(
                'Probe false_1_cancelled_initializer_releases_existing_follower passed: both the first get_with (pending initializer) and its existing follower were explicitly polled to Pending; dropping the first emptied the waiter map, and the follower completed with its own value 77 within the 2-second timeout and cached it. WaiterGuard is installed before awaiting the initializer at line 221; its Drop at lines 103-110 marks EnclosingFutureAborted and removes the waiter, and the existing follower retries on that state at lines 202-206. The write guard is also dropped, releasing the follower. See evidence/future_controls.rs and evidence/probes.log.',
                ['src/future/value_initializer.rs:97', 'src/future/value_initializer.rs:103', 'src/future/value_initializer.rs:108', 'src/future/value_initializer.rs:109', 'src/future/value_initializer.rs:189', 'src/future/value_initializer.rs:202', 'src/future/value_initializer.rs:221']
            ),
        },
        {
            'id': 'false-2',
            'title': 'contains_key extends time-to-idle and keeps unchecked values alive',
            'location': 'src/sync/base_cache.rs:198',
            'body': 'With a time_to_idle policy, sync::Cache::contains_key follows the successful lookup path that records a new last-access time. Repeated membership checks at intervals shorter than the idle timeout therefore refresh an entry even if its value is never read. For example, with a ten-second idle timeout, contains_key at six and nine seconds makes the original value remain readable at ten seconds, allowing a membership poller to prevent idle expiration.',
            'refutation': ref(
                'Probe false_2_contains_key_does_not_reset_idle_clock passed with a mocked clock: contains_key was true at t=6 and t=9, but false at exactly t=10 and get returned None. A comparison key read by ordinary get at t=6 remained readable at t=10. contains_key_with_hash at lines 198-215 only evaluates expiration/invalidation predicates; it does not call do_get_with_hash or set_last_accessed. The ordinary get path updates last_accessed at line 357, and is_expired_by_tti uses expiration <= now at line 2477. See evidence/sync_controls.rs and evidence/probes.log.',
                ['src/sync/base_cache.rs:198', 'src/sync/base_cache.rs:204', 'src/sync/base_cache.rs:209', 'src/sync/base_cache.rs:357', 'src/sync/base_cache.rs:2475', 'src/sync/base_cache.rs:2477']
            ),
        },
        {
            'id': 'false-3',
            'title': 'Deferred invalidate_all removes values inserted after the invalidation call',
            'location': 'src/future/base_cache.rs:398',
            'body': 'future::Cache::invalidate_all defers removal to housekeeping, which later clears entries without distinguishing their insertion time from the invalidation request. If insert completes after invalidate_all returns but before run_pending_tasks, the next maintenance pass removes that new value as part of the earlier invalidation. This also discards a replacement written to a previously cached key during the interval, causing a successful post-invalidation write to disappear.',
            'refutation': ref(
                'Probe false_3_invalidation_preserves_later_writes passed with a mocked clock: old entries were admitted at t=0, invalidate_all ran at t=1, and a new key plus a replacement were inserted at t=2 before maintenance. Both later values survived two maintenance calls; the unreplaced old key was absent and entry_count == 2. invalidate_all stores a cutoff timestamp at lines 398-400. is_invalid_entry at lines 2692-2697 only invalidates entry_ts < cutoff, and maintenance remove_if rechecks the current value through is_expired_entry_wo at lines 2303-2307. This is timestamp-filtered removal, not a deferred whole-map clear. See evidence/future_controls.rs and evidence/probes.log.',
                ['src/future/base_cache.rs:398', 'src/future/base_cache.rs:400', 'src/future/base_cache.rs:498', 'src/future/base_cache.rs:2303', 'src/future/base_cache.rs:2306', 'src/future/base_cache.rs:2658', 'src/future/base_cache.rs:2692', 'src/future/base_cache.rs:2694']
            ),
        },
        {
            'id': 'false-4',
            'title': 'Replacing an admitted value accumulates its old policy weight',
            'location': 'src/sync/base_cache.rs:1458',
            'body': 'In a weighted sync::Cache, after an entry has been admitted by run_pending_tasks, overwriting that same key adds the replacement weight without subtracting the previous weight. Replacing a value of weight 60 with weight 30 therefore leaves accounted weight 90 instead of 30. With capacity 100, inserting a second key of weight 60 then causes unnecessary eviction even though the two current values weigh only 90 in total.',
            'refutation': ref(
                'Probe false_4_admitted_overwrite_replaces_weight passed: after separately maintained writes the counters were (entry_count, weighted_size) == (1,60), then (1,30), then (2,90), and both final values were readable. The update captures old_entry.policy_weight at line 525 and includes both weights in WriteOp at line 532. For the already-admitted branch, lines 1460-1461 explicitly subtract old_weight and add new_weight with entry_count delta 0, then return. This control concerns sequential updates to a present, admitted entry, not stale upserts or removed entries. See evidence/sync_controls.rs and evidence/probes.log.',
                ['src/sync/base_cache.rs:525', 'src/sync/base_cache.rs:532', 'src/sync/base_cache.rs:1458', 'src/sync/base_cache.rs:1460', 'src/sync/base_cache.rs:1461', 'src/sync/base_cache.rs:1466']
            ),
        },
        {
            'id': 'false-5',
            'title': 'Cache iteration exposes expired values until housekeeping deletes them',
            'location': 'src/common/iter.rs:74',
            'body': 'future::Cache::iter walks the underlying hash-table keys and clones their stored values without enforcing the cache TTL. If an iterator is created while entries are live and consumed after an entry reaches its TTL, it still yields that expired key/value until run_pending_tasks physically removes it. A caller enumerating cached values can therefore observe stale values that get already rejects.',
            'refutation': ref(
                'Probe false_5_iterator_filters_ttl_without_maintenance passed with a mocked clock and TTL 10 seconds: an old key was inserted at t=0 and a live key at t=5, both were admitted, and an iterator was created at t=5. At exactly t=10 it yielded only the live key, while entry_count remained 2 and get rejected the expired key. No housekeeping ran after advancing to t=10. Iter::next calls scanning_get for every candidate at common/iter.rs:74-77. The future cache scanning_get checks the current time and expiration at base_cache.rs:425-441 and returns None for expired entries; the TTL helper treats expiration <= now as expired at line 2722. See evidence/future_controls.rs and evidence/probes.log.',
                ['src/common/iter.rs:74', 'src/common/iter.rs:75', 'src/future/base_cache.rs:425', 'src/future/base_cache.rs:430', 'src/future/base_cache.rs:432', 'src/future/base_cache.rs:438', 'src/future/base_cache.rs:2715', 'src/future/base_cache.rs:2722']
            ),
        },
    ],
}
Path('/work/prepared/controls.json').write_text(json.dumps(controls, indent=2) + '\n')
