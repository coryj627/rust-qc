impl<K, V, S> Inner<K, V, S>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    S: BuildHasher + Clone + Send + Sync + 'static,
{
    fn do_run_pending_tasks(
        &self,
        timeout: Option<Duration>,
        max_log_sync_repeats: u32,
        eviction_batch_size: u32,
    ) -> bool {
        if self.max_capacity == Some(0) {
            return false;
        }

        // Acquire some locks.
        let mut deqs = self.deques.lock();
        let mut timer_wheel = self.timer_wheel.lock();

        let started_at = if timeout.is_some() {
            Some(self.current_time())
        } else {
            None
        };
        let mut should_process_logs = true;
        let mut calls = 0u32;
        let current_ec = self.entry_count.load();
        let current_ws = self.weighted_size.load();
        let mut eviction_state =
            EvictionState::new(current_ec, current_ws, self.removal_notifier.as_ref());

        loop {
            if should_process_logs {
                let r_len = self.read_op_ch.len();
                if r_len > 0 {
                    self.apply_reads(&mut deqs, &mut timer_wheel, r_len);
                }

                let w_len = self.write_op_ch.len();
                if w_len > 0 {
                    self.apply_writes(&mut deqs, &mut timer_wheel, w_len, &mut eviction_state);
                }

                if self.eviction_policy == EvictionPolicyConfig::TinyLfu
                    && self.should_enable_frequency_sketch(&eviction_state.counters)
                {
                    self.enable_frequency_sketch(&eviction_state.counters);
                }

                calls += 1;
            }

            // Set this flag to `false`. The `evict_*` and `invalidate_*` methods
            // below may set it to `true` if there are more entries to evict in next
            // loop.
            eviction_state.more_entries_to_evict = false;
            let last_eviction_count = eviction_state.counters.eviction_count;

            // Evict entries if there are any expired entries in the hierarchical
            // timer wheels.
            if timer_wheel.is_enabled() {
                self.evict_expired_entries_using_timers(
                    &mut timer_wheel,
                    &mut deqs,
                    &mut eviction_state,
                );
            }

            // Evict entries if there are any expired entries in the write order or
            // access order deques.
            if self.has_expiry() || self.has_valid_after() {
                self.evict_expired_entries_using_deqs(
                    &mut deqs,
                    &mut timer_wheel,
                    eviction_batch_size,
                    &mut eviction_state,
                );
            }

            // Evict entries if there are any invalidation predicates set by the
            // `invalidate_entries_if` method.
            if let Some(invalidator) = &self.invalidator {
                if !invalidator.is_empty() {
                    self.invalidate_entries(
                        invalidator,
                        &mut deqs,
                        &mut timer_wheel,
                        eviction_batch_size,
                        &mut eviction_state,
                    );
                }
            }

            // Evict if this cache has more entries than its capacity.
            let weights_to_evict = self.weights_to_evict(&eviction_state.counters);
            if weights_to_evict > 0 {
                self.evict_lru_entries(
                    &mut deqs,
                    &mut timer_wheel,
                    eviction_batch_size,
                    weights_to_evict,
                    &mut eviction_state,
                );
            }

            // Check whether to continue this loop or not.

            should_process_logs = calls <= max_log_sync_repeats
                && (self.read_op_ch.len() >= READ_LOG_FLUSH_POINT
                    || self.write_op_ch.len() >= WRITE_LOG_FLUSH_POINT);

            let should_evict_more_entries = eviction_state.more_entries_to_evict
                // Check if there were any entries evicted in this loop.
                && (eviction_state.counters.eviction_count - last_eviction_count) > 0;

            // Break the loop if there will be nothing to do in next loop.
            if !should_process_logs && !should_evict_more_entries {
                break;
            }

            // Break the loop if the eviction listener is set and timeout has been
            // reached.
            if let (Some(to), Some(started)) = (timeout, started_at) {
                let elapsed = self.current_time().saturating_duration_since(started);
                if elapsed >= to {
                    break;
                }
            }
        }

        debug_assert_eq!(self.entry_count.load(), current_ec);
        debug_assert_eq!(self.weighted_size.load(), current_ws);
        self.entry_count.store(eviction_state.counters.entry_count);
        self.weighted_size
            .store(eviction_state.counters.weighted_size);

        crossbeam_epoch::pin().flush();

        // Ensure the deqs lock is held until here.
        drop(deqs);

        eviction_state.more_entries_to_evict
    }
}

