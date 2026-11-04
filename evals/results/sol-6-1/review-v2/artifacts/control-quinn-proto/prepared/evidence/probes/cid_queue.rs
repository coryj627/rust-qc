use super::*;

#[test]
fn negative_control_false_2_sparse_cid_rotation_uses_sequence_offsets() {
    let initial = ConnectionId::new(b"initial0");
    let second = ConnectionId::new(b"second02");
    let fourth = ConnectionId::new(b"fourth04");
    let token2 = ResetToken::from([2; crate::RESET_TOKEN_SIZE]);
    let token4 = ResetToken::from([4; crate::RESET_TOKEN_SIZE]);
    let mut queue = CidQueue::new(initial);
    for (sequence, id, reset_token) in [(2, second, token2), (4, fourth, token4)] {
        assert_eq!(queue.insert(NewConnectionId { sequence, id, reset_token, retire_prior_to: 0 }), Ok(None));
    }
    assert_eq!(queue.next(), Some((token2, 0..2)));
    assert_eq!(queue.active_seq(), 2);
    assert_eq!(queue.active(), second);
    assert_eq!(queue.next(), Some((token4, 2..4)));
    assert_eq!(queue.active_seq(), 4);
    assert_eq!(queue.active(), fourth);
    assert_eq!(queue.next(), None);
    assert_eq!(queue.active(), fourth);
    println!("false-2: sparse 0/2/4 queue rotated to sequence 2 then 4; retirement ranges 0..2 and 2..4; active CID remains valid");
}
