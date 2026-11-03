cd /work/tmp/probe
M="cargo +nightly-2026-10-01 miri test --offline --test ub"
F='^test |Undefined Behavior|Data race|panicked|assertion|test result|^[0-9]+ '
echo "== indexmap_retain TB"; MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-disable-isolation" timeout 150 $M -- --exact indexmap_retain_panic_stale --nocapture 2>&1 | grep -E "$F|Some|None" | cut -c1-200 | head -14
echo "== indexmap_clear release"; MIRIFLAGS="-Zmiri-disable-isolation" timeout 150 $M --release -- --exact indexmap_clear_panic_stale --nocapture 2>&1 | grep -E "$F|Some|None" | cut -c1-200 | head -12
echo "== peek seeds"; MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-many-seeds=0..16" timeout 200 $M -- --exact spsc_peek_loop 2>&1 | grep -E "Undefined Behavior|Data race|Trying seed|test result|seed" | sort | uniq -c | cut -c1-200 | head -8
echo "== producer ready"; MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-many-seeds=0..16" timeout 200 $M -- --exact spsc_prod_ready_enqueue_unchecked 2>&1 | grep -E "Undefined Behavior|Data race|test result|seed" | sort | uniq -c | cut -c1-200 | head -8
echo "== ord panic"; MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows -Zmiri-ignore-leaks" timeout 200 $M -- --exact heap_and_list_ord_panic 2>&1 | grep -E "Undefined Behavior|test result|error:" | cut -c1-200 | head