//
// private methods
//
impl<K, V, S> Inner<K, V, S>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Send + Sync + 'static,
    S: BuildHasher + Clone + Send + Sync + 'static,
{
    fn has_enough_capacity(&self, candidate_weight: u32, counters: &EvictionCounters) -> bool {
        self.max_capacity.map_or(true, |limit| {
            counters.weighted_size + candidate_weight as u64 <= limit
        })
    }

    fn weights_to_evict(&self, counters: &EvictionCounters) -> u64 {
        self.max_capacity
            .map(|limit| counters.weighted_size.saturating_sub(limit))
            .unwrap_or_default()
    }

    #[inline]
    fn should_enable_frequency_sketch(&self, counters: &EvictionCounters) -> bool {
        match self.max_capacity {
            None | Some(0) => false,
            Some(max_cap) => {
                if self.frequency_sketch_enabled.load(Ordering::Acquire) {
                    false // The frequency sketch is already enabled.
                } else {
                    counters.weighted_size >= max_cap / 2
                }
            }
        }
    }

    #[inline]
    fn enable_frequency_sketch(&self, counters: &EvictionCounters) {
        if let Some(max_cap) = self.max_capacity {
            let c = counters;
            let cap = if self.weigher.is_none() {
                max_cap
            } else {
                (c.entry_count as f64 * (c.weighted_size as f64 / max_cap as f64)) as u64
            };
            self.do_enable_frequency_sketch(cap);
        }
    }

    #[cfg(test)]
    fn enable_frequency_sketch_for_testing(&self) {
        if let Some(max_cap) = self.max_capacity {
            self.do_enable_frequency_sketch(max_cap);
        }
    }

    #[inline]
    fn do_enable_frequency_sketch(&self, cache_capacity: u64) {
        let skt_capacity = common::sketch_capacity(cache_capacity);
        self.frequency_sketch.write().ensure_capacity(skt_capacity);
        self.frequency_sketch_enabled.store(true, Ordering::Release);
    }

    fn apply_reads(&self, deqs: &mut Deques<K>, timer_wheel: &mut TimerWheel<K>, count: usize) {
        use ReadOp::{Hit, Miss};
        let mut freq = self.frequency_sketch.write();
        let ch = &self.read_op_ch;
        for _ in 0..count {
            match ch.try_recv() {
                Ok(Hit {
                    value_entry,
                    is_expiry_modified,
                }) => {
                    let kh = value_entry.entry_info().key_hash();
                    freq.increment(kh.hash);
                    if is_expiry_modified {
                        self.update_timer_wheel(&value_entry, timer_wheel);
                    }
                    deqs.move_to_back_ao(&value_entry);
                }
                Ok(Miss(hash)) => freq.increment(hash),
                Err(_) => break,
            }
        }
    }

    fn apply_writes(
        &self,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        count: usize,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        use WriteOp::{Remove, Upsert};
        let freq = self.frequency_sketch.read();
        let ch = &self.write_op_ch;

        for _ in 0..count {
            match ch.try_recv() {
                Ok(Upsert {
                    key_hash: kh,
                    value_entry: entry,
                    entry_gen: gen,
                    old_weight,
                    new_weight,
                }) => self.handle_upsert(
                    kh,
                    entry,
                    gen,
                    old_weight,
                    new_weight,
                    deqs,
                    timer_wheel,
                    &freq,
                    eviction_state,
                ),
                Ok(Remove {
                    kv_entry: KvEntry { key: _key, entry },
                    entry_gen: gen,
                }) => {
                    Self::handle_remove(
                        deqs,
                        timer_wheel,
                        entry,
                        Some(gen),
                        &mut eviction_state.counters,
                    );
                }
                Err(_) => break,
            };
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_upsert(
        &self,
        kh: KeyHash<K>,
        entry: MiniArc<ValueEntry<K, V>>,
        gen: u16,
        old_weight: u32,
        new_weight: u32,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        freq: &FrequencySketch,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        {
            let counters = &mut eviction_state.counters;

            if entry.is_admitted() {
                // The entry has been already admitted, so treat this as an update.
                counters.saturating_sub(0, old_weight);
                counters.saturating_add(0, new_weight);
                self.update_timer_wheel(&entry, timer_wheel);
                deqs.move_to_back_ao(&entry);
                deqs.move_to_back_wo(&entry);
                entry.entry_info().set_policy_gen(gen);
                return;
            }

            if self.has_enough_capacity(new_weight, counters) {
                // There are enough room in the cache (or the cache is unbounded).
                // Add the candidate to the deques.
                self.handle_admit(&entry, new_weight, deqs, timer_wheel, counters);
                entry.entry_info().set_policy_gen(gen);
                return;
            }
        }

        if let Some(max) = self.max_capacity {
            if new_weight as u64 > max {
                // The candidate is too big to fit in the cache. Reject it.

                // Lock the key for removal if blocking removal notification is enabled.
                let kl = self.maybe_key_lock(&kh.key);
                let _klg = &kl.as_ref().map(|kl| kl.lock());

                let removed = self.cache.remove_if(
                    kh.hash,
                    |k| k == &kh.key,
                    |_, current_entry| {
                        MiniArc::ptr_eq(entry.entry_info(), current_entry.entry_info())
                            && current_entry.entry_info().entry_gen() == gen
                    },
                );
                if let Some(entry) = removed {
                    if eviction_state.is_notifier_enabled() {
                        let key = Arc::clone(&kh.key);
                        eviction_state.notify_entry_removal(key, &entry, RemovalCause::Size);
                    }
                    eviction_state.counters.incr_eviction_count();
                }
                entry.entry_info().set_policy_gen(gen);
                return;
            }
        }

        // TODO: Refactoring the policy implementations.
        // https://github.com/moka-rs/moka/issues/389

        // Try to admit the candidate.
        let admission_result = match &self.eviction_policy {
            EvictionPolicyConfig::TinyLfu => {
                let mut candidate = EntrySizeAndFrequency::new(new_weight);
                candidate.add_frequency(freq, kh.hash);
                Self::admit(&candidate, &self.cache, deqs, freq)
            }
            EvictionPolicyConfig::Lru => AdmissionResult::Admitted {
                victim_keys: SmallVec::default(),
            },
        };

        match admission_result {
            AdmissionResult::Admitted { victim_keys } => {
                // Try to remove the victims from the hash map.
                for (vic_kh, vic_la) in victim_keys {
                    let vic_key = vic_kh.key;
                    let vic_hash = vic_kh.hash;

                    // Lock the key for removal if blocking removal notification is enabled.
                    let kl = self.maybe_key_lock(&vic_key);
                    let _klg = &kl.as_ref().map(|kl| kl.lock());

                    if let Some((vic_key, vic_entry)) = self.cache.remove_entry_if_and(
                        vic_hash,
                        |k| k == &vic_key,
                        |_, entry| entry.entry_info().last_accessed() == vic_la,
                        |k, v| (k.clone(), v.clone()),
                    ) {
                        if eviction_state.is_notifier_enabled() {
                            eviction_state.notify_entry_removal(
                                vic_key,
                                &vic_entry,
                                RemovalCause::Size,
                            );
                        }
                        eviction_state.counters.incr_eviction_count();
                        // And then remove the victim from the deques.
                        Self::handle_remove(
                            deqs,
                            timer_wheel,
                            vic_entry,
                            None,
                            &mut eviction_state.counters,
                        );
                    } else {
                        // Could not remove the victim from the cache. Skip it as its
                        // ValueEntry might have been invalidated.
                        if let Some(node) = deqs.probation.peek_front() {
                            if node.element.key() == &vic_key && node.element.hash() == vic_hash {
                                deqs.probation.move_front_to_back();
                            }
                        }
                    }
                }

                // Add the candidate to the deques.
                self.handle_admit(
                    &entry,
                    new_weight,
                    deqs,
                    timer_wheel,
                    &mut eviction_state.counters,
                );
                entry.entry_info().set_policy_gen(gen);
            }
            AdmissionResult::Rejected => {
                // Lock the key for removal if blocking removal notification is enabled.
                let kl = self.maybe_key_lock(&kh.key);
                let _klg = &kl.as_ref().map(|kl| kl.lock());

                // Remove the candidate from the cache (hash map) if the entry
                // generation matches.
                let key = Arc::clone(&kh.key);
                let removed = self.cache.remove_if(
                    kh.hash,
                    |k| k == &key,
                    |_, current_entry| {
                        MiniArc::ptr_eq(entry.entry_info(), current_entry.entry_info())
                            && current_entry.entry_info().entry_gen() == gen
                    },
                );

                if let Some(entry) = removed {
                    entry.entry_info().set_policy_gen(gen);
                    if eviction_state.is_notifier_enabled() {
                        eviction_state.notify_entry_removal(key, &entry, RemovalCause::Size);
                    }
                    eviction_state.counters.incr_eviction_count();
                }
            }
        };
    }

    /// Performs size-aware admission explained in the paper:
    /// [Lightweight Robust Size Aware Cache Management][size-aware-cache-paper]
    /// by Gil Einziger, Ohad Eytan, Roy Friedman, Ben Manes.
    ///
    /// [size-aware-cache-paper]: https://arxiv.org/abs/2105.08770
    ///
    /// There are some modifications in this implementation:
    /// - To admit to the main space, candidate's frequency must be higher than
    ///   the aggregated frequencies of the potential victims. (In the paper,
    ///   `>=` operator is used rather than `>`)  The `>` operator will do a better
    ///   job to prevent the main space from polluting.
    /// - When a candidate is rejected, the potential victims will stay at the LRU
    ///   position of the probation access-order queue. (In the paper, they will be
    ///   promoted (to the MRU position?) to force the eviction policy to select a
    ///   different set of victims for the next candidate). We may implement the
    ///   paper's behavior later?
    ///
    #[inline]
    fn admit(
        candidate: &EntrySizeAndFrequency,
        cache: &CacheStore<K, V, S>,
        deqs: &mut Deques<K>,
        freq: &FrequencySketch,
    ) -> AdmissionResult<K> {
        const MAX_CONSECUTIVE_RETRIES: usize = 5;
        let mut retries = 0;

        let mut victims = EntrySizeAndFrequency::default();
        let mut victim_keys = SmallVec::default();

        let deq = &mut deqs.probation;

        // Get first potential victim at the LRU position.
        let mut next_victim = deq.peek_front_ptr();

        // Aggregate potential victims.
        while victims.policy_weight < candidate.policy_weight
            && victims.freq <= candidate.freq
            && retries <= MAX_CONSECUTIVE_RETRIES
        {
            let Some(victim) = next_victim.take() else {
                // No more potential victims.
                break;
            };
            next_victim = DeqNode::next_node_ptr(victim);

            let vic_elem = &unsafe { victim.as_ref() }.element;
            if vic_elem.is_dirty() {
                // Skip this node as its ValueEntry have been updated or invalidated.
                unsafe { deq.move_to_back(victim) };
                retries += 1;
                continue;
            }

            let key = vic_elem.key();
            let hash = vic_elem.hash();
            let last_accessed = vic_elem.entry_info().last_accessed();

            if let Some(vic_entry) = cache.get(hash, |k| k == key) {
                victims.add_policy_weight(vic_entry.policy_weight());
                victims.add_frequency(freq, hash);
                victim_keys.push((KeyHash::new(Arc::clone(key), hash), last_accessed));
                retries = 0;
            } else {
                // Could not get the victim from the cache (hash map). Skip this node
                // as its ValueEntry might have been invalidated (after we checked
                // `is_dirty` above`).
                unsafe { deq.move_to_back(victim) };
                retries += 1;
            }
        }

        // Admit or reject the candidate.

        // TODO: Implement some randomness to mitigate hash DoS attack.
        // See Caffeine's implementation.

        if victims.policy_weight >= candidate.policy_weight && candidate.freq > victims.freq {
            AdmissionResult::Admitted { victim_keys }
        } else {
            AdmissionResult::Rejected
        }
    }

    fn handle_admit(
        &self,
        entry: &MiniArc<ValueEntry<K, V>>,
        policy_weight: u32,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        counters: &mut EvictionCounters,
    ) {
        counters.saturating_add(1, policy_weight);

        self.update_timer_wheel(entry, timer_wheel);

        // Update the deques.
        deqs.push_back_ao(
            CacheRegion::MainProbation,
            KeyHashDate::new(entry.entry_info()),
            entry,
        );
        if self.is_write_order_queue_enabled() {
            deqs.push_back_wo(KeyHashDate::new(entry.entry_info()), entry);
        }
        entry.set_admitted(true);
    }

    /// NOTE: This method may enable the timer wheel.
    fn update_timer_wheel(
        &self,
        entry: &MiniArc<ValueEntry<K, V>>,
        timer_wheel: &mut TimerWheel<K>,
    ) {
        // Enable the timer wheel if needed.
        if entry.entry_info().expiration_time().is_some() && !timer_wheel.is_enabled() {
            timer_wheel.enable();
        }

        // Update the timer wheel.
        match (
            entry.entry_info().expiration_time().is_some(),
            entry.timer_node(),
        ) {
            // Do nothing; the cache entry has no expiration time and not registered
            // to the timer wheel.
            (false, None) => (),
            // Register the cache entry to the timer wheel; the cache entry has an
            // expiration time and not registered to the timer wheel.
            (true, None) => {
                let timer = timer_wheel.schedule(
                    MiniArc::clone(entry.entry_info()),
                    MiniArc::clone(entry.deq_nodes()),
                );
                entry.set_timer_node(timer);
            }
            // Reschedule the cache entry in the timer wheel; the cache entry has an
            // expiration time and already registered to the timer wheel.
            (true, Some(tn)) => {
                let result = timer_wheel.reschedule(tn);
                if let ReschedulingResult::Removed(removed_tn) = result {
                    // The timer node was removed from the timer wheel because the
                    // expiration time has been unset by other thread after we
                    // checked.
                    entry.set_timer_node(None);
                    drop(removed_tn);
                }
            }
            // Unregister the cache entry from the timer wheel; the cache entry has
            // no expiration time but registered to the timer wheel.
            (false, Some(tn)) => {
                entry.set_timer_node(None);
                timer_wheel.deschedule(tn);
            }
        }
    }

    fn handle_remove(
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        entry: MiniArc<ValueEntry<K, V>>,
        gen: Option<u16>,
        counters: &mut EvictionCounters,
    ) {
        if let Some(timer_node) = entry.take_timer_node() {
            timer_wheel.deschedule(timer_node);
        }
        Self::handle_remove_without_timer_wheel(deqs, entry, gen, counters);
    }

    fn handle_remove_without_timer_wheel(
        deqs: &mut Deques<K>,
        entry: MiniArc<ValueEntry<K, V>>,
        gen: Option<u16>,
        counters: &mut EvictionCounters,
    ) {
        if entry.is_admitted() {
            entry.set_admitted(false);
            counters.saturating_sub(1, entry.policy_weight());
            // The following two unlink_* functions will unset the deq nodes.
            deqs.unlink_ao(&entry);
            Deques::unlink_wo(&mut deqs.write_order, &entry);
        } else {
            entry.unset_q_nodes();
        }
        if let Some(g) = gen {
            entry.entry_info().set_policy_gen(g);
        }
    }

    fn handle_remove_with_deques(
        ao_deq_name: &str,
        ao_deq: &mut Deque<KeyHashDate<K>>,
        wo_deq: &mut Deque<KeyHashDate<K>>,
        timer_wheel: &mut TimerWheel<K>,
        entry: MiniArc<ValueEntry<K, V>>,
        counters: &mut EvictionCounters,
    ) {
        if let Some(timer) = entry.take_timer_node() {
            timer_wheel.deschedule(timer);
        }
        if entry.is_admitted() {
            entry.set_admitted(false);
            counters.saturating_sub(1, entry.policy_weight());
            // The following two unlink_* functions will unset the deq nodes.
            Deques::unlink_ao_from_deque(ao_deq_name, ao_deq, &entry);
            Deques::unlink_wo(wo_deq, &entry);
        } else {
            entry.unset_q_nodes();
        }
    }

    fn evict_expired_entries_using_timers(
        &self,
        timer_wheel: &mut TimerWheel<K>,
        deqs: &mut Deques<K>,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        use crate::common::timer_wheel::TimerEvent;

        let now = self.current_time();

        // NOTES:
        //
        // 1. When necessary, the iterator returned from advance() will unset the
        //    timer node pointer in the `ValueEntry`, so we do not have to do it
        //    here.
        // 2. If an entry is dirty or `cache.remove_if` returns `None`, we will skip
        //    it as it may have been read, updated or invalidated by other thread.
        //    - The timer node should have been unset in the current `ValueEntry` as
        //      described above.
        //    - When necessary, a new timer node will be recreated for the current or
        //      new `ValueEntry` when its `WriteOp` or `ReadOp` is processed.
        for event in timer_wheel.advance(now) {
            // We do not have to do anything if event is `TimerEvent::Descheduled(_)`
            // or `TimerEvent::Rescheduled(_)`.
            if let TimerEvent::Expired(node) = event {
                let entry_info = node.element.entry_info();

                if entry_info.is_dirty() {
                    // Skip this entry as it has been updated or invalidated by other
                    // thread.
                    continue;
                }

                let kh = entry_info.key_hash();
                let key = &kh.key;
                let hash = kh.hash;

                // Lock the key for removal if blocking removal notification is
                // enabled.
                let kl = self.maybe_key_lock(key);
                let _klg = &kl.as_ref().map(|kl| kl.lock());

                // Remove the key from the map only when the entry is really expired.
                let maybe_entry = self.cache.remove_if(
                    hash,
                    |k| k == key,
                    |_, v| is_expired_by_per_entry_ttl(v.entry_info(), now),
                );

                if let Some(entry) = maybe_entry {
                    if eviction_state.is_notifier_enabled() {
                        let key = Arc::clone(key);
                        eviction_state.notify_entry_removal(key, &entry, RemovalCause::Expired);
                    }
                    eviction_state.counters.incr_eviction_count();
                    Self::handle_remove_without_timer_wheel(
                        deqs,
                        entry,
                        None,
                        &mut eviction_state.counters,
                    );
                } else {
                    // Skip this entry as the key may have been read, updated or
                    // invalidated by other thread.
                }
            }
        }
    }

    fn evict_expired_entries_using_deqs(
        &self,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        batch_size: u32,
        state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        use CacheRegion::{MainProbation as Probation, MainProtected as Protected, Window};

        let now = self.current_time();

        if self.is_write_order_queue_enabled() {
            self.remove_expired_wo(deqs, timer_wheel, batch_size, now, state);
        }

        if self.expiration_policy.time_to_idle().is_some() || self.has_valid_after() {
            self.remove_expired_ao(Window, deqs, timer_wheel, batch_size, now, state);
            self.remove_expired_ao(Probation, deqs, timer_wheel, batch_size, now, state);
            self.remove_expired_ao(Protected, deqs, timer_wheel, batch_size, now, state);
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn remove_expired_ao(
        &self,
        cache_region: CacheRegion,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        batch_size: u32,
        now: Instant,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        let tti = &self.expiration_policy.time_to_idle();
        let va = &self.valid_after();
        let deq_name = cache_region.name();
        let (ao_deq, wo_deq) = deqs.select_mut(cache_region);
        let mut more_to_evict = true;

        for _ in 0..batch_size {
            let maybe_key_hash_ts = ao_deq.peek_front().map(|node| {
                let elem = &node.element;
                (
                    Arc::clone(elem.key()),
                    elem.hash(),
                    elem.is_dirty(),
                    elem.last_accessed(),
                )
            });

            let (key, hash, cause) = match maybe_key_hash_ts {
                Some((key, hash, false, Some(ts))) => {
                    let cause = match is_entry_expired_ao_or_invalid(tti, va, ts, now) {
                        (true, _) => RemovalCause::Expired,
                        (false, true) => RemovalCause::Explicit,
                        (false, false) => {
                            more_to_evict = false;
                            break;
                        }
                    };
                    (key, hash, cause)
                }
                // TODO: Remove the second pattern `Some((_key, false, None))` once
                // we change `last_modified` and `last_accessed` in `EntryInfo` from
                // `Option<Instant>` to `Instant`.
                Some((key, hash, true, _) | (key, hash, false, None)) => {
                    // `is_dirty` is true or `last_modified` is None. Skip this entry
                    // as it may have been updated by this or other async task but
                    // its `WriteOp` is not processed yet.
                    self.skip_updated_entry_ao(&key, hash, deq_name, ao_deq, wo_deq);
                    // Set `more_to_evict` to `false` to make `run_pending_tasks` to
                    // return early. This will help that `schedule_write_op` to send
                    // the `WriteOp` to the write op channel.
                    more_to_evict = false;
                    continue;
                }
                None => {
                    more_to_evict = false;
                    break;
                }
            };

            // Lock the key for removal if blocking removal notification is enabled.
            let kl = self.maybe_key_lock(&key);
            let _klg = &kl.as_ref().map(|kl| kl.lock());

            // Remove the key from the map only when the entry is really
            // expired. This check is needed because it is possible that the entry in
            // the map has been updated or deleted but its deque node we checked
            // above has not been updated yet.
            let maybe_entry = self.cache.remove_if(
                hash,
                |k| k == &key,
                |_, v| is_expired_entry_ao(tti, va, v, now),
            );

            if let Some(entry) = maybe_entry {
                if eviction_state.is_notifier_enabled() {
                    eviction_state.notify_entry_removal(key, &entry, cause);
                }
                eviction_state.counters.incr_eviction_count();
                Self::handle_remove_with_deques(
                    deq_name,
                    ao_deq,
                    wo_deq,
                    timer_wheel,
                    entry,
                    &mut eviction_state.counters,
                );
            } else {
                self.skip_updated_entry_ao(&key, hash, deq_name, ao_deq, wo_deq);
                more_to_evict = false;
            }
        }

        if more_to_evict {
            eviction_state.more_entries_to_evict = true;
        }
    }

    #[inline]
    fn skip_updated_entry_ao(
        &self,
        key: &K,
        hash: u64,
        deq_name: &str,
        deq: &mut Deque<KeyHashDate<K>>,
        write_order_deq: &mut Deque<KeyHashDate<K>>,
    ) {
        if let Some(entry) = self.cache.get(hash, |k| (k.borrow() as &K) == key) {
            // The key exists and the entry may have been read or updated by other
            // thread.
            Deques::move_to_back_ao_in_deque(deq_name, deq, &entry);
            if entry.is_dirty() {
                Deques::move_to_back_wo_in_deque(write_order_deq, &entry);
            }
        } else {
            // Skip this entry as the key may have been invalidated by other thread.
            // Since the invalidated ValueEntry (which should be still in the write
            // op queue) has a pointer to this node, move the node to the back of the
            // deque instead of popping (dropping) it.
            deq.move_front_to_back();
        }
    }

    #[inline]
    fn skip_updated_entry_wo(&self, key: &K, hash: u64, deqs: &mut Deques<K>) {
        if let Some(entry) = self.cache.get(hash, |k| (k.borrow() as &K) == key) {
            // The key exists and the entry may have been read or updated by other
            // thread.
            deqs.move_to_back_ao(&entry);
            deqs.move_to_back_wo(&entry);
        } else {
            // Skip this entry as the key may have been invalidated by other thread.
            // Since the invalidated `ValueEntry` (which should be still in the write
            // op queue) has a pointer to this node, move the node to the back of the
            // deque instead of popping (dropping) it.
            deqs.write_order.move_front_to_back();
        }
    }

    #[inline]
    fn remove_expired_wo(
        &self,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        batch_size: u32,
        now: Instant,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        let ttl = &self.expiration_policy.time_to_live();
        let va = &self.valid_after();
        let mut more_to_evict = true;

        for _ in 0..batch_size {
            let maybe_key_hash_ts = deqs.write_order.peek_front().map(|node| {
                let elem = &node.element;
                (
                    Arc::clone(elem.key()),
                    elem.hash(),
                    elem.is_dirty(),
                    elem.last_modified(),
                )
            });

            let (key, hash, cause) = match maybe_key_hash_ts {
                Some((key, hash, false, Some(ts))) => {
                    let cause = match is_entry_expired_wo_or_invalid(ttl, va, ts, now) {
                        (true, _) => RemovalCause::Expired,
                        (false, true) => RemovalCause::Explicit,
                        (false, false) => {
                            more_to_evict = false;
                            break;
                        }
                    };
                    (key, hash, cause)
                }
                // TODO: Remove the second pattern `Some((_key, false, None))` once
                // we change `last_modified` and `last_accessed` in `EntryInfo` from
                // `Option<Instant>` to `Instant`.
                Some((key, hash, true, _) | (key, hash, false, None)) => {
                    self.skip_updated_entry_wo(&key, hash, deqs);
                    more_to_evict = false;
                    continue;
                }
                None => {
                    more_to_evict = false;
                    break;
                }
            };

            // Lock the key for removal if blocking removal notification is enabled.
            let kl = self.maybe_key_lock(&key);
            let _klg = &kl.as_ref().map(|kl| kl.lock());

            let maybe_entry = self.cache.remove_if(
                hash,
                |k| k == &key,
                |_, v| is_expired_entry_wo(ttl, va, v, now),
            );

            if let Some(entry) = maybe_entry {
                if eviction_state.is_notifier_enabled() {
                    eviction_state.notify_entry_removal(key, &entry, cause);
                }
                eviction_state.counters.incr_eviction_count();
                Self::handle_remove(deqs, timer_wheel, entry, None, &mut eviction_state.counters);
            } else {
                self.skip_updated_entry_wo(&key, hash, deqs);
                more_to_evict = false;
            }
        }

        if more_to_evict {
            eviction_state.more_entries_to_evict = true;
        }
    }

    fn invalidate_entries(
        &self,
        invalidator: &Invalidator<K, V, S>,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        batch_size: u32,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        let now = self.current_time();

        // If the write order queue is empty, we are done and can remove the predicates
        // that have been registered by now.
        if deqs.write_order.len() == 0 {
            invalidator.remove_predicates_registered_before(now);
            return;
        }

        let mut candidates = Vec::new();
        let mut len = 0;
        let has_next;
        {
            let iter = &mut deqs.write_order.peekable();

            while len < batch_size {
                if let Some(kd) = iter.next() {
                    if !kd.is_dirty() {
                        if let Some(ts) = kd.last_modified() {
                            let key = kd.key();
                            let hash = self.hash(&**key);
                            candidates.push(KeyDateLite::new(key, hash, ts));
                            len += 1;
                        }
                    }
                } else {
                    break;
                }
            }

            has_next = iter.peek().is_some();
        }

        if len == 0 {
            return;
        }

        let is_truncated = len == batch_size && has_next;
        let (invalidated, is_done) =
            invalidator.scan_and_invalidate(self, candidates, is_truncated);

        for KvEntry { key: _key, entry } in invalidated {
            Self::handle_remove(deqs, timer_wheel, entry, None, &mut eviction_state.counters);
        }
        if is_done {
            deqs.write_order.reset_cursor();
        }
        if !invalidator.is_empty() {
            eviction_state.more_entries_to_evict = true;
        }
    }

    fn evict_lru_entries(
        &self,
        deqs: &mut Deques<K>,
        timer_wheel: &mut TimerWheel<K>,
        batch_size: u32,
        weights_to_evict: u64,
        eviction_state: &mut EvictionState<'_, K, V>,
    ) where
        V: Clone,
    {
        const CACHE_REGION: CacheRegion = CacheRegion::MainProbation;
        let deq_name = CACHE_REGION.name();
        let (ao_deq, wo_deq) = deqs.select_mut(CACHE_REGION);
        let mut evicted = 0u64;
        let mut more_to_evict = true;

        for _ in 0..batch_size {
            if evicted >= weights_to_evict {
                more_to_evict = false;
                break;
            }

            let maybe_key_hash_ts = ao_deq.peek_front().map(|node| {
                let entry_info = node.element.entry_info();
                (
                    Arc::clone(node.element.key()),
                    node.element.hash(),
                    entry_info.is_dirty(),
                    entry_info.last_accessed(),
                )
            });

            let (key, hash, ts) = match maybe_key_hash_ts {
                Some((key, hash, false, Some(ts))) => (key, hash, ts),
                // TODO: Remove the second pattern `Some((_key, false, None))` once we change
                // `last_modified` and `last_accessed` in `EntryInfo` from `Option<Instant>` to
                // `Instant`.
                Some((key, hash, true, _) | (key, hash, false, None)) => {
                    // `is_dirty` is true or `last_modified` is None. Skip this entry
                    // as it may have been updated by this or other async task but
                    // its `WriteOp` is not processed yet.
                    self.skip_updated_entry_ao(&key, hash, deq_name, ao_deq, wo_deq);
                    // Set `more_to_evict` to `false` to make `run_pending_tasks` to
                    // return early. This will help that `schedule_write_op` to send
                    // the `WriteOp` to the write op channel.
                    more_to_evict = false;
                    continue;
                }
                None => {
                    more_to_evict = false;
                    break;
                }
            };

            // Lock the key for removal if blocking removal notification is enabled.
            let kl = self.maybe_key_lock(&key);
            let _klg = &kl.as_ref().map(|kl| kl.lock());

            let maybe_entry = self.cache.remove_if(
                hash,
                |k| k == &key,
                |_, v| {
                    if let Some(la) = v.last_accessed() {
                        la == ts
                    } else {
                        false
                    }
                },
            );

            if let Some(entry) = maybe_entry {
                if eviction_state.is_notifier_enabled() {
                    eviction_state.notify_entry_removal(key, &entry, RemovalCause::Size);
                }
                eviction_state.counters.incr_eviction_count();
                let weight = entry.policy_weight();
                Self::handle_remove_with_deques(
                    deq_name,
                    ao_deq,
                    wo_deq,
                    timer_wheel,
                    entry,
                    &mut eviction_state.counters,
                );
                evicted = evicted.saturating_add(weight as u64);
            } else {
                self.skip_updated_entry_ao(&key, hash, deq_name, ao_deq, wo_deq);
                more_to_evict = false;
            }
        }

        if more_to_evict {
            eviction_state.more_entries_to_evict = true;
        }
    }
}

impl<K, V, S> Inner<K, V, S>
where
    K: Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub(crate) fn notify_single_removal(
        &self,
        key: Arc<K>,
        entry: &MiniArc<ValueEntry<K, V>>,
        cause: RemovalCause,
    ) {
        if let Some(notifier) = &self.removal_notifier {
            notifier.notify(key, entry.value.clone(), cause);
        }
    }

    #[inline]
    fn notify_upsert(
        &self,
        key: Arc<K>,
        entry: &MiniArc<ValueEntry<K, V>>,
        last_accessed: Option<Instant>,
        last_modified: Option<Instant>,
    ) {
        let now = self.current_time();
        let exp = &self.expiration_policy;

        let mut cause = RemovalCause::Replaced;

        if let Some(last_accessed) = last_accessed {
            if is_expired_by_tti(&exp.time_to_idle(), last_accessed, now) {
                cause = RemovalCause::Expired;
            }
        }

        if let Some(last_modified) = last_modified {
            if is_expired_by_ttl(&exp.time_to_live(), last_modified, now) {
                cause = RemovalCause::Expired;
            } else if is_invalid_entry(&self.valid_after(), last_modified) {
                cause = RemovalCause::Explicit;
            }
        }

        self.notify_single_removal(key, entry, cause);
    }

    #[inline]
    fn notify_invalidate(&self, key: &Arc<K>, entry: &MiniArc<ValueEntry<K, V>>) {
        let now = self.current_time();
        let exp = &self.expiration_policy;

        let mut cause = RemovalCause::Explicit;

        if let Some(last_accessed) = entry.last_accessed() {
            if is_expired_by_tti(&exp.time_to_idle(), last_accessed, now) {
                cause = RemovalCause::Expired;
            }
        }

        if let Some(last_modified) = entry.last_modified() {
            if is_expired_by_ttl(&exp.time_to_live(), last_modified, now) {
                cause = RemovalCause::Expired;
            }
        }

        self.notify_single_removal(Arc::clone(key), entry, cause);
    }
}

//
// for testing
//
