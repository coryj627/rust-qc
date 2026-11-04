#[cfg(test)]
mod negative_control_probe {
    use super::*;

    #[test]
    fn false_4_negative_send_window_clamps_and_recovers() {
        let mut flow = FlowControl::new();
        flow.inc_window(65_535).unwrap();
        flow.assign_capacity(65_535).unwrap();
        flow.send_data(5).unwrap();
        // SETTINGS_INITIAL_WINDOW_SIZE changes from 65535 to 0.
        flow.dec_send_window(65_535).unwrap();
        assert_eq!(flow.window_size.0, -5);
        assert_eq!(flow.window_size(), 0);
        // Applying a WINDOW_UPDATE that only clears the debt gives no credit.
        flow.inc_window(5).unwrap();
        assert_eq!(flow.window_size.0, 0);
        assert_eq!(flow.window_size(), 0);
        flow.inc_window(1).unwrap();
        assert_eq!(flow.window_size(), 1);
        println!("false-4: raw send window -5 exposes zero capacity; WINDOW_UPDATE(5) gives zero, then WINDOW_UPDATE(1) gives one");
    }
}
