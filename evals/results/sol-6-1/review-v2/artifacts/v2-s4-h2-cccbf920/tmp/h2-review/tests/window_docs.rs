use h2_support::prelude::*;

#[tokio::test]
async fn initial_window_size_panics_before_returning_a_result() {
    let (io, _peer) = mock::new();
    let (_client, mut connection) = client::handshake(io).await.unwrap();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        connection.set_initial_window_size(u32::MAX)
    }));
    assert!(outcome.is_err());
}
