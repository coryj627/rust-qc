printf '##### hist as_view non-Copy\n'; cd /work/tmp/cfail && cargo check --offline 2>&1 | grep -E '^error|Finished|Copy' | head -5
cd /work/tmp/probe
export MIRIFLAGS="-Zmiri-disable-isolation"
printf '\n##### MIRI deque box\n'; timeout 150 cargo +nightly-2026-10-01 miri test --offline --test deque -- --exact second_branch_box_double_free --nocapture 2>&1 | grep -E 'error|len |test result|-->' | head -8
printf '\n##### MIRI vec retain panic (stacked)\n'; timeout 100 cargo +nightly-2026-10-01 miri test --offline --test vecretain 2>&1 | grep -E 'error|test result|-->' | head -6
printf '\n##### MIRI vec retain panic (tree)\n'; MIRIFLAGS="-Zmiri-tree-borrows" timeout 100 cargo +nightly-2026-10-01 miri test --offline --test vecretain 2>&1 | grep -E 'error|test result|-->' | head -6
printf '\n##### MIRI release clear uaf\n'; timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test imap -- --exact clear_panic_uaf_string_key --nocapture 2>&1 | grep -E 'error|contains|test result|-->|freed' | head -8
printf '\n##### MIRI release retain stale, no SB\n'; MIRIFLAGS="-Zmiri-disable-stacked-borrows" timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test imap -- --exact retain_panic_stale_indices --nocapture 2>&1 | grep -E 'error|get |len after|test result|-->' | head -10
printf '\n##### release pos zero\n'; timeout 100 cargo test --offline --release --test imap -- --nocapture --exact pos_zero_at_full_65536 2>&1 | grep -E 'insert|len |left|right|test result' 
printf '\n##### debug pos zero\n'; timeout 100 cargo test --offline --test imap -- --nocapture --exact pos_zero_at_full_65536 2>&1 | grep -E 'insert|len |precondition|test result|Aborted' 
printf '\n##### MIRI hook ABA\n'; cd /work/tmp/hookprobe; timeout 150 cargo +nightly-2026-10-01 miri test --offline --test aba -- --nocapture 2>&1 | grep -E 'error|preempt|test result|-->|freed' | head -8
printf '\n##### MIRI pool\n'; cd /work/tmp/probe2; timeout 200 cargo +nightly-2026-10-01 miri test --offline --test pool 2>&1 | grep -E '^error|Data race|test result|-->' | head -10
printf '\n##### mpmc stress\n'; cd /work/tmp/probe; timeout 120 cargo test --offline --release --test mpmc_stress -- --nocapture 2>&1 | grep -E 'dequeued|test result|double|left|error' ; echo "stress exit=$?"
