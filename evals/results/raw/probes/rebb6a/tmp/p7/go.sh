cd /work/tmp/p7; J=/work/tmp/bin/just
for f in pan1 pan2 pan3; do echo "== $f"; RUST_BACKTRACE=1 $J -f $f --dump 2>&1 | grep -v '^ *at /\|rustc\|^ *[0-9]*: \(std\|core\|__rust\|rust_\)' | head -24 | cut -c1-140; done
RUST_BACKTRACE=1 $J -f pan2 2>&1 | head -5
cp nl1 nl1.t; $J --fmt -f nl1.t; od -c nl1.t | head; cp nl1.t nl2.t; $J --fmt -f nl2.t; od -c nl2.t | head -4; $J --fmt -f nl2.t; od -c nl2.t | head -4
