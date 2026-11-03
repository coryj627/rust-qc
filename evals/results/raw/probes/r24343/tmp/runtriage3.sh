export RUST_BACKTRACE=0; cd /work/tmp
echo "--- 196-byte xz, no ulimit"; timeout 60 /target/probe2/release/triage abort-debug-1006.bin 2>&1 | tail -3 | cut -c1-200; echo "rc=${PIPESTATUS[0]}"
python3 - <<'PY'
b=open('/work/tmp/abort-debug-1006.bin','rb').read()
orig=open('/work/crate/tests/data/xz.zip','rb').read()
print(len(orig), [ (i,hex(orig[i]),hex(b[i])) for i in range(min(len(b),len(orig))) if orig[i]!=b[i]][:12])
print(b[39:42+20].hex())
PY
for f in $(ls abort-*.bin | head -400); do o=$( ( timeout 20 /target/probe2/release/triage $f 2>&1 ) | tail -2 | cut -c1-100 | tr -d '\0'); case "$o" in *"memory allocation"*) echo "$f: $o" | tr '\n' ' '; echo;; esac; done 2>/dev/null | sed -E 's/abort-[a-z]+-[0-9]+.bin: //' | cut -c1-60 | sort | uniq -c | sort -rn | head
