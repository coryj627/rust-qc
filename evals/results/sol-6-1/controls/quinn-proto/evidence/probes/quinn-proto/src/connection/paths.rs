
#[cfg(test)]
mod negative_control_probes {
    use super::*;

    #[test]
    fn false_1_ack_delay_cannot_underflow_rtt_adjustment() {
        let mut rtt = RttEstimator::new(Duration::from_millis(100));
        rtt.update(Duration::ZERO, Duration::from_millis(10));
        rtt.update(Duration::from_millis(20), Duration::from_millis(10));
        assert_eq!(rtt.latest, Duration::from_millis(10));
        assert_eq!(rtt.min(), Duration::from_millis(10));
        assert_eq!(rtt.get(), Duration::from_millis(10));
        // Even when subtraction is numerically possible, it must not lower
        // the sample below the minimum: 15 - 10 < 10, so retain 15.
        rtt.update(Duration::from_millis(10), Duration::from_millis(15));
        assert_eq!(rtt.get(), Duration::from_micros(10_625));
        println!("false-1: 20ms delay with 10ms RTT is ignored; below-minimum adjustment is also ignored");
    }

    #[test]
    fn false_4_path_challenges_are_deduplicated_and_bounded() {
        let remote: SocketAddr = "127.0.0.1:4000".parse().unwrap();
        let mut responses = PathResponses::default();
        for packet in 0..1_000 {
            responses.push(packet, packet + 10, remote);
        }
        assert_eq!(responses.pending.len(), 1);
        responses.push(1, 999_999, remote); // older packet cannot replace newest
        assert_eq!(responses.pop_on_path(remote), Some(1_009));
        for index in 0..1_000u16 {
            responses.push(index as u64, index as u64, SocketAddr::new(remote.ip(), 5000 + index));
        }
        assert_eq!(responses.pending.len(), 16);
        println!("false-4: 1000 same-address challenges retain 1 response; 1000 distinct addresses retain 16");
    }
}
