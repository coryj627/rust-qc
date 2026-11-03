#[cfg(test)]
mod negative_control_probe {
    use super::*;

    #[test]
    fn false_5_unrelated_ack_preserves_shutdown_ping() {
        let mut pp = PingPong::new();
        pp.ping_shutdown();
        // Simulate the existing pending shutdown PING having been transmitted.
        pp.pending_ping.as_mut().unwrap().sent = true;
        let wrong = [0x5a; 8];
        assert_ne!(wrong, Ping::SHUTDOWN);
        assert!(matches!(pp.recv_ping(Ping::pong(wrong)), ReceivedPing::Unknown));
        let pending = pp.pending_ping.as_ref().expect("shutdown ping must be retained");
        assert_eq!(pending.payload, Ping::SHUTDOWN);
        assert!(pending.sent);
        assert!(matches!(pp.recv_ping(Ping::pong(Ping::SHUTDOWN)), ReceivedPing::Shutdown));
        assert!(pp.pending_ping.is_none());
        println!("false-5: unrelated ACK retains shutdown payload and sent flag; subsequent matching ACK completes shutdown");
    }
}
