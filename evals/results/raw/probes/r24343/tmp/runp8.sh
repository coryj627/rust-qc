export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --release --bin p4 --bin p3 2>&1 | grep -E '^error|^  *--> src|Finished' | head -20
echo "--- release p4 tail"; timeout 300 /target/probe2/release/p4 2>&1 | sed -n '/failing reader/,$p' | cut -c1-300
echo "--- release failing writer"; FW=1 timeout 300 /target/probe2/release/p4 2>/dev/null | sed -n '/failing writer/,/panics=/p' | cut -c1-200
echo "--- debug failing writer find n"; cargo build --offline --bin p4 2>&1 | tail -1 >/dev/null; FW=1 timeout 300 /target/probe2/debug/p4 2>&1 | grep -E 'PANIC|panic at|aborting' | head -6 | cut -c1-200
echo "--- mem"; for w in lzma ppmd; do (timeout 120 /target/probe2/release/p3 mem $w 2>&1 | cut -c1-240); echo "rc=$?"; done
echo "--- killed"; cd /work/tmp; for s in $(grep -h 'rc=137' fuzz-*summary.log | awk '{print $2}' | head -6); do [ -f last-$s.bin ] && python3 - last-$s.bin <<'PY'
import sys,struct
b=open(sys.argv[1],'rb').read()
ms=[];p=0
while True:
    p=b.find(b'PK\x01\x02',p)
    if p<0: break
    ms.append((struct.unpack('<H',b[p+10:p+12])[0],struct.unpack('<II',b[p+20:p+28]))); p+=4
print(sys.argv[1],len(b),ms[:4])
PY
done
grep -h 'rc=' fuzz-*summary.log | awk '{print $3}' | sort | uniq -c
