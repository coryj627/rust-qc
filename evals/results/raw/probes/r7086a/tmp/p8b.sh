J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; timeout 10 "$@" </dev/null 2>&1 | head -${LINES_MAX:-10}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
# editor signal
mk; printf 'r:\n' > justfile; printf '#!/bin/sh\nsleep 3; echo editor-done\n' > /work/tmp/ed; chmod +x /work/tmp/ed; (VISUAL=/work/tmp/ed $J --edit </dev/null & P=$!; sleep 0.5; kill -INT $P; sleep 0.2; if kill -0 $P 2>/dev/null; then echo just-still-alive; else echo just-exited; fi; wait $P; echo "rc=$?"; pgrep -f /work/tmp/ed | wc -l)
VISUAL="sh -c true" run visual-args $J --edit
# chroot root justfile
mkdir -p /work/tmp/rootfs/lib /work/tmp/rootfs/lib64 /work/tmp/rootfs/usr/lib /work/tmp/rootfs/bin; cp /target/debug/just /work/tmp/rootfs/bin/just; for l in $(ldd /target/debug/just | grep -o '/[^ ]*'); do mkdir -p /work/tmp/rootfs$(dirname $l); cp -L $l /work/tmp/rootfs$l; done; printf 'r:\n    echo hi\n' > /work/tmp/rootfs/justfile
id -u; timeout 10 chroot /work/tmp/rootfs /bin/just --fmt --check 2>&1 | head -5; echo "chroot rc=${PIPESTATUS[0]}"; timeout 10 chroot /work/tmp/rootfs /bin/just --list 2>&1 | head -5; echo rc=$?
timeout 10 unshare -r chroot /work/tmp/rootfs /bin/just --fmt --check 2>&1 | head -5; echo "unshare rc=${PIPESTATUS[0]}"
