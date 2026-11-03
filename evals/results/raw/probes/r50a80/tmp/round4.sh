cd /work/probe && cargo build --offline 2>&1 | grep -E "^error|^ +--> src|Finished" | head -30
export TMPDIR=/tmp
B=/target/debug/probe
sh run_all.sh $B skip_reuc cred_inject cred_multi cred_empty_nosh http_username_newline push_negotiation_empty transport_io_nul diff_mode_panic
for w in stats patch print blob merge_empty header; do echo "=== empty_bufs $w"; $B empty_bufs $w 2>&1 | grep -v "^note\|^ *[0-9]*:\|^ *at \|^stack" | cut -c1-200 | head -6; done
cargo build --offline --release 2>&1 | grep -E "^error|Finished"
for n in buf_empty remote_list_empty diff_binary_cb blame_buffer; do echo "=== release $n"; /target/release/probe $n 2>&1 | cut -c1-200 | head -6; echo "exit=$?"; done
