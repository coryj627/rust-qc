#!/bin/sh
cd /work/tmp/probe
cargo test --offline --no-run --tests 2>&1 | tail -15
for t in deque sll vecprobe misc imap; do
  bin=$(cargo test --offline --no-run --test $t --message-format=json 2>/dev/null | python3 -c "import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except: continue
    if m.get('executable') and m.get('profile',{}).get('test'): print(m['executable'])")
  for name in $($bin --list 2>/dev/null | sed -n 's/: test$//p'); do
    printf '\n##### %s::%s\n' "$t" "$name"
    timeout 60 $bin --exact "$name" --nocapture --test-threads=1 2>&1 | grep -v '^$' | grep -v 'RUST_BACKTRACE' | tail -14
    echo "exit=$?"
  done
done
