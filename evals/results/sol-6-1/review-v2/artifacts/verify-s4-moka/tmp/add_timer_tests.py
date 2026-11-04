from pathlib import Path
for kind in ('sync','future'):
    async_kind = kind=='future'
    await_ = '.await' if async_kind else ''
    test_attr = '#[tokio::test]\n    async fn' if async_kind else '#[test]\n    fn'
    text = '''
#[cfg(test)]
mod independent_timer_regression {
    use super::Cache;
    use crate::{Expiry, notification::RemovalCause, common::{time::Clock, concurrent::constants::READ_LOG_CH_SIZE}};
    use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::{Duration,Instant}};
    struct ReadExpiry;
    impl Expiry<u32,u32> for ReadExpiry {
        fn expire_after_read(&self,key:&u32,_:&u32,_:Instant,d:Option<Duration>,_:Instant)->Option<Duration> {
            if *key==1 {Some(Duration::from_millis(40))} else {d}
        }
    }
    TEST_ATTR dropped_read_loses_new_timer() {
        for drop_read in [false,true] {
            let (clock,mock) = Clock::mock();
            let notifications=Arc::new(AtomicUsize::new(0));
            let count=Arc::clone(&notifications);
            let mut c=Cache::builder().max_capacity(10).expire_after(ReadExpiry).clock(clock)
                .eviction_listener(move |_,_,cause| {if cause==RemovalCause::Expired {count.fetch_add(1,Ordering::SeqCst);}}).build();
            c.reconfigure_for_testing()AWAIT;
            c.insert(1,1)AWAIT; c.insert(2,2)AWAIT;
            c.run_pending_tasks()AWAIT;
            if drop_read {
                // Exercise the exact bounded-channel Full branch, using its
                // existing capacity. No stress or resource consumption probe.
                for k in 100..100+READ_LOG_CH_SIZE as u32 {assert_eq!(c.get(&k)AWAIT,None);}
            }
            assert_eq!(c.get(&1)AWAIT,Some(1));
            mock.increment(Duration::from_secs(2));
            for _ in 0..5 {c.run_pending_tasks()AWAIT;}
            assert_eq!(c.iter().count(),1);
            let stored_count=c.entry_count();
            let expired_notifications=notifications.load(Ordering::SeqCst);
            let retained_value=c.remove(&1)AWAIT;
            eprintln!("drop_read={drop_read}: count={stored_count}, expired notifications={expired_notifications}, removed={retained_value:?}");
            assert_eq!((stored_count,expired_notifications,retained_value),(1,1,None));
        }
    }
}
'''.replace('TEST_ATTR',test_attr).replace('AWAIT',await_)
    path=Path('/work/tmp/verify/src')/kind/'cache.rs'
    with path.open('a') as f: f.write(text)
