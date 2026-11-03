// crash: high peer min_ack_delay at low RTT panics Duration::clamp
// Injected into /work/reprotree and compiled as part of quinn-proto (reaches pub(crate)/pub(super)).


#[cfg(test)]
mod repro_key {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    // qp-6912dad (crash-from-input): candidate_max_ack_delay clamps a candidate delay
    // with a peer-derived lower bound (min_ack_delay) and an RTT-derived upper bound.
    // Nothing guarantees lower <= upper, so a peer advertising a large min_ack_delay at
    // low RTT makes Duration::clamp panic ("min > max"). Transport-parameter validation
    // only checks min_ack_delay <= max_ack_delay, not against the RTT.
    #[test]
    fn repro_key_6912dad_high_min_ack_delay_panics() {
        // Peer advertises min_ack_delay = 100_000 us (100 ms).
        let peer_params = TransportParameters {
            min_ack_delay: Some(100_000u32.into()),
            ..TransportParameters::default()
        };
        let state = AckFrequencyState::new(Duration::from_millis(100));
        let config = crate::AckFrequencyConfig::default();
        // Low RTT (5 ms): upper clamp bound becomes max(5ms, 25ms) = 25ms < 100ms lower.
        let res = catch_unwind(AssertUnwindSafe(|| {
            state.candidate_max_ack_delay(Duration::from_millis(5), &config, &peer_params)
        }));
        assert!(
            res.is_err(),
            "expected Duration::clamp to panic (min > max) at baseline; got {:?}",
            res.ok()
        );
    }
}
