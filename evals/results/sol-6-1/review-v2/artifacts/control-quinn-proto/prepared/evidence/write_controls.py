import json
from pathlib import Path

command = 'cd /work/tmp/control-probe && RUSTUP_AUTO_INSTALL=0 cargo test --locked -p quinn-proto negative_control_ -- --nocapture'

def refutation(evidence, refs):
    return {
        'method': 'ran',
        'evidence': evidence + ' All seven named probes passed in the single filtered run recorded in /work/prepared/evidence/probes.log; probe modules were added only to the scratch workspace.',
        'commands': ['python3 /work/prepared/evidence/prepare_probes.py', command],
        'source_refs': refs,
    }

controls = {
    'project': 'quinn-proto',
    'decoys': [
        {
            'id': 'decoy-1',
            'kind': 'wrong-behavior',
            'summary': 'Datagrams::send(data, false) evicts older queued datagrams and returns success when the new payload exceeds the remaining send-buffer space. With an 8-byte send buffer, enqueueing two 4-byte datagrams followed by a 1-byte datagram silently discards an earlier payload instead of returning SendDatagramError::Blocked.',
            'location': ['quinn-proto/src/connection/datagrams.rs:49'],
            'match_rule': 'Identify loss of previously queued datagrams or successful over-capacity enqueueing specifically in Datagrams::send with drop=false, rather than the documented drop=true eviction policy; the expected behavior is Blocked with the rejected Bytes and preservation of the queue.',
            'refutation': refutation(
                'The eviction loop is inside if drop (line 38). The mutually exclusive drop=false branch checks outgoing_total + data.len() against capacity and returns Blocked(data) at line 53 before either queue insertion or byte-count increment. Probe negative_control_decoy_1_datagram_backpressure_preserves_queue in evidence/probes/public_api.rs uses connected in-memory endpoints and the public datagram API: two 4-byte payloads exactly fill an 8-byte buffer, a further byte returns Blocked containing that same byte, buffer space stays zero, and both original payloads arrive in order. After draining, space is 8 and the rejected payload can be retried and received.',
                ['quinn-proto/src/connection/datagrams.rs:21', 'quinn-proto/src/connection/datagrams.rs:38', 'quinn-proto/src/connection/datagrams.rs:49', 'quinn-proto/src/connection/datagrams.rs:53', 'quinn-proto/src/connection/datagrams.rs:55']),
        },
        {
            'id': 'decoy-2',
            'kind': 'wrong-behavior',
            'summary': 'SendStream::write_chunks counts a partially written Bytes element as a fully consumed chunk when stream flow-control credit ends inside it. An application advancing its input by Written::chunks therefore skips the unwritten suffix; for [ab, cdef] with four bytes of credit, it reports two consumed chunks instead of leaving ef for retry.',
            'location': ['quinn-proto/src/connection/streams/mod.rs:233', 'quinn-proto/src/connection/streams/send.rs:184'],
            'match_rule': 'Identify erroneous full-chunk accounting or loss of the remaining Bytes suffix specifically for a partial write_chunks element at a flow-control boundary, including the claim that Written::chunks includes that partial element.',
            'refutation': refutation(
                'BytesArray::pop_chunk increments consumed and chunks_consumed only when chunk.len() <= limit (lines 175-178). The partial branch split_to(limit) returns a prefix without incrementing either counter (lines 183-185), leaving the suffix in the caller\'s Bytes. Send::write adds only the returned chunks_consumed. This matches the public contract at streams/mod.rs:229-232. Probe negative_control_decoy_2_partial_chunk_preserves_suffix in evidence/probes/public_api.rs exercises the public API with four bytes of peer stream credit and [empty, ab, empty, cdef]. It returns Written { bytes: 4, chunks: 3 }: the three complete elements are consumed, the partial fourth element retains ef, and the receiver gets abcd. After reading restores credit, retrying that suffix returns { bytes: 2, chunks: 1 } and delivers ef.',
                ['quinn-proto/src/connection/streams/mod.rs:229', 'quinn-proto/src/connection/streams/send.rs:71', 'quinn-proto/src/connection/streams/send.rs:175', 'quinn-proto/src/connection/streams/send.rs:183', 'quinn-proto/src/connection/streams/send.rs:251']),
        },
    ],
    'false_findings': [
        {
            'id': 'false-1',
            'title': 'VarInt conversion narrows u128 before validating its range',
            'location': 'quinn-proto/src/varint.rs:106',
            'body': 'The TryFrom<u128> implementation narrows the input to u64 before applying the 62-bit VarInt bound, so high bits are discarded before validation. For example, VarInt::try_from((1u128 << 64) + 7) succeeds as VarInt(7), although the public conversion contract requires an error for every value at or above 2^62. Callers converting wide application counters can therefore serialize a different integer without receiving VarIntBoundsExceeded.',
            'refutation': refutation(
                'The narrowing operation is x.try_into(), a checked u128-to-u64 conversion whose error is mapped to VarIntBoundsExceeded and propagated by ?. It is not an as cast. Inputs fitting u64 then pass through from_u64, which separately requires x < 2^62. Probe negative_control_false_1_u128_conversion_checks_before_narrowing in evidence/probes/varint.rs accepts 2^62-1 and 7 with their exact values, and rejects 2^62, u64::MAX, 2^64, 2^64+7, and u128::MAX. In particular, the claimed counterexample returns Err rather than 7.',
                ['quinn-proto/src/varint.rs:30', 'quinn-proto/src/varint.rs:32', 'quinn-proto/src/varint.rs:102', 'quinn-proto/src/varint.rs:106']),
        },
        {
            'id': 'false-2',
            'title': 'Sparse connection-ID rotation advances into an empty slot',
            'location': 'quinn-proto/src/cid_queue.rs:92',
            'body': 'CidQueue::next selects the second occupied entry with iter().nth(1), but then uses its position as a ring-slot displacement when updating cursor and offset. If the active CID has sequence 0 and the next supplied CIDs have sequences 2 and 4, the missing sequence 1 makes the rotation advance by one instead of two. The active slot is then empty and active() panics, interrupting an otherwise valid CID rotation after reordered delivery.',
            'refutation': refutation(
                'CidQueue::iter returns (step, cid_data), preserving each entry\'s original ring displacement before filter_map removes empty slots (lines 103-105). nth(1) selects the second occupied entry but its tuple still contains step=2 in this example. next uses that displacement for both offset and cursor and clears the old active slot only after a successor has been found. Probe negative_control_false_2_sparse_cid_rotation_uses_sequence_offsets in evidence/probes/cid_queue.rs constructs exactly sequences 0, 2, and 4 with distinct IDs and tokens. Rotation yields the correct CID/token for 2 with retirement 0..2, then 4 with retirement 2..4; a subsequent next returns None while active remains the valid sequence-4 CID.',
                ['quinn-proto/src/cid_queue.rs:91', 'quinn-proto/src/cid_queue.rs:96', 'quinn-proto/src/cid_queue.rs:102', 'quinn-proto/src/cid_queue.rs:105', 'quinn-proto/src/cid_queue.rs:116']),
        },
        {
            'id': 'false-3',
            'title': 'Overlapping stream acknowledgments free buffered bytes twice',
            'location': 'quinn-proto/src/connection/send_buffer.rs:54',
            'body': 'SendBuffer::ack subtracts the acknowledged range from unacked_len even when it overlaps a previous acknowledgment or a prefix already removed from the buffer. Ordinary retransmissions can produce duplicate or overlapping acknowledgments of the same stream offsets. For a 12-byte stream, acknowledging 0..4, then 8..12 twice, then 2..6 causes already acknowledged bytes to be counted again, allowing outstanding bytes to be freed prematurely and corrupting subsequent buffer accounting.',
            'refutation': refutation(
                'ack first clamps both endpoints to the retained base offset (lines 44-46), inserts into a union of acknowledged ranges, and only subtracts a merged range when it begins at the retained base (lines 48-54). RangeSet::insert ignores empty/contained ranges and merges predecessor/successor overlap at btree_range_set.rs:58-80, so duplicates are not additive. Probe negative_control_false_3_overlapping_acks_do_not_double_subtract in evidence/probes/send_buffer.rs writes three 4-byte segments and transmits all 12 bytes. ACKs 0..4, 8..12, duplicate 8..12, and 2..6 leave respectively 8, 4, 4, and 2 outstanding bytes, with the still-unacknowledged gh accessible at 6..8. A final overlapping 4..10 ACK merges through the saved suffix to reach exactly zero; a late duplicate 0..12 leaves zero and is_fully_acked true.',
                ['quinn-proto/src/connection/send_buffer.rs:44', 'quinn-proto/src/connection/send_buffer.rs:48', 'quinn-proto/src/connection/send_buffer.rs:50', 'quinn-proto/src/connection/send_buffer.rs:188', 'quinn-proto/src/range_set/mod.rs:7', 'quinn-proto/src/range_set/btree_range_set.rs:58', 'quinn-proto/src/range_set/btree_range_set.rs:63', 'quinn-proto/src/range_set/btree_range_set.rs:72']),
        },
        {
            'id': 'false-4',
            'title': 'Switching to unordered reads exposes duplicate buffered stream bytes',
            'location': 'quinn-proto/src/connection/assembler.rs:40',
            'body': 'Assembler::ensure_ordering(false) switches the receive mode without normalizing frames already buffered in ordered mode. If an application reads offsets 0..3 in order, buffers two copies of 3..6 and an overlapping 4..8 frame, then switches to unordered reads, the existing heap entries remain independently readable. Offsets 3..6 can consequently be delivered more than once, even though unordered stream reads must still deliver each stream byte once. The received-range tracking only prevents duplicates from later insertions.',
            'refutation': refutation(
                'The mode transition explicitly calls defragment whenever buffered data exists (lines 45-47), before seeding recvd with both the consumed prefix and the normalized buffered chunks (lines 49-54). defragment processes chunks in stream order and try_mark_defragment removes complete duplicates or advances past overlapping prefixes (lines 273-283). Later unordered insertions also discard previously received ranges at lines 158-173. Probe negative_control_false_4_ordered_to_unordered_deduplicates_buffered_data in evidence/probes/assembler.rs reads abc in order, inserts exactly two def frames at offset 3 plus efgh at offset 4, and switches modes. Two-byte unordered reads deliver offsets 3 through 7 exactly once. Inserting a nonempty retransmission of abcdefghij then delivers only new offsets 8..10 (ij), with bytes_read exactly 10. This control concerns duplicate nonempty data during the mode transition, distinct from the supplied known zero-length unordered-frame defect.',
                ['quinn-proto/src/connection/assembler.rs:40', 'quinn-proto/src/connection/assembler.rs:45', 'quinn-proto/src/connection/assembler.rs:49', 'quinn-proto/src/connection/assembler.rs:112', 'quinn-proto/src/connection/assembler.rs:158', 'quinn-proto/src/connection/assembler.rs:273']),
        },
        {
            'id': 'false-5',
            'title': 'A zero idle-timeout advertisement disables the other endpoint\'s finite limit',
            'location': 'quinn-proto/src/connection/mod.rs:4067',
            'body': 'negotiate_max_idle_timeout treats a zero max_idle_timeout on either side as disabling the negotiated timeout for the whole connection. When one endpoint advertises 0 and the other advertises 5000 milliseconds, the helper returns None instead of retaining the finite five-second limit. Since reset_idle_timeout returns immediately for None, an idle connection can remain open past the timeout requested by the endpoint with the finite setting.',
            'refutation': refutation(
                'The first match arm requires BOTH tuple elements to be zero or None before returning None. The next two arms preserve the other endpoint\'s nonzero value when only one endpoint opts out, and the final arm takes the minimum of two finite limits (lines 4067-4070). set_peer_params actually assigns the result of this helper to self.idle_timeout at lines 3471-3472. Probe negative_control_false_5_zero_idle_timeout_preserves_other_peers_limit in evidence/probes/idle_timeout.rs verifies zero/5000 and None/5000 in both argument orders all return Some(5000ms), both-disabled cases return None, and 9000/5000 returns Some(5000ms). The timer consumer at line 1966 then uses the finite result, subject to its existing minimum of three PTOs. The probe directly tests the production negotiation helper; it does not simulate a five-second connection expiration.',
                ['quinn-proto/src/connection/mod.rs:3471', 'quinn-proto/src/connection/mod.rs:4065', 'quinn-proto/src/connection/mod.rs:4067', 'quinn-proto/src/connection/mod.rs:4068', 'quinn-proto/src/connection/mod.rs:4069', 'quinn-proto/src/connection/mod.rs:4070', 'quinn-proto/src/connection/mod.rs:1957', 'quinn-proto/src/connection/mod.rs:1966']),
        },
    ],
}
Path('/work/prepared/controls.json').write_text(json.dumps(controls, indent=2) + '\n')
