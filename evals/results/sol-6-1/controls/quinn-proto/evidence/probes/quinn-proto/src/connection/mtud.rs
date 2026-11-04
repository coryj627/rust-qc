
#[cfg(test)]
mod negative_control_probes {
    use super::*;

    #[test]
    fn false_3_mtu_probes_obey_peer_cap_and_cached_cap() {
        let mut config = MtuDiscoveryConfig::default();
        config.upper_bound(1452).minimum_change(1);
        let now = Instant::now();
        for initial in [1200, 1500] {
            let mut mtu = MtuDiscovery::new(initial, 1200, Some(1280), config.clone());
            assert_eq!(mtu.current_mtu(), initial.min(1280));
            let mut count = 0;
            for pn in 0..100 {
                let Some(size) = mtu.poll_transmit(now, pn) else { break };
                assert!(size <= 1280, "probe exceeded peer cap: {size}");
                assert!(mtu.on_acked(SpaceId::Data, pn, size));
                count += 1;
            }
            assert!(count < 100, "search did not complete");
            assert_eq!(mtu.current_mtu(), 1280);
            println!("false-3: initial={initial}, peer cap=1280, configured upper bound=1452: {count} probes, final MTU=1280");
        }
    }
}
