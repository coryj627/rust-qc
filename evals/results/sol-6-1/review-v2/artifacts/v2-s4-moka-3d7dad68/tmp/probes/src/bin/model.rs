use moka::ops::compute::Op;
use std::collections::HashMap;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let sync = moka::sync::Cache::<String, u32>::builder()
        .support_invalidation_closures()
        .build();
    let segmented = moka::sync::Cache::<String, u32>::builder()
        .segments(3)
        .support_invalidation_closures()
        .build();
    let future = moka::future::Cache::<String, u32>::builder()
        .support_invalidation_closures()
        .build();
    let mut expected = HashMap::new();
    let mut seed = 0x123456789abcdefu64;
    for i in 0..10_000u32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let key = format!("clé-{}", (seed >> 16) % 40);
        match seed % 8 {
            0 => {
                expected.insert(key.clone(), i);
                sync.insert(key.clone(), i);
                segmented.insert(key.clone(), i);
                future.insert(key.clone(), i).await;
            }
            1 => {
                expected.remove(&key);
                sync.invalidate(key.as_str());
                segmented.invalidate(key.as_str());
                future.invalidate(key.as_str()).await;
            }
            2 => {
                let value = *expected.entry(key.clone()).or_insert(i);
                assert_eq!(sync.get_with_by_ref(key.as_str(), || i), value);
                assert_eq!(segmented.get_with_by_ref(key.as_str(), || i), value);
                assert_eq!(
                    future.get_with_by_ref(key.as_str(), async { i }).await,
                    value
                );
            }
            3 => {
                let value = expected.get(&key).copied().map(|v| v + 1).unwrap_or(1);
                expected.insert(key.clone(), value);
                assert_eq!(
                    sync.entry_by_ref(key.as_str())
                        .and_upsert_with(|e| e.map(|e| e.into_value() + 1).unwrap_or(1))
                        .into_value(),
                    value
                );
                assert_eq!(
                    future
                        .entry_by_ref(key.as_str())
                        .and_upsert_with(
                            |e| async move { e.map(|e| e.into_value() + 1).unwrap_or(1) }
                        )
                        .await
                        .into_value(),
                    value
                );
                segmented.insert(key.clone(), value);
            }
            4 => {
                let exp = expected.get(&key).copied();
                let a = sync.try_get_with_by_ref(key.as_str(), || Err::<u32, _>("missing"));
                let b = future
                    .try_get_with_by_ref(key.as_str(), async { Err::<u32, _>("missing") })
                    .await;
                assert_eq!(a.as_ref().ok().copied(), exp);
                assert_eq!(b.as_ref().ok().copied(), exp);
                if exp.is_none() {
                    assert_eq!(a.unwrap_err().as_ref(), &"missing");
                    assert_eq!(b.unwrap_err().as_ref(), &"missing");
                }
            }
            5 => {
                expected.retain(|_, v| *v % 3 != 0);
                sync.invalidate_entries_if(|_, v| v % 3 == 0).unwrap();
                segmented.invalidate_entries_if(|_, v| v % 3 == 0).unwrap();
                future.invalidate_entries_if(|_, v| v % 3 == 0).unwrap();
            }
            6 if i % 17 == 0 => {
                expected.clear();
                sync.invalidate_all();
                segmented.invalidate_all();
                future.invalidate_all();
            }
            _ => {
                let result = sync.entry(key.clone()).and_compute_with(|_| Op::Nop);
                assert_eq!(
                    result.into_entry().map(|e| e.into_value()),
                    expected.get(&key).copied()
                );
            }
        }
        assert_eq!(sync.get(key.as_str()), expected.get(&key).copied());
        assert_eq!(segmented.get(key.as_str()), expected.get(&key).copied());
        assert_eq!(future.get(key.as_str()).await, expected.get(&key).copied());
        if i % 100 == 0 {
            sync.run_pending_tasks();
            segmented.run_pending_tasks();
            future.run_pending_tasks().await;
            let actual: HashMap<_, _> = sync.iter().map(|(k, v)| ((*k).clone(), v)).collect();
            assert_eq!(actual, expected);
            let actual: HashMap<_, _> = future.iter().map(|(k, v)| ((*k).clone(), v)).collect();
            assert_eq!(actual, expected);
        }
    }
    println!("10,000 model operations passed for sync, segmented, and future caches");
}
