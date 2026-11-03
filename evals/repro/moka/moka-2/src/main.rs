//! moka-2: future::Cache::and_compute_with drops its per-key waiter before it writes,
//! so a second call on the key reads the old value and one update is lost.
//! Docs (src/future/entry_selector.rs:163): "concurrent calls on the same key are
//! executed serially". A serial counter must therefore end at writers * writes.
//! Exit 0 = at least one round lost increments.
use moka::{future::Cache, ops::compute};

const WRITERS: u64 = 8;
const WRITES: u64 = 100;
const ROUNDS: u64 = 10;

async fn one_round() -> u64 {
    let cache: Cache<String, u64> = Cache::builder().max_capacity(100).build();
    let key = "counter".to_string();
    cache.insert(key.clone(), 0).await;
    let mut handles = vec![];
    for _ in 0..WRITERS {
        let cache = cache.clone();
        let key = key.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..WRITES {
                cache
                    .entry_by_ref(&key)
                    .and_compute_with(|entry| async move {
                        match entry {
                            Some(e) => compute::Op::Put(e.into_value() + 1),
                            None => compute::Op::Nop,
                        }
                    })
                    .await;
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    cache.get(&key).await.unwrap()
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    let expected = WRITERS * WRITES;
    let (mut hit, mut lost_total) = (0, 0);
    for r in 0..ROUNDS {
        let actual = one_round().await;
        println!("round {r}: counter = {actual}, expected {expected}, lost {}", expected - actual);
        if actual != expected {
            hit += 1;
            lost_total += expected - actual;
        }
    }
    println!("rounds with lost updates: {hit} of {ROUNDS}; increments lost: {lost_total} of {}", expected * ROUNDS);
    std::process::exit(if hit > 0 { 0 } else { 1 });
}
