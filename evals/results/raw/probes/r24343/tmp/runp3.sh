export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe
cargo build --offline --bin p2 --bin fuzz 2>&1 | grep -E '^error|^  *--> src|Finished' | head -20
for t in append api misc; do echo "=== $t"; timeout 120 /target/probe/debug/p2 $t 2>&1 | grep -v 'note: run with\|^$\|^  *[0-9]*: \|^  *at ' | cut -c1-500; echo "exit=${PIPESTATUS[0]}"; done
python3 -c "
b=bytes(x for x in range(0x20,0x100) if x not in (0x2f,0x5c)).decode('cp437')
c=open('/work/tmp/cp437.txt',encoding='utf8').read()
print('cp437 equal', b==c, [ (hex(ord(x)),hex(ord(y))) for x,y in zip(b,c) if x!=y][:8])"
