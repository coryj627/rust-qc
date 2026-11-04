import json
from pathlib import Path

command = ('cd /work/tmp/heapless-controls && RUSTUP_AUTO_INSTALL=0 '
           'CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR=/target/heapless-controls '
           'cargo test --offline --locked '
           '--features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes '
           '--test control_probes -- --nocapture --test-threads=1 '
           '> /work/prepared/evidence/probes.log 2>&1')

def refutation(evidence, refs, test_name, log_line):
    return {
        'method': 'ran',
        'evidence': evidence + ' Public-API regression: ' + test_name +
            ' in /work/prepared/evidence/control_probes.rs; passed in the seven-test run. ' +
            f'Observed result: /work/prepared/evidence/probes.log:{log_line}.',
        'commands': [command],
        'source_refs': refs,
    }

controls = {
    'project': 'heapless',
    'decoys': [
        {
            'id': 'decoy-1',
            'kind': 'wrong-behavior',
            'summary': 'Vec::extend_from_slice partially appends the fitting prefix before returning CapacityError. With Vec<u8, 4> containing [10, 20, 30], appending [40, 50] returns an error but leaves [10, 20, 30, 40], so retrying after the error observes a modified vector.',
            'location': ['src/vec/mod.rs:629', 'src/vec/mod.rs:641'],
            'match_rule': 'The report must identify partial mutation of Vec contents or length on the capacity-error path of extend_from_slice when only a prefix of the supplied slice fits. Claims about Clone panics or a different extension API do not match.',
            'refutation': refutation(
                'The capacity comparison at src/vec/mod.rs:641 precedes the entire cloning loop. The insufficient-capacity branch returns Err(CapacityError) at line 643 without modifying the buffer or length; writes and length increments occur only in the else branch at lines 645-649. The probe received Err for [40,50] and verified the vector remained [10,20,30] with length 3. A subsequent fitting append of [60] produced [10,20,30,60].',
                ['src/vec/mod.rs:641', 'src/vec/mod.rs:643', 'src/vec/mod.rs:645', 'src/vec/mod.rs:649'],
                'decoy_1_slice_capacity_error_preserves_vector', 6),
        },
        {
            'id': 'decoy-2',
            'kind': 'wrong-behavior',
            'summary': 'LinearMap::insert rejects replacement of an existing key when the map is full because it tries to append a new pair before finding the existing entry. A full LinearMap<u8, u8, 2> with {1:10, 2:20} returns Err((1,99)) for insert(1,99) and leaves key 1 mapped to 10 instead of replacing it and returning the old value.',
            'location': ['src/linear_map.rs:279', 'src/linear_map.rs:285'],
            'match_rule': 'The report must identify rejection of an existing-key value replacement specifically because a LinearMap is at capacity. Failure to insert a genuinely new key into a full map, or claims about IndexMap, do not match.',
            'refutation': refutation(
                'LinearMap::insert searches existing entries first at line 280. On a match it swaps the stored value and returns Ok(Some(old_value)) at lines 281-282; the capacity-limited buffer.push at line 285 is reached only for an absent key. The full-capacity probe returned Ok(Some(10)) for insert(1,99), verified key 1 held 99, key 2 still held 20, and length remained 2. Inserting the new key 3 separately returned Err((3,30)).',
                ['src/linear_map.rs:258', 'src/linear_map.rs:280', 'src/linear_map.rs:281', 'src/linear_map.rs:282', 'src/linear_map.rs:285'],
                'decoy_2_full_linear_map_replaces_existing_key', 8),
        },
    ],
    'false_findings': [
        {
            'id': 'false-1',
            'title': 'String::truncate panics for lengths beyond the current string',
            'location': 'src/string/mod.rs:575',
            'body': 'String::truncate applies its character-boundary assertion to a requested length beyond the current string, although the documented contract requires such calls to have no effect. For example, String::<8>::try_from("aéz") contains four bytes, but truncate(5) fails the boundary check and panics. Code that truncates to a maximum permitted length therefore panics on an already shorter string instead of leaving it unchanged.',
            'refutation': refutation(
                'The outer if new_len <= self.len() at line 576 guards both the boundary assertion and the underlying vector truncation. Larger lengths skip the body entirely, matching the contract at lines 548-551. The probe called truncate(5) and truncate(usize::MAX) on the four-byte string "aéz"; both returned normally with identical contents and length 4.',
                ['src/string/mod.rs:548', 'src/string/mod.rs:550', 'src/string/mod.rs:575', 'src/string/mod.rs:576', 'src/string/mod.rs:577', 'src/string/mod.rs:578'],
                'false_1_truncate_past_string_length_is_noop', 10),
        },
        {
            'id': 'false-2',
            'title': 'CString rejects an exact-fit append by counting two terminators',
            'location': 'src/c_string.rs:126',
            'body': 'The capacity calculation used by CString::extend_from_bytes counts both the existing trailing nul and the appended slice\'s trailing nul. Consequently, a CString<5> containing b"ab\\0" rejects extend_from_bytes(b"cd\\0") with a capacity error, even though the result b"abcd\\0" needs exactly five bytes. Callers appending already terminated chunks lose one byte of usable capacity.',
            'refutation': refutation(
                'capacity_with_bytes explicitly handles a nontrivial nul-terminated slice at lines 130-134 by returning self.inner.len() + bytes.len() - 1. Thus the example computes 3 + 3 - 1 = 5, and the capacity > N rejection at line 169 does not apply. extend_from_bytes_unchecked removes the old terminator at line 227 before appending the supplied bytes at line 229. The probe successfully appended b"cd\\0" to CString<5> containing b"ab\\0", verified b"abcd\\0" and the text "abcd", and verified appending just b"\\0" was a no-op even at capacity.',
                ['src/c_string.rs:126', 'src/c_string.rs:130', 'src/c_string.rs:134', 'src/c_string.rs:169', 'src/c_string.rs:174', 'src/c_string.rs:177', 'src/c_string.rs:227', 'src/c_string.rs:229'],
                'false_2_cstring_terminated_append_uses_shared_terminator_capacity', 12),
        },
        {
            'id': 'false-3',
            'title': 'Mutating a heap root through peek_mut leaves its priority stale',
            'location': 'src/binary_heap.rs:627',
            'body': 'PeekMutInner::deref_mut returns the root without marking its sift flag, so dropping a guard after changing the value skips heap repair. With a Max heap built from [9,7,8,1], assigning 0 through peek_mut and letting the guard leave scope leaves peek() returning 0 rather than 8. Subsequent removals therefore violate the heap\'s priority ordering after an ordinary public-API mutation.',
            'refutation': refutation(
                'peek_mut constructs every nonempty guard with sift: true at lines 360-363, so deref_mut need not set it. Normal Drop checks that flag and calls sift_down_to_bottom(0) at lines 600-603. The repair function moves down through the selected children and then sifts up at lines 466-483. The probe changed the maximum 9 to 0, dropped the guard normally, observed peek() == Some(&8), and popped exactly [8,7,1,0].',
                ['src/binary_heap.rs:356', 'src/binary_heap.rs:360', 'src/binary_heap.rs:362', 'src/binary_heap.rs:600', 'src/binary_heap.rs:601', 'src/binary_heap.rs:602', 'src/binary_heap.rs:466', 'src/binary_heap.rs:483', 'src/binary_heap.rs:627'],
                'false_3_peek_mut_drop_restores_max_heap_order', 14),
        },
        {
            'id': 'false-4',
            'title': 'HistoryBuf::recent reports an empty buffer at exact wrap boundaries',
            'location': 'src/history_buf.rs:429',
            'body': 'HistoryBuf::recent_index treats write_at == 0 as the empty-buffer state without distinguishing a filled buffer whose write cursor just wrapped. For HistoryBuf<u8,4>, writing 1,2,3,4 therefore makes recent() return None instead of Some(&4), and the same loss of the latest-sample result occurs after every complete cycle. Applications reading the most recent sample receive a spurious empty result whenever the write count is a multiple of capacity.',
            'refutation': refutation(
                'write sets write_at to 0 and filled to true together at lines 381-383. recent_index explicitly distinguishes the states: when write_at == 0 and filled is true, it returns Some(self.capacity() - 1) at lines 430-432; None is confined to the unfilled branch at line 434. recent maps that index to the stored value at lines 412-414. The probe observed Some(&4) and Some(&8) at the two exact wraps with index Some(3). It also verified that capacity 1 returns index Some(0) and the latest value after every write, while a newly empty buffer returns None.',
                ['src/history_buf.rs:373', 'src/history_buf.rs:381', 'src/history_buf.rs:382', 'src/history_buf.rs:383', 'src/history_buf.rs:412', 'src/history_buf.rs:429', 'src/history_buf.rs:430', 'src/history_buf.rs:431', 'src/history_buf.rs:432', 'src/history_buf.rs:434'],
                'false_4_history_recent_survives_exact_wrap', 16),
        },
        {
            'id': 'false-5',
            'title': 'Dropping a partially consumed Vec drain loses the untouched suffix',
            'location': 'src/vec/drain.rs:87',
            'body': 'Vec::drain shortens the source vector to the start of the range, but restoration of the untouched suffix occurs only when the iterator is exhausted. If [10,20,30,40,50] is drained over 1..4, only 20 is consumed, and the iterator is then dropped normally, the source remains [10] instead of [10,50]. Breaking early from a draining loop therefore deletes elements outside the requested range.',
            'refutation': refutation(
                'drain records the untouched tail start and length at src/vec/mod.rs:485-486. Drain::drop defines a DropGuard whose destructor copies that tail to the retained prefix and restores the length at src/vec/drain.rs:93-105. For these nonzero-sized elements, the guard is instantiated at line 131 before the remaining drained elements are dropped at line 152, so restoration runs on normal early drop independently of iterator exhaustion. The probe consumed only 20, confirmed the drain still held [30,40], dropped it, and observed the correct vector [10,50] with length 2. A following push produced [10,50,60].',
                ['src/vec/mod.rs:430', 'src/vec/mod.rs:481', 'src/vec/mod.rs:485', 'src/vec/mod.rs:486', 'src/vec/drain.rs:86', 'src/vec/drain.rs:91', 'src/vec/drain.rs:93', 'src/vec/drain.rs:103', 'src/vec/drain.rs:105', 'src/vec/drain.rs:131', 'src/vec/drain.rs:152'],
                'false_5_dropping_partial_drain_restores_untouched_suffix', 18),
        },
    ],
}
Path('/work/prepared/controls.json').write_text(json.dumps(controls, indent=2, ensure_ascii=False)+'\n')
