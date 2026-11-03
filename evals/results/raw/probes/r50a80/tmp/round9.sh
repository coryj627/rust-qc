cd /work/probe
export TMPDIR=/tmp
cargo build --offline --bin probe 2>&1 | grep -E "^error|^ +--> src|Finished" | head
for i in 0 1 2 3 6; do /target/debug/probe fetchhead_forms $i 2>&1 | grep -E "^form|assertion|unwrap" | cut -c1-160; done
cd /work
timeout 500 bash /work/skill/scripts/check.sh --audit --features "" /work/crate > /work/tmp/out/check.txt 2>&1; echo check rc=$?
grep -nE "^(==|PASS|FAIL|ok|FAILED|error: )|passed|failed|skipp" /work/tmp/out/check.txt | cut -c1-140 | tail -25
